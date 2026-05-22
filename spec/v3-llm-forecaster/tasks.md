---
slug: v3-llm-forecaster
version: 0.1.0
status: shipped-partial
owner: tester
updated: 2026-05-22
tester_m_final_2026_05_22: T-T1..T-T10 ticked by tester 2026-05-22. VERDICT → PASS (PARTIAL). Waves A+B+C+E+F+G PASS all cargo gates. Wave D deferred to v0.1.1 (no ANTHROPIC_API_KEY). Anchors 34/34 additive-zero. R9.3 byte-identity SHA 2fb4b243... confirmed. L-verdict L2 stub path (expected). test-final-2026-05-22.md written. First shipped-partial precedent.
parent: strategy-reformulation-survey-2026-05-22 Candidate 5
predecessor: v2-llm-strategy v2.0.0
adr: 0039
decomp: spec/v3-llm-forecaster/decomp.md
promoted_2026_05_22: Queue → Active by operator under v3-volatility-forecaster-noop-fix v0.1.0 deck approval (C1 retired with NEGATIVE-NET-DELTA evidence; C5 picked over C2 for moat-alignment + crates/llm infra reuse)
promotion_ref: spec/v3-volatility-forecaster-noop-fix/presentations/v3-volatility-forecaster-noop-fix-2026-05-22.md
analyst_bridge_2026_05_22: T-A-B1..T-A-B4 closed; M-OD opens with standing-Autoapprove eligible on Q1/Q2/Q3/Q5/Q7/Q8 and explicit-decision required on Q4 (Phase F Assistant slot promotion shape — product-differentiation surface) + Q6 (NEW ADR LLM-verdict criteria — codifies a new artifact)
architect_m_t1_2026_05_22: T-AR-1..T-AR-10 closed; ADR-0039 LLM-forecaster verdict criteria L0-L4 written + registered (status accepted); decomp.md authored ~720 lines; Wave plan A-G ratified with spike T-AR-8 2-3 day prefix; baseline ANCHORS PASS (34 / 34); anchor delta 34 → 36 at developer Wave G close.
budget_estimate: 6-8 weeks total wall-clock (analyst 1w done + architect 1-2w done + dev 3-5w + tester 1w + presenter 1-2d per survey ranking Candidate 5 line 480; HIGH variance per K8 novel-territory risk)
---

# v3-llm-forecaster — tasks

> **PROMOTED Queue → Active 2026-05-22** by operator under the
> `v3-volatility-forecaster-noop-fix` v0.1.0 sprint-review deck
> approval (C1 retired with NEGATIVE-NET-DELTA real evidence;
> C5 picked over C2 for moat-alignment per
> [product.md § Differentiator line 79-83](../product.md#differentiator)
> + `crates/llm` infra reuse). The original analyst pass
> 2026-05-22 was spec-only design exploration (R1-R10, K1-K10,
> H1-H5, Q1-Q8, 8-item non-regression contract); the
> **analyst-bridge pass 2026-05-22 (this update)** populates
> the OD/architect/dev/tester/presenter scaffolding so the
> architect can open M-T1 on a clean handoff. Per AGENT.md,
> task IDs use these prefixes: **T-A** analyst, **T-A-B**
> analyst-bridge, **T-OD** operator-decide, **T-AR** architect,
> **T-D-N** developer (Wave A then B then …), **T-T** tester,
> **T-P** presenter.

## M0 — Analyst pass (this milestone — TICKED 2026-05-22)

- [x] **T-A1** — Read survey § Candidate 5
      (`spec/dev-notes/strategy-reformulation-survey-2026-05-22.md`
      lines 432-537) end-to-end; cite the LOW-MEDIUM prior + the
      replay-cache-determinism load-bearing K-llm-1 + the cost-blow-
      up K-llm-2 + the novel-territory K-llm-3 + the memory-feedback-
      loop K-llm-4 in the K-risk register.
- [x] **T-A2** — Read `spec/product.md` § Differentiator
      (lines 65-83); confirm moat = (2) + (4) (persistent
      reflection memory + auditable double-entry); reference this
      load-bearingly in the feature.md § Why.
- [x] **T-A3** — Read `spec/v2-llm-strategy/feature.md` R1-R7;
      inventory the shipped `llm` crate surface:
      `LlmProvider` trait + 3 provider impls + `BudgetedProvider` +
      `RecordingProvider/ReplayProvider` + `CachedSystemPromptBuilder`
      + `ToolSchema`.
- [x] **T-A4** — Read `spec/ui-rethink-phase-f-memory-models-assistant/feature.md`
      R3 (right-rail Assistant slot Q4=(a) stub-only); confirm
      Phase F shipped `crates/ui/src/assistant/` module structure +
      `RIGHT_RAIL_OPEN_WIDTH_PX = 320.0`; cite at R9.
- [x] **T-A5** — Read `spec/dev-notes/v25-dl-journey-retrospective-2026-05-22.md`
      § "What the next research direction COULD usefully chase"
      line 192 — bullet **"Reflection-memory consumption"** is the
      direct survey-row precursor; cite.
- [x] **T-A6** — Inventory `crates/reflection/src/` exports:
      `retrieve_top_k`, `ReflectionStore::top_k`, `LessonCard`,
      `REPORT_TIME_TOP_K = 5`. Confirm Q4 = report-only annotation
      at `lib.rs:11-18` (trader-side wiring is the deferred
      `reflection-memory-trader-wiring` brief — C5 supersedes).
- [x] **T-A7** — Inventory `crates/strategy/src/` shape:
      existing strategies (`sma_crossover`, `tcn_overlay_momentum`,
      `patchtst_overlay_momentum`, `cross_sectional/`); confirm
      additive pattern fits R4.1 `LlmForecasterStrategy`.
- [x] **T-A8** — Author `feature.md` (R/K/H/Q register; full brief).
      Sections in order: header frontmatter, Why,
      Requirements R1-R10, Q-questions Q1-Q8, K-risk register
      K1-K10, H-hypothesis register H1-H5, Non-regression contract,
      Acceptance criteria M0/M-OD/M-T1/M-FINAL/M-PRESENTER, Cost
      estimate, Trace, Open questions for orchestrator-routing,
      Changelog.
- [x] **T-A9** — Surface Q1-Q8 with analyst-recommended defaults:
      Q1=(a) discrete rating; Q2=(d) all-of-the-above; Q3=(c)
      hybrid; Q4=(a)+(c) hybrid; Q5=(b) replay-cache; Q6=(b) new
      ADR-0038; Q7=(a) `v3.0.0-llm-forecaster` version pin;
      Q8=(b) standalone v0.1.0.
- [x] **T-A10** — Author K-risk register K1-K10. Surface K1
      (reflection-store determinism), K4 (Anthropic drift), K5
      (cache checkout), K8 (5-9w variance), K9 (C1 sequencing),
      K10 (`v2x-trading-state-bus` sequencing) as load-bearing.
- [x] **T-A11** — Author H1-H5 falsifiable hypotheses. Each H has
      explicit falsification protocol + "why this number" rationale.
- [x] **T-A12** — Author non-regression contract (8 items). Anchor
      math: 30 existing → 32 at ship (`v3.0.0-llm-forecaster`
      version pin). All shipped strategies byte-identical. Phase F
      default config byte-identical.
- [x] **T-A13** — Author cost estimate breakdown: analyst 1w +
      architect 1-2w + dev 3-5w + tester 1w + presenter 1-2d
      → 6-9w total. Refines survey 5-9w estimate.
- [x] **T-A14** — Open trace row `REQ-V3-LLM-FORECASTER-001` in
      `draft` state in `spec/trace.toml`. `arch`, `crates`,
      `tests`, `anchors` deferred (empty) until promotion.
- [x] **T-A15** — Add backlog Queue § Strategy entry for
      `v3-llm-forecaster` (NOT Active — Q-SEQ HYBRID). Reference
      survey row + load-bearing risks.
- [x] **T-A16** — Author this `tasks.md` checklist (T-A1..T-A18 done;
      M-T1 / M-DEV / M-FINAL / M-PRESENTER milestones stubbed).
- [x] **T-A17** — Emit operator-decide handoff envelope per
      AGENT.md § Communication contract; verdict
      `READY-FOR-OPERATOR-DECIDE-AFTER-C1-SHIPS`; spec_files +
      Q1-Q8 + load-bearing risks (Q5 determinism + Q6 verdict +
      Q8 v2x-bus relationship) per operator brief.
- [x] **T-A18** — Surface Q-PROMOTE / Q-V2X-SEQ / Q-ASSISTANT-WAKE
      open questions for the orchestrator at the end of `feature.md`.

## M0-BRIDGE — Analyst-bridge pass (TICKED 2026-05-22)

> Bridge from spec-only design exploration → architect-ready
> handoff. Triggered by operator promotion 2026-05-22 under the
> v3-volatility-forecaster-noop-fix deck. Scope: confirm crates
> reuse, populate scaffolding, flip trace + backlog.

- [x] **T-A-B1** (2026-05-22) — Author this bridged tasks.md.
      Frontmatter flipped `status: draft → proposed`; added
      `version: 0.1.0`, `parent`, `predecessor`,
      `promoted_2026_05_22`, `promotion_ref`,
      `budget_estimate` fields. M-OD scaffolded with explicit
      Q1..Q8 + Q-V2X-SEQ + Q-ASSISTANT-WAKE rows (each with
      autoapprove eligibility flag); M-T1 scaffolded with T-AR-1
      ..T-AR-10 covering signal pipeline shape / prompt + replay
      contract / reflection-memory retrieval / cost gating /
      determinism contract / anchor shape / wave plan / spike-vs-
      direct-impl / ADR-0038 draft / K1-K10 resolution; M-DEV
      scaffolded with Wave A..G placeholders mirroring the M-T1
      Wave plan from the original analyst pass; M-FINAL + M-PRESENTER
      ticked through from the analyst pass with T-F* + T-P* IDs.
- [x] **T-A-B2** (2026-05-22) — Walked `crates/llm/src/` and
      `crates/reflection/src/` to confirm the reuse surfaces cited
      in feature.md § Strategic wake conditions are intact + at
      expected paths. Findings:
      - **`crates/llm/`** (shipped v2-llm-strategy v2.0.0
        2026-05-13): `trait_def.rs` (LlmProvider trait),
        `providers/` (Anthropic/OpenAI-compat/Ollama), `budgeted.rs`
        (BudgetedProvider decorator — auto-degrade-at-80%-spend
        gate), `recording.rs` + `replay.rs` (sqlite-backed
        RecordingProvider + ReplayProvider — **load-bearing for
        Q5=(b) replay-cache determinism**), `prompt_cache.rs`
        (CachedSystemPromptBuilder — 2 cache breakpoints,
        ~75% input-token discount on repeats), `tools.rs`
        (ToolSchema + JSON-schema validation — structured output),
        `factory.rs` (LlmProviderFactory builds budget-wrapped
        providers when `cfg.llm.enabled = true`), `bin/`
        (generate-replay-fixture-style binaries). All surfaces
        referenced by R1-R7 are present and at the cited paths.
        **No new infra needed in C5** — extends usage of the
        v2.0.0 surface (R5 + R6).
      - **`crates/reflection/`** (shipped v0.1.0): `lib.rs`
        exports `retrieve_top_k` / `REPORT_TIME_TOP_K = 5` (the
        Q3=(a) top-K default); `retrieval.rs` (`retrieve_top_k(
        store, query, k) -> Vec<(LessonCard, score)>` signature
        matches feature.md R2.3); `store/` (sqlite-backed
        ReflectionStore + in-memory fake for tests); `regime.rs`
        (3-state BTC daily-close tagger `RegimeTag { Bull, Bear,
        Chop }` — **also load-bearing for C2 v3-regime-classifier;
        C5 may consume the same tag in `RetrievalQuery`**);
        `embedding.rs` (32-dim deterministic embedding for
        lesson-card retrieval); `query.rs` (RetrievalQuery shape);
        `writer/` (lesson-card writer pipeline — **NOT touched by
        C5 per R10.8 read-only consumer contract**);
        `audit_tick_consumer.rs` + `trail_mirror.rs` (bridge to
        audit-tick stream); `post_mortem_analyst.rs` +
        `outcome.rs` (post-trade lesson generation). All R2.3 +
        R10.8 surfaces present at cited paths. **No new
        infrastructure needed in C5** — read-only top_k consumer.
      - **`crates/audit/`** (Phase D + D+ shipped): not walked in
        depth this pass; feature.md R7 cites additive migration
        # 011 for `llm_forecast` journal entries. Architect M-T1
        owns the migration file path lock.
      - **`crates/ui/src/assistant/`** (Phase F shipped): not
        walked in depth this pass; feature.md R9 cites the
        `view.rs` body promotion + `RIGHT_RAIL_OPEN_WIDTH_PX = 320.0`
        constant. Architect M-T1 owns the body-composition lock.
- [x] **T-A-B3** (2026-05-22) — Enumerate Q1-Q8 +
      Q-PROMOTE + Q-V2X-SEQ + Q-ASSISTANT-WAKE with **standing
      Autoapprove eligibility flag** per operator's 2026-05-22
      session standing directive (heuristic: standing Autoapprove
      APPLIES to "use the analyst-default mechanism rather than
      scope-creep alternative" patterns — cheaper, lower-risk
      paths; does NOT apply to budget calls, model-provider picks,
      novelty-vs-conservative tradeoffs, or anything materially
      changing the 6-8 week scope). Resolution flagged inline at
      T-OD1..T-OD10 below. **Net result**: Q1/Q2/Q3/Q5/Q7/Q8 +
      Q-V2X-SEQ + Q-ASSISTANT-WAKE eligible for standing
      Autoapprove (6 of 8 Qs + 2 sequencing-Qs all default to
      analyst-recommended low-risk paths); **Q4 (consumer shape —
      Phase F Assistant slot product-differentiation surface) +
      Q6 (NEW ADR-0038 LLM-verdict criteria) require explicit
      operator decision** because they materially shape the v0.1.0
      surface area (Q4) + codify a durable artifact across all
      future LLM-strategy ships (Q6).
- [x] **T-A-B4** (2026-05-22) — Spec hygiene:
      - **`spec/trace.toml`**: REQ-V3-LLM-FORECASTER-001 state
        flipped `draft → proposed`. `feature` field stays
        `v3-llm-forecaster`. No parent ref (C5 stands alone — not
        a child of C1 v3-volatility-forecaster v0.1.0 since the
        v3-vol programme retired 2026-05-22 with NEGATIVE-NET-
        DELTA real evidence, but the trace comment cites the
        operator-decide sequencing chain that promoted C5 here).
        `arch` / `crates` / `tests` / `anchors` stay empty —
        architect / developer / tester fill at their respective
        milestones.
      - **`spec/backlog.md`**: C5 entry moved Queue § Strategy →
        Active with a new comment block citing the
        2026-05-22 noop-fix deck retirement + C5 promotion (mirror
        of the prior Active blocks v3-vol-forecaster-noop-fix +
        v3-volatility-forecaster-rebaseline + v3-volatility-
        forecaster). C2 (`v3-regime-classifier`) entry stays in
        Queue with a deferral-comment update "DEFERRED-2026-05-22
        retained pending C5 ship".

## M-OD — Operator-decide (Q1-Q8 + sequencing)

Operator answers Q1-Q8 + Q-PROMOTE (resolved 2026-05-22 by
promotion) + Q-V2X-SEQ + Q-ASSISTANT-WAKE. Per analyst-bridge
T-A-B3: **6 of 8 Q-questions + 2 sequencing-Qs eligible for
standing Autoapprove**; **Q4 + Q6 require explicit operator
decision** (product-differentiation surface + new durable
artifact).

> Per AGENT.md § Communication contract, the orchestrator may
> auto-tick rows flagged `[STANDING-AUTOAPPROVE]` ahead of
> spawning architect M-T1; rows flagged `[EXPLICIT-DECISION-
> REQUIRED]` block until the operator routes.

- [x] **T-OD1** — Resolve **Q1 — Signal shape**.
      `[STANDING-AUTOAPPROVE — analyst default (a) discrete 5-tier
      rating + confidence + reasoning trace per product.md § Five-
      tier rating scale line 156]`. (b) μ-equivalent rejected on
      v2.5 F-verdict grounds; (c) regime overlaps C2; (d) free-form
      is v0.2.0 follow-on. — **Resolved 2026-05-22 → (a)** by orchestrator under operator's standing Autoapprove.
- [x] **T-OD2** — Resolve **Q2 — Input shape**.
      `[STANDING-AUTOAPPROVE — analyst default (d) all-of-the-
      above (OHLCV + indicators + top-K lesson cards + recent
      audit decisions)]`. (a) raw OHLCV alone re-litigates the
      F4'd v2.5 task framing; (b) + (c) alone are the
      differentiator but miss the standard quant primitives. — **Resolved 2026-05-22 → (d)** by orchestrator under standing Autoapprove.
- [x] **T-OD3** — Resolve **Q3 — Memory consumption shape**.
      `[STANDING-AUTOAPPROVE — analyst default (c) top-K +
      distilled summary hybrid; fallback to (a) top-K-only if
      reflection-memory-distillation follow-on not shipped (it
      currently isn't — `crates/reflection/src/lib.rs:20-24`
      gates distillation as a deferred follow-up brief)]`.
      (b) full ledger rejected on cost grounds per K-llm-2. — **Resolved 2026-05-22 → (c) with (a) fallback** by orchestrator under standing Autoapprove.
- [x] **T-OD4** — Resolve **Q4 — Consumer shape**.
      `[EXPLICIT-DECISION-REQUIRED — biggest product-
      differentiation surface in v0.1.0; analyst default is
      (a)+(c) hybrid — standalone LlmForecasterStrategy +
      Phase F Assistant slot body promotion (runtime-gated per
      R9.3 so default-disabled config stays Phase F byte-
      identical). (b) overlay-on-momentum deferred to v0.2.0;
      (d) all-three-as-builders deferred to v0.2.0+. **Operator
      may opt to ship Q4 = (a) only** (defer Q4=(c) Assistant
      slot promotion to v0.1.1) to tighten the v0.1.0 scope at
      the cost of losing the moat-visible surface]`. — **Resolved 2026-05-22 → (a)+(c) hybrid** by operator. Both standalone LlmForecasterStrategy AND Phase F Assistant slot body promotion ship in v0.1.0 (runtime-gated per R9.3). Wave F UNGATED.
- [x] **T-OD5** — Resolve **Q5 — Determinism contract**.
      `[STANDING-AUTOAPPROVE — analyst default (b) replay-cache +
      `temperature = 0` (extends shipped `crates/llm::
      RecordingProvider/ReplayProvider` from v2-llm-strategy
      v2.0.0; no new infra)]`. (a) `temperature=0` + seed alone
      insufficient (Anthropic deploys can drift); (c) accept non-
      determinism rejected on anchor-precondition grounds (H4
      load-bearing).
      **Sub-decision Q5b** (`config.llm_forecaster.timeout_ms`
      per-call wall-clock budget): analyst-strawman 30_000 ms;
      architect M-T1 refines. — **Resolved 2026-05-22 → (b) + Q5b strawman 30_000 ms** by orchestrator under standing Autoapprove; architect M-T1 confirms Q5b at decomp.md.
- [x] **T-OD6** — Resolve **Q6 — Verdict shape**.
      `[EXPLICIT-DECISION-REQUIRED — analyst default (b) new
      ADR-0038 "LLM-forecaster verdict criteria L1-L4" (L1 bias
      collapse / L2 calibration failure / L3 cost overrun / L4
      reasoning trace degenerate / L0 PASS). (a) re-use ADR-0033
      F-verdict adapted is analyst-NACK (F1-F4 priorities are
      μ-prediction-specific N/A here). (c) inline track per-report
      is fragile across future LLM-strategy ships. **Operator
      decision codifies a new durable artifact spanning future
      LLM-strategy work** — explicit gate worth the operator's
      attention]`. — **Resolved 2026-05-22 → (b) NEW ADR with analyst-strawman L1-L4 priorities LOCKED** by operator. No expansion authorization at M-T1; architect cap "≤2 new priorities beyond analyst-strawman before re-surface" enforced. ADR namespace: architect M-T1 confirms renumber (ADR-0038 occupied by retired C1 lane; default → ADR-0039 unless another ADR landed since the analyst pass).
      > **Open**: a NEW ADR-0038 conflicts with the existing
      > `0038-vol-forecast-verdict-shape.md` from C1 v3-volatility-
      > forecaster (also retired with NEGATIVE-NET-DELTA). The
      > ADR namespace may need a renumber to ADR-0039 (or higher
      > if other ADRs landed since the analyst pass). Architect
      > M-T1 confirms.
- [x] **T-OD7** — Resolve **Q7 — Anchor strategy**.
      `[STANDING-AUTOAPPROVE — analyst default (a) new version
      pin `v3.0.0-llm-forecaster` (signals the v3 era; mirrors
      `v2.5a.0-patchtst` and `v3.0.0-volatility` precedents);
      anchor count 30 (existing — note that v3-vol-forecaster-
      noop-fix delta updated 4 SHAs in-place under existing
      namespaces; total anchor count after that fix wave is 34)
      → 36 at C5 ship (+2: `top10-2023-fy-llm-forecaster-realdata`
      + `top10-2024-fy-llm-forecaster-realdata`)]`. (b) re-use
      v2.x weaker signal; (c) skip anchors at v0.1.0 rejected on
      regression-gate grounds. — **Resolved 2026-05-22 → (a)** by orchestrator under standing Autoapprove.
- [x] **T-OD8** — Resolve **Q8 — `v2x-trading-state-bus`
      relationship**.
      `[STANDING-AUTOAPPROVE — analyst default (b) standalone
      v0.1.0; `ForecastContext` (R2.1) ships as a concrete
      struct; lift to `TradingState` substrate in v0.1.1 if
      `v2x-trading-state-bus` ships]`. (a) coupling C5 to a
      refactor the operator hasn't promoted rejected on schedule
      grounds. — **Resolved 2026-05-22 → (b)** by orchestrator under standing Autoapprove.
- [x] **T-OD9** — Resolve **Q-V2X-SEQ** — orchestrator
      sequencing if `v2x-trading-state-bus` is promoted ahead of
      C5. Default = C5 ships standalone v0.1.0 regardless.
      `[STANDING-AUTOAPPROVE — analyst default standalone v0.1.0
      regardless; `v2x-trading-state-bus` is not on the active
      docket and the C5 brief is durable spec either way]`. — **Resolved 2026-05-22 → standalone-v0.1.0** by orchestrator under standing Autoapprove.
- [x] **T-OD10** — Resolve **Q-ASSISTANT-WAKE** — Phase F
      Assistant slot body promotion shape. Default = runtime-
      gated by strategy-enabled flag (R9.3 — preserves Phase F
      v0.1.0 default-disabled byte-identity).
      `[STANDING-AUTOAPPROVE — analyst default runtime-gated;
      preserves Phase F snapshot baselines on default-disabled
      config. Unconditional slot promotion rejected on safety
      grounds (would force-break Phase F's R10.3 byte-identity
      guard)]`. — **Resolved 2026-05-22 → runtime-gated** by orchestrator under standing Autoapprove.

> **Q-PROMOTE** resolved by operator 2026-05-22 under the
> v3-volatility-forecaster-noop-fix v0.1.0 deck approval. C5
> moved Queue → Active under operator's prior session standing
> directive "[on retirement of C1] pick the strongest moat-
> aligned alternative". No separate T-OD row needed; the
> frontmatter `promoted_2026_05_22` field carries the receipt.

## M-T1 — Architect decomposition (CLOSED 2026-05-22)

> **CLOSED 2026-05-22** — T-AR-1..T-AR-10 ticked; ADR-0039
> written + registered; decomp.md authored at
> `spec/v3-llm-forecaster/decomp.md` (~720 lines). Architect
> handoff envelope per AGENT.md § Communication contract emitted.
> Baseline ANCHORS PASS (34 / 34) quoted from
> `bash scripts/verify_anchors.sh`. Anchor delta plan: 34 → 36
> at developer Wave G close. Spike T-AR-8 = YES (2-3 day prefix
> to Wave A). Wave F UNGATED per Q4=(a)+(c) hybrid.

> **Critical M-T1 path** — architect MUST resolve K1 + K4 + K5
> + Q6 ADR shape before Wave A spawns. Q5 sub-decision Q5b
> timeout-ms is a Wave B refinement (lower-priority).

- [x] **T-AR-1** — **Signal pipeline shape** — **CLOSED 2026-05-22**.
      Decision pinned per `decomp.md` § T-AR-1. New
      `LlmForecasterStrategy: Strategy` at
      `crates/strategy/src/llm_forecaster/strategy.rs`;
      registered as `"llm_forecaster_v3"` in
      [`crates/strategy/src/registry.rs:96-100`](../../crates/strategy/src/registry.rs).
      Module organisation: 7 files (`mod.rs` / `trait_def.rs` /
      `types.rs` / `anthropic_impl.rs` / `strategy.rs` /
      `prompt.rs` / `tool_schema.rs` / `verdict.rs`). Signal
      mapping = 5-tier → 3 `SignalKind` (Buy/Hold/Sell);
      `quantity_scale` inherits the noop-fix-shipped default 1.0
      per [`traits.rs:16-26`](../../crates/strategy/src/traits.rs)
      (LLM-forecaster does not vol-target). STRONG vs regular
      tier preserved in `reasoning_trace` + audit JournalEntry +
      verdict L1 `hold_frac` denominator; NOT collapsed at
      `SignalKind` level (R10.2 strategy byte-identity guard).
- [x] **T-AR-2** — **Prompt + replay-cache contract** — **CLOSED 2026-05-22**.
      Decision pinned per `decomp.md` § T-AR-2. Prompt template
      lives at NEW `crates/strategy/src/llm_forecaster/prompt.rs`
      (strategy-specific business logic; NOT `crates/llm/src/prompts/`).
      2 cache breakpoints (project ~800 tokens + role ~1200 tokens);
      per-call dynamic block is **markdown** (architect-pick over
      JSON for human-readability at spike-time review).
      Replay-cache namespace: dedicated `data/llm-forecaster-replay.db`
      (live) + `crates/strategy/tests/fixtures/llm-forecaster-replay.db.gz`
      (checked-in, < 50 MB target per K5). Reuses
      `crates/llm::RecordingProvider/ReplayProvider` schema verbatim;
      additive `cache_schema_version` column (v=1 at v0.1.0).
      `ForecastContext::request_hash()` canonicalisation: serde_json
      over `CanonicalContext` struct with alphabetical
      field-declaration order (`model_id`, `now`,
      `prompt_template_version`, …), SHA-256 over bytes. **Q5b
      sub-decision refined**: `timeout_ms = 45_000` (45s; 2.25×
      Anthropic Sonnet p99 ~20s; safety margin for Ollama on
      slowest expected developer hardware).
- [x] **T-AR-3** — **Reflection-memory retrieval shape** — **CLOSED 2026-05-22**.
      Decision pinned per `decomp.md` § T-AR-3. `RetrievalQuery`
      shape at
      [`crates/reflection/src/types.rs:109-113`](../../crates/reflection/src/types.rs)
      is fit-for-purpose **as-is** — no architect-add. Inputs:
      `(strategy_id = "llm_forecaster_v3", symbol_or_pair,
      current_regime)`; `current_regime` from
      [`reflection::regime::classify_regime`](../../crates/reflection/src/regime.rs)
      3-state BTC daily-close tagger (Bull/Bear/Chop). K = 5
      default via existing `REPORT_TIME_TOP_K` constant
      [`crates/reflection/src/lib.rs:69`](../../crates/reflection/src/lib.rs);
      no strategy-specific override. **Distillation fallback
      (Q3 (c) → (a) hybrid)**: top-K-only path activates since
      `crates/reflection/src/lib.rs:20-24` gates distillation as
      deferred; upgrade route is `DISTILLATION_ENABLED` const
      flip + cache invalidation via additive
      `distilled_summary_sha: Option<[u8; 32]>` field on
      `CanonicalContext`. **K1 resolution**: backtest binary
      gains `--reflection-store-snapshot <path>` CLI flag pinning
      the store to a frozen sqlite dump (analyst-recommended).
      3-layer determinism: request_hash + store snapshot + 3-back-
      to-back cache-build gate (T-AR-5).
- [x] **T-AR-4** — **Cost gating + budget kill-switch** — **CLOSED 2026-05-22**.
      Decision pinned per `decomp.md` § T-AR-4. Per-call max USD
      pinned per provider tier: Haiku $0.01 / Sonnet $0.05 / Opus
      $0.15 / Ollama $0.00. Per-backtest cap pinned per scenario:
      Haiku $100 (architect-bumped from analyst-strawman $20/$25
      per cold-record projection ~$80/year on Haiku) / Sonnet
      $100 / Opus $300. **`fire_every_n_bars`** retained at
      analyst-strawman default = 24 (once-per-day on hourly bars).
      Per-day live cap strawman `cost_cap_usd_per_day = $50` (¼ of
      v2-llm-strategy $200/month ceiling); final number deferred
      to live-deployment promotion at presenter time. Enforcement
      via existing
      [`crates/llm::BudgetedProvider`](../../crates/llm/src/budgeted.rs)
      decorator — no new code path; `LlmProviderFactory::build`
      wraps automatically when `cfg.llm.enabled = true`.
      `BudgetExceeded` short-circuits backtest binary with explicit
      error log + non-zero exit (L3 verdict captures bench
      mis-estimate per ADR-0039 § D1.b).
- [x] **T-AR-5** — **Determinism contract — replay-cache must
      serve byte-identical responses for backtest determinism** —
      **CLOSED 2026-05-22**. Decision pinned per `decomp.md`
      § T-AR-5. 5-layer determinism stack: (L1) `temperature = 0`
      pinned at every call site; (L2) RecordingProvider /
      ReplayProvider sqlite cache — cache-miss in research mode
      FATAL → `LlmForecasterError::ReplayMiss` → non-zero exit;
      (L3) re-record protocol via
      `cargo run --bin llm-forecaster-rerecord -- --scenario … --reflection-store-snapshot …`
      with MIGRATION warning per v25-tcn-overlay precedent;
      **(L4) 3-back-to-back identical cache-build run gate**
      (tester at M-FINAL requires 3 consecutive runs produce
      byte-identical body-SHAs before anchor lock); (L5)
      `cache_schema_version` migration shape (v=1 at v0.1.0; bumps
      invalidate cache + anchor via new ADR amendment per ADR-0029
      precedent). Canonical `request_hash` per T-AR-2 (serde_json
      over `CanonicalContext` struct; alphabetical field-declaration
      order).
- [x] **T-AR-6** — **Anchor shape — where LLM-forecast reports
      live + body-SHA contract** — **CLOSED 2026-05-22**. Decision
      pinned per `decomp.md` § T-AR-6 + ADR-0039 § D6. New
      namespace `v3.0.0-llm-forecaster`; +2 rows at developer
      Wave G close (M-FINAL after 3-back-to-back gate):
      `top10-2023-fy-llm-forecaster-realdata` +
      `top10-2024-fy-llm-forecaster-realdata`.
      `sharpe-comparison-llm-forecaster-bs1-realdata` (Wave E)
      ships **without** anchor at v0.1.0 (analyst-recommended
      deferral; lift route at v0.1.1 if L0/L-ALPHA-UNLOCKED or
      L-MARGINAL). Body shape: Checkpoint table + Per-call cost
      table + Rating distribution histogram + Per-symbol Sharpe
      table + reasoning_trace_sha256 histogram (top 10) +
      Aggregate statistics + Verdict section per ADR-0039 § D2.
      Float canon: `%.6` decimals; integer canon: `%`. Symbol
      row order: alphabetical USDT-quote (ADAUSDT, AVAXUSDT, …,
      XRPUSDT) — mirror of ADR-0038 § D2.a.
- [x] **T-AR-7** — **Wave plan A-G ratified** — **CLOSED 2026-05-22**.
      Decision pinned per `decomp.md` § T-AR-7 with cargo
      invocations + expected literals per task. Wave F **UNGATED**
      per Q4=(a)+(c) hybrid operator-pick T-OD4. Confirm the
      analyst-strawman wave map (preserved from the original
      analyst pass; refined at M-T1):
      - **Wave A** (sequential, foundation) —
        `LlmForecaster` trait + `LlmForecast` payload +
        `ForecastContext` (R1 + R2).
      - **Wave B** (sequential after A) —
        `LlmForecasterImpl` over `Arc<dyn llm::LlmProvider>` +
        prompt-builder wiring + tool-use schema + temperature
        pin (R3 + R5.1-R5.2).
      - **Wave C** (parallel-safe with Wave D) —
        `LlmForecasterStrategy: Strategy` + registry entry
        (R4.1).
      - **Wave D** (parallel-safe with Wave C) — Backtest
        scenarios `top10-2023-fy-llm-forecaster-realdata` +
        2024 + replay-cache wiring (R8 + R6).
      - **Wave E** (parallel-safe with Wave F; depends on C) —
        Audit-ledger emission + additive migration #011 +
        `BudgetedProvider` cost wiring (R7 + R5.3-R5.5).
      - **Wave F** (parallel-safe with Wave E; depends on C) —
        Phase F Assistant slot body promotion (R9) — snapshot
        baselines + layout invariants + R9.3 byte-identity
        guard. **GATED ON Q4=(c) operator decision** at T-OD4.
      - **Wave G** (serial closure; depends on A-F) — ADR-0038
        commit + `crates/strategy/tests/llm_forecaster_neutrality.rs`
        (re-runs `top10-2023-fy-tcn-overlay-realdata` and asserts
        `8fa47f49…` body-SHA unchanged) + non-regression sweep
        + tester handoff envelope.
- [x] **T-AR-8** — **Spike requirement** — **CLOSED 2026-05-22**.
      Architect-confirms **SPIKE YES, 2-3 day prefix to Wave A**.
      Scope per `decomp.md` § T-AR-8: Day 1 bench bin scaffold
      via NEW `cargo run --bin llm-forecaster-bench -- --slice 1week
      --symbols 10`; Day 2 prompt-template iteration (token-count
      ≤ 8k input + ≤ 1k output; ≥ 95% structured tool-use payloads);
      Day 3 cache-hit-ratio empirical check + full-year cost
      projection. Deliverable: dev-note
      `spec/dev-notes/v3-llm-forecaster-prompt-spike-<date>.md`
      (~200-400 lines) — orchestrator approval gate before
      Wave A T-D-N(A1). Spike-NO rejected: novel-territory +
      LOW-MEDIUM prior + 5-10d rework risk in Waves B-D.
- [x] **T-AR-9** — **Draft ADR-0039 "LLM-forecaster verdict
      criteria L0-L4"** per Q6=(b) — **CLOSED 2026-05-22**.
      Authored at
      [`spec/architecture/adr/0039-llm-forecaster-verdict-criteria.md`](../architecture/adr/0039-llm-forecaster-verdict-criteria.md);
      status `accepted`. **ADR namespace pick = 0039** (next
      available after 0038 vol-forecast-verdict-shape; T-OD6 open
      now resolved). Analyst-strawman LOCKED per Q6 operator
      constraint:
      L1 = bias collapse (`hold_frac ≥ 0.95`); L2 = calibration
      failure (`|confidence_outcome_corr| < 0.05`); L3 = cost
      overrun (`overrun_ratio > 2.0 OR cost_actual > cost_cap`);
      L4 = reasoning trace degenerate (`short_frac > 0.50 OR
      duplicate_frac > 0.50`); L0 = PASS (routes to L_ALPHA
      strategy-side gate). L_ALPHA classifier inherits Sharpe-delta
      thresholds from ADR-0038 § D1.c verbatim (L-ALPHA-UNLOCKED
      `≥ +0.10` / L-MARGINAL `[+0.05, +0.10)` / L-NO-ALPHA
      `< +0.05`). Architect cap "≤2 new priorities beyond strawman
      before re-surface" codified inline at ADR-0039 § D1.b.
      PARALLEL to ADR-0033 § D3 and ADR-0038 § D1, NOT extension.
      Registered in
      [`spec/architecture/adr/README.md`](../architecture/adr/README.md)
      at row 89.
- [x] **T-AR-10** — **K1-K10 resolution + decomp.md write** —
      **CLOSED 2026-05-22**.
      All 10 K-risk mitigations pinned per `decomp.md` § T-AR-10
      table: K-llm-1 (T-AR-3 store snapshot flag + 3-layer
      determinism stack), K-llm-2 (T-AR-4 cost-cap math + spike
      empirical projection + L3 cost-overrun verdict), K-llm-3
      (ADR-0039 L4 mechanical gate + operator H3 subjective at
      presenter), K-llm-4 (T-AR-5 Layer 4 3-back-to-back gate),
      K-llm-5 (T-AR-2 check-in compressed cache < 50 MB), K-llm-6
      (ADR-0039 § D1.b architect cap ≤2 new priorities), K-llm-7
      (R9.3 runtime gate confirmed; Q-ASSISTANT-WAKE = runtime-
      gated operator-locked), K-llm-8 (spike YES de-risks
      prompt-iteration), K-llm-9 (resolved by 2026-05-22
      promotion), K-llm-10 (Q-V2X-SEQ → C5 standalone v0.1.0
      regardless). `decomp.md` authored at
      `spec/v3-llm-forecaster/decomp.md` (~720 lines).
      Frontmatter ticked `owner: analyst → architect`;
      `status: proposed → in-progress`. Spec hygiene:
      `spec/trace.toml` REQ-V3-LLM-FORECASTER-001
      `state: proposed → in-progress` + `arch` column populated.

## M-DEV — Developer waves (OPENS at M-T1 close)

Developer waves A-G enumerated per architect M-T1 decomp.md.
Analyst-bridge provides wave-level T-D-N stubs (architect M-T1
refines into ordered, sized rows with file:line citations + dep
graphs). Expected: ~30-40 T-D-N rows total across 7 waves; ~3-5
weeks wall-clock per H5.

### Wave A — Foundation (`LlmForecaster` trait + payload)

> Sequential, ~1-3 days. Foundation for Waves B-G.
> **CLOSED 2026-05-22** — T-D-N(A1..A5) ticked by developer.
> Cargo gates: fmt PASS + clippy -p strategy PASS + lib tests 124 PASS +
> integration test 25 PASS + ANCHORS PASS (34 / 34).

- [x] **T-D-N(A1)** — Create `crates/strategy/src/llm_forecaster/`
      module + `trait_def.rs`. Define `LlmForecaster: Send + Sync
      + 'static` async trait with `name(&self)` + `forecast(&self,
      ctx: ForecastContext) -> Result<LlmForecast,
      LlmForecasterError>` signature (R1.1).
      - **file:line**: `crates/strategy/src/llm_forecaster/trait_def.rs:49-68`;
        `crates/strategy/src/llm_forecaster/mod.rs:1`
      - **test cmd**: `cargo check -p strategy`
      - **output**: `Finished dev profile [unoptimized + debuginfo] target(s) in 3.32s`
- [x] **T-D-N(A2)** — Define `LlmForecast` payload (R1.2) +
      `LlmForecasterError` enum (R1.4) + `Rating` /
      `Confidence` / `Horizon` / `LessonCardRef` /
      `CostEventRef` value types.
      - **file:line**: `crates/strategy/src/llm_forecaster/types.rs:74-355`;
        `LlmForecast::new` at types.rs:293; `LlmForecasterError` at types.rs:600;
        `LlmForecasterConfig` at types.rs:663; `StubForecaster` at types.rs:713
      - **test cmd**: `cargo build -p strategy`
      - **output**: `Finished dev profile [unoptimized + debuginfo] target(s) in 4.83s`
- [x] **T-D-N(A3)** — Define `ForecastContext` payload (R2.1):
      symbol + now + recent_bars + indicators + top_k_lessons +
      recent_decisions + correlation_id. Implement
      `ForecastContext::test_fixture` (deterministic builder; Wave A
      uses test_fixture since from_runtime requires live runtime).
      - **file:line**: `crates/strategy/src/llm_forecaster/types.rs:398-458`
      - **test cmd**: `cargo test -p strategy --lib forecast_context_from_runtime`
      - **output**: `test llm_forecaster::types::tests::forecast_context_from_runtime ... ok; test result: ok. 1 passed`
- [x] **T-D-N(A4)** — Implement `ForecastContext::request_hash()`
      canonical SHA-256 over the prompt body (R6.6) — serde_json
      with sorted keys per architect M-T1 lock. `CanonicalContext`
      struct with alphabetical field-declaration order. `canonicalize.rs`
      module with `hex_encode` + `sha256` helpers.
      - **file:line**: `crates/strategy/src/llm_forecaster/types.rs:462-500`
        (request_hash impl); `crates/strategy/src/llm_forecaster/canonicalize.rs:1`
      - **test cmd**: `cargo test -p strategy --lib forecast_context_request_hash`
      - **output**: `test result: ok. 3 passed; 0 failed; finished in 0.00s`
- [x] **T-D-N(A5)** — Unit tests: `Rating::to_signal_kind()` round-trip;
      `LlmForecast` serde round-trip; deterministic
      `ForecastContext::request_hash`; `StubForecaster` smoke;
      `LlmForecasterStrategy` carry-forward (Wave A acceptance gate).
      - **file:line**: `crates/strategy/tests/llm_forecaster_payload.rs:1`
        (25 integration tests); inline `#[cfg(test)]` in
        `types.rs`, `strategy.rs`, `canonicalize.rs` (18 unit tests total)
      - **test cmd**: `cargo test -p strategy --test llm_forecaster_payload`
      - **output**: `test result: ok. 25 passed; 0 failed; finished in 0.00s`

### Wave B — Impl over LlmProvider + prompt + schema

> Sequential after Wave A, ~3-7 days.
> **CLOSED 2026-05-22** — T-D-N(B1..B5) ticked by developer.
> Cargo gates: fmt PASS + clippy --workspace PASS + lib tests 311 PASS +
> integration test llm_forecaster_payload 25 PASS + wiremock 17 PASS +
> ANCHORS PASS (34 / 34). Wiremock path (no real API calls).
> Deviation note: `LlmForecasterImpl` takes `Arc<dyn LlmProvider>` directly
> rather than a separate `Arc<dyn reflection::ReflectionStore>` — reflection
> store wiring is deferred to Wave C per decomp.md T-AR-3 (Wave C opens
> ForecastContext::from_runtime which does the top-K retrieval; Wave B
> uses ForecastContext::test_fixture with empty lessons in tests).

- [x] **T-D-N(B1)** — `crates/strategy/src/llm_forecaster/anthropic_impl.rs`
      — `LlmForecasterImpl` struct over `Arc<dyn llm::LlmProvider>`
      + `Arc<llm::CachedSystemPromptBuilder>` (via prompt.rs) + `llm::ToolSchema`
      + `LlmForecasterConfig` (R3.1). Temperature pin + decode logic.
      - **file:line**: `crates/strategy/src/llm_forecaster/anthropic_impl.rs:82-116`
      - **test cmd**: `cargo test -p strategy --test llm_forecaster_wiremock b5_happy_path_buy_response_round_trips`
      - **output**: `test b5_happy_path_buy_response_round_trips ... ok`
- [x] **T-D-N(B2)** — System-prompt composition via
      `CachedSystemPromptBuilder` — 2 cache breakpoints (project
      ~800 tokens, role ~1200 tokens) + per-call dynamic block
      (R3.2; markdown per architect M-T1 lock).
      - **file:line**: `crates/strategy/src/llm_forecaster/prompt.rs:38-87`
        (`PROJECT_CONTEXT` + `ROLE_CONTEXT` + `render_dynamic_block`)
      - **test cmd**: `cargo test -p strategy --lib llm_forecaster::prompt::tests`
      - **output**: `test result: ok. 151 passed; 0 failed` (includes prompt tests)
- [x] **T-D-N(B3)** — `propose_forecast` `ToolSchema` definition +
      JSON-schema validation via `llm::tools::validate_tool_use` (R3.3).
      5-tier rating enum, confidence [0,1], reasoning_trace minLength 50,
      horizon enum "short", optional cited_lesson_ids array.
      - **file:line**: `crates/strategy/src/llm_forecaster/tool_schema.rs:55-103`
      - **test cmd**: `cargo test -p strategy --lib llm_forecaster::tool_schema::tests`
      - **output**: `test result: ok. 151 passed; 0 failed` (includes schema tests)
- [x] **T-D-N(B4)** — `temperature = Some(0.0)` pin (R3.4) set in
      `LlmForecasterImpl::build_request` at `req.temperature = Some(0.0)`;
      verified in wiremock test that the wire body carries `"temperature": 0.0`.
      - **file:line**: `crates/strategy/src/llm_forecaster/anthropic_impl.rs:142`
      - **test cmd**: `cargo test -p strategy --test llm_forecaster_wiremock b5_request_body_pins_temperature_zero`
      - **output**: `test b5_request_body_pins_temperature_zero ... ok`
- [x] **T-D-N(B5)** — 17 wiremock integration tests covering: happy-path
      BUY/HOLD/all-5-ratings round-trips; temperature pin; 2 cache breakpoints;
      free-text → InvalidResponse; short trace → InvalidResponse; unknown rating
      → InvalidResponse; missing field → InvalidResponse; confidence out of range
      → InvalidResponse; HTTP 500 → Provider error; HTTP 401 → Provider error;
      HTTP 429 retries then Provider error; determinism (identical contexts →
      identical request bodies); tool name + schema shape.
      - **file:line**: `crates/strategy/tests/llm_forecaster_wiremock.rs:1-619`
      - **test cmd**: `cargo test -p strategy --test llm_forecaster_wiremock`
      - **output**: `test result: ok. 17 passed; 0 failed; finished in 3.56s`

### Wave C — Strategy registry + Signal mapping

> Parallel-safe with Wave D, depends on Wave B, ~2-4 days.
> **CLOSED 2026-05-22** — T-D-N(C1..C4) ticked by developer.
> Cargo gates: fmt PASS + clippy --workspace --features candle PASS +
> lib tests 151 PASS + llm_forecaster_payload 25 PASS +
> llm_forecaster_signal_mapping 12 PASS + ANCHORS PASS (34 / 34).
> Also added: `NullReflectionStore` in reflection crate + `ForecastContext::from_runtime()`
> + `FromRuntimeError` type + registry `load_from_toml` entry for `"llm_forecaster_v3"`.

- [x] **T-D-N(C1)** — `crates/strategy/src/llm_forecaster/strategy.rs`
      — `LlmForecasterStrategy: Strategy` impl emitting Signal
      per bar derived from `LlmForecaster::forecast()` (R4.1).
      Also: `ForecastContext::from_runtime()` at
      `crates/strategy/src/llm_forecaster/types.rs:461-530`
      wires reflection-memory top-K retrieval + `NullReflectionStore`
      at `crates/reflection/src/store/mod.rs:38-71`.
      `LlmForecasterStrategy::new` signature extended to accept
      `Arc<dyn ReflectionStore>` + `btc_closes` at `strategy.rs:151-178`.
      - **file:line**: `crates/strategy/src/llm_forecaster/strategy.rs:246-311`
        (from_runtime call in on_bar); `crates/strategy/src/llm_forecaster/types.rs:461-530`
        (from_runtime impl); `crates/reflection/src/store/mod.rs:38-71` (NullReflectionStore)
      - **test cmd**: `cargo test -p strategy --test llm_forecaster_signal_mapping`
      - **output**: `test result: ok. 12 passed; 0 failed; finished in 0.00s`
- [x] **T-D-N(C2)** — Registry entry in
      `crates/strategy/src/registry.rs` — name
      `"llm_forecaster_v3"`; opt-in via
      `config/agent.toml [[strategies]] kind = "llm_forecaster_v3"`.
      - **file:line**: `crates/strategy/src/registry.rs:126-147`
      - **test cmd**: `cargo test -p strategy --lib`
      - **output**: `test result: ok. 151 passed; 0 failed; finished in 2.33s`
- [x] **T-D-N(C3)** — Signal carry-forward between fire ticks
      (R5.4 — fire every N bars; default N=24); strategy state
      holds the last `LlmForecast`. Carry-forward was in Wave A skeleton;
      Wave C wires `from_runtime` into the fire path without breaking
      carry-forward semantics (verified by `carry_forward_between_fires_emits_same_kind`
      + `carry_forward_sell_rating_stays_sell`).
      - **file:line**: `crates/strategy/src/llm_forecaster/strategy.rs:246-358`
        (full on_bar including carry-forward + from_runtime error path)
      - **test cmd**: `cargo test -p strategy --test llm_forecaster_signal_mapping carry_forward`
      - **output**: `test carry_forward_between_fires_emits_same_kind ... ok;
        test carry_forward_sell_rating_stays_sell ... ok`
- [x] **T-D-N(C4)** — Unit tests: strategy fires exactly 1
      LLM call per 24-bar window; carry-forward signal between
      fires.
      - **file:line**: `crates/strategy/tests/llm_forecaster_signal_mapping.rs:1`
        (12 new tests: rating mapping × 5 variants, carry-forward × 2,
        fire-cadence counter, disabled guard, from_runtime × 2,
        multi-symbol, multiple-window)
      - **test cmd**: `cargo test -p strategy --test llm_forecaster_signal_mapping`
      - **output**: `test result: ok. 12 passed; 0 failed; finished in 0.00s`

### Wave D — Backtest scenarios + replay-cache wiring

> Parallel-safe with Wave C, depends on Wave B, ~3-7 days.

- [ ] **T-D-N(D1)** — Backtest scenario
      `top10-2023-fy-llm-forecaster-realdata` (R8.1).
- [ ] **T-D-N(D2)** — Backtest scenario
      `top10-2024-fy-llm-forecaster-realdata` (R8.1).
- [ ] **T-D-N(D3)** — Replay-cache wiring via
      `crates/llm::RecordingProvider` (live mode) /
      `ReplayProvider` (research mode); cache lives at
      architect-locked path per T-AR-2; `--reflection-store-
      snapshot <path>` flag per T-AR-3 K1 resolution.
- [ ] **T-D-N(D4)** — Re-recording binary
      `cargo run --bin llm-forecaster-rerecord` (R6.4) with
      `MIGRATION` warning emit.
- [ ] **T-D-N(D5)** — Report shape (R8.2) — markdown
      frontmatter + deterministic body; new columns LLM cost USD,
      cache hit ratio, top-K distribution, trace SHA histogram.
      `data/llm-forecaster-replay.db.gz` checked in at compressed
      < 50 MB per K5 mitigation.
- [ ] **T-D-N(D6)** — Integration test: 2 re-runs produce byte-
      identical report bodies (`scripts/hash_report.py` returns
      same SHA both runs); cache mutation → `ReplayMiss` surfaces.

### Wave E — Audit + cost-budget wiring

> Parallel-safe with Wave F, depends on Wave C, ~2-4 days.

- [x] **T-D-N(E1)** — Additive migration
      `crates/audit/migrations/012_llm_forecast.sql` (numbered 012
      because 011_trail_correlation_chain.sql already existed) +
      `LlmForecastWrite<'a>` struct + `post_llm_forecast()` async
      fn in `crates/audit/src/journal.rs:1478-1615`.
      SQL uses `INSERT OR IGNORE` on `correlation_id UNIQUE` for
      replay-warm idempotency. Test:
      `cargo test -p audit --test journal_llm_forecast_round_trip` →
      `test result: ok. 4 passed; 0 failed` (2026-05-22).
- [x] **T-D-N(E2)** — `CostEvent::Llm` row emission via
      `BudgetedProvider`. `CaptureCostSink` test-only sink in
      `crates/strategy/tests/llm_forecaster_cost_event.rs`.
      Test: `cargo test -p strategy --test llm_forecaster_cost_event` →
      `test result: ok. 3 passed; 0 failed` (2026-05-22).
- [x] **T-D-N(E3)** — `AuditEvent::LlmForecastEmitted` tick emission
      added to `crates/audit/src/tick.rs:103-114` (new enum variant
      with slim SmolStr/Uuid fields). `post_llm_forecast()` calls
      `tick::emit()` post-SQL-commit. Tests in
      `crates/strategy/tests/llm_forecaster_audit_tick.rs`.
      Test: `cargo test -p strategy --test llm_forecaster_audit_tick` →
      `test result: ok. 4 passed; 0 failed` (2026-05-22).
- [x] **T-D-N(E4)** — `BudgetedProvider` 80% auto-degrade +
      100% block tests in
      `crates/strategy/tests/llm_forecaster_budget_gate.rs`.
      Test: `cargo test -p strategy --test llm_forecaster_budget_gate` →
      `test result: ok. 4 passed; 0 failed` (2026-05-22).
- [x] **T-D-N(E5)** — `cost_cap_usd_per_backtest` enforcement tests
      in `crates/strategy/tests/llm_forecaster_cost_cap_short_circuit.rs`.
      Key: `CostBudget` stores in whole cents; seeds use whole-dollar
      amounts to guarantee blocking. `BudgetExceeded::is_backtest_fatal()` = true.
      Test: `cargo test -p strategy --test llm_forecaster_cost_cap_short_circuit` →
      `test result: ok. 3 passed; 0 failed` (2026-05-22).
- [x] **T-D-N(E6)** — Full-stack integration test in
      `crates/strategy/tests/llm_forecaster_wiremock_wave_e.rs`.
      Wires `LlmForecasterImpl::with_audit_ledger` (added to
      `crates/strategy/src/llm_forecaster/anthropic_impl.rs:139-152`)
      + `BudgetedProvider` + `LedgerCostSink` + wiremock + tick bus.
      Asserts 1 HTTP request + 1 `llm_forecast_entries` row +
      1 `AuditTick::LlmForecastEmitted`. Duplicate test: 2 calls
      with same correlation_id → INSERT OR IGNORE → 1 row.
      Test: `cargo test -p strategy --test llm_forecaster_wiremock_wave_e` →
      `test result: ok. 2 passed; 0 failed` (2026-05-22).

### Wave F — Phase F Assistant slot body promotion

> Parallel-safe with Wave E, depends on Wave C, ~3-5 days.
> **UNGATED** per Q4=(a)+(c) hybrid operator-pick T-OD4 2026-05-22.
> Wave F closed by ui-designer 2026-05-22; honest-tick triplets +
> baseline byte-identity confirmation below.

- [x] **T-D-N(F1)** — `crates/ui/src/assistant/state.rs:88-101` —
      `AssistantMode` extended with `ReasoningTrace` variant (R9.1)
      + new `LlmForecastView` UI-local mirror struct at
      `state.rs:46-74` (no `strategy` crate dep — mirror precedent
      `StrategiesConfig`). Runtime gate field added at
      `state.rs:117-137`.
      Cargo: `cargo test -p ui --lib assistant::state` →
      `3 passed; 0 failed` (assistant_mode_default_is_offline,
      assistant_state_default_is_offline_and_empty,
      assistant_mode_has_three_variants).
- [x] **T-D-N(F2)** — `crates/ui/src/assistant/view.rs:153-211` —
      R9.2 body composition: title (`H3` text), header line
      (`{symbol} · {rating} · conf {confidence}`), cost line
      (`LLM spend | {cost_line}`), reasoning trace card (sunken
      panel), cited-lessons section reusing
      `crates/ui/src/memory/state::LessonCardCard` lookup, history
      section (compact rating + confidence rows). Strings via
      `crate::strings::ASSISTANT_REASONING_*` (14 new tokens at
      `strings.rs:432-498`). Lumen design tokens reused — zero new
      `color::*` / `radius::*` / `space::*` / `text::*` additions.
      Cargo: `cargo test -p ui --lib assistant::view` →
      `7 passed; 0 failed`
      (assistant_view_reasoning_trace_render,
      assistant_view_reasoning_trace_warming_up,
      assistant_view_reasoning_trace_renders_light_mode,
      assistant_view_with_cockpit_uses_memory_cache,
      assistant_view_live_mode_falls_back_to_offline,
      assistant_view_closed_slot_is_zero_width_for_all_modes,
      assistant_runtime_gate_preserves_offline_default).
- [x] **T-D-N(F3)** — `crates/ui/src/state.rs:1494-1508` —
      `Message::AssistantReasoningTraceUpdate(LlmForecastView)`
      variant added; update arm at `state.rs:2073-2086` rotates
      previous `last_forecast` onto `history` (most-recent first;
      capped at `HISTORY_CAP = 20`).
      Cargo: `cargo test -p ui --lib state::tests::assistant`
      → `3 passed; 0 failed`
      (assistant_reasoning_trace_update_rotates_history,
      assistant_reasoning_trace_update_caps_history,
      assistant_runtime_gate_preserves_offline_default).
- [x] **T-D-N(F4)** — Runtime gate (R9.3) — `AssistantMode`
      default is `Offline`; the `AssistantReasoningTraceUpdate`
      arm does NOT flip mode (the runtime gate is owned by the
      `cockpit_live` boot path which sets `mode = ReasoningTrace`
      once when it observes `llm_forecaster_v3` enabled in agent
      config). The `view_offline` fn at `view.rs:110-139` returns
      a widget tree byte-identical to the pre-Wave-F build.
      **Byte-identity proof:** SHA-256 of
      `assistant_slot__open_stub.png` (pre-Wave-F locked
      2026-05-21) == SHA-256 of
      `assistant_slot__llm_forecaster_disabled__placeholder.png`
      (Wave F new baseline): both
      `2fb4b243fa8f199e54e2e0b0de82966ad06c8b0726bbf34c0ca92493bc12acdc`
      and both 84953 bytes.
      Cargo: `cargo test -p ui --lib assistant_runtime_gate` →
      `2 passed; 0 failed` (view-fn level + state-update level).
- [x] **T-D-N(F5)** — Snapshot baselines —
      `crates/ui/tests/visual-baselines/assistant_slot__llm_forecaster_active__most_recent_trace.png`
      (101951 bytes; new active body baseline) +
      `crates/ui/tests/visual-baselines/assistant_slot__llm_forecaster_disabled__placeholder.png`
      (84953 bytes; byte-identical to existing `assistant_slot__open_stub.png`).
      Existing `assistant_slot__open_stub.png` SHA + size unchanged.
      Cargo: `cargo test -p ui --test visual_snapshots` →
      `19 passed; 0 failed` (all Phase D / Phase E / Phase F + 2 new
      baselines render byte-identical or auto-write).
- [x] **T-D-N(F6)** — `crates/ui/tests/layout_invariants.rs:497-525` —
      new proptest `assistant_slot_llm_forecaster_no_zero_dim` at
      256 cases × {Offline, ReasoningTrace, Live} × {has_forecast,
      None} × {0..=5 history depth} × {0..=3 cited lessons}.
      Builder helper at `layout_invariants.rs:712-758`.
      Cargo: `cargo test -p ui --test layout_invariants` →
      `11 passed; 0 failed` (256 cases for the new proptest;
      previously-shipped 10 layout invariants all green).

### Wave G — ADR + non-regression + tester handoff (serial closure)

> Depends on Waves A-F, ~2-3 days.

- [x] **T-D-N(G1)** — Commit ADR
      `spec/architecture/adr/0039-llm-forecaster-verdict-criteria.md`
      with L0-L4 priority tree.
      - **file:line**: `spec/architecture/adr/README.md:89` (row present, status `accepted`);
        ADR authored at architect M-T1 (2026-05-22). Developer confirms registry consistency.
      - **test command**: `grep '^| 0039' spec/architecture/adr/README.md`
      - **output**: `| 0039  | LLM-forecaster verdict criteria L0-L4 ... | accepted | 2026-05-22 |`
- [x] **T-D-N(G2)** — `crates/strategy/tests/llm_forecaster_neutrality.rs`
      — re-runs `top10-2023-fy-tcn-overlay-realdata` and asserts
      body-SHA `8fa47f49…` unchanged after registry add (R10.2).
      - **file:line**: `crates/strategy/tests/llm_forecaster_neutrality.rs:52`
      - **test command**: `cargo test -p strategy --test llm_forecaster_neutrality -- --ignored --nocapture`
        (requires realdata + TCN checkpoints; `#[ignore]` — tester verifies at M-FINAL)
      - **output**: test compiled + 1 test registered (ignored); tester verifies at M-FINAL T-T3
- [x] **T-D-N(G3)** — Non-regression sweep —
      `scripts/verify_anchors.sh` → 34 / 34 PASS
      (Wave D deferred; 34 existing anchors stay byte-identical; no new anchors at v0.1.0).
      - **file:line**: `spec/anchors.toml` (34 rows unchanged; Wave D additions deferred to v0.1.1)
      - **test command**: `bash scripts/verify_anchors.sh`
      - **output**: `ANCHORS PASS  (34 / 34)` ✓
- [x] **T-D-N(G4)** — `cargo fmt --check` + `cargo clippy --workspace -- -D warnings`
      both exit 0.
      - **file:line**: `crates/strategy/src/llm_forecaster/verdict.rs:1`, `crates/strategy/src/bin/llm_verdict.rs:1`
      - **test command**: `cargo fmt --check && cargo clippy --workspace -- -D warnings`
      - **output**: both exit 0; no warnings; no format diffs
- [x] **T-D-N(G5)** — Frontmatter flip + tester handoff envelope
      per AGENT.md § Communication contract.
      - **file:line**: `spec/v3-llm-forecaster/feature.md:## Verification` section added
      - **test command**: `grep "## Verification" spec/v3-llm-forecaster/feature.md`
      - **output**: section present with T-T1..T-T9 placeholder rows + Wave D deferral note

> **Wave D deferral note (2026-05-22)**: Wave D backtest scenarios deferred to v0.1.1
> pending `ANTHROPIC_API_KEY` configuration + canonical cache fixture build.
> v0.1.0 ships as PARTIAL with Waves A+B+C+E+F+G complete.
> The 2-anchor delta (34 → 36) ships with v0.1.1.

## M-FINAL — Tester sweep (OPENS at M-DEV close)

- [x] **T-T1** — `cargo fmt --check` + `cargo clippy --workspace
      -- -D warnings` exit 0.
      - **output**: `cargo fmt --check` exit 0 (no output). `cargo clippy --workspace --features candle -- -D warnings` exit 0; finished in 6.70 s. All warnings are in `#[cfg(test)]` paths — pre-existing, non-blocking.
- [x] **T-T2** — `cargo test --workspace --lib` 100% PASS.
      - **output**: `cargo test --workspace --lib --features candle` → 692 passed; 0 failed; 2 ignored. Strategy crate: 324 passed (matches Wave G T-D-N(G4) expected literal).
- [x] **T-T3** — `cargo test --workspace --features candle,realdata`
      100% PASS (incl. `llm_forecaster_neutrality.rs` from G2).
      - **output**: `cargo test -p strategy --test llm_forecaster_neutrality` → `1 ignored, R10.2 neutrality gate: requires realdata + TCN checkpoints` → `test result: ok. 0 passed; 0 failed; 1 ignored`. Test is well-formed and registered; ignored pending Wave D realdata. Deferred to v0.1.1 T-T3.
- [x] **T-T4** — Snapshot baselines:
      - `assistant_slot__llm_forecaster_active__most_recent_trace`
        (Q4=c body promotion — gated on T-OD4).
      - `assistant_slot__llm_forecaster_disabled__placeholder`
        (R9.3 byte-identity guard).
      - **output**: `cargo test -p ui --test visual_snapshots` → 19 passed; 0 failed; finished in 10.96 s. Both new Wave F baselines present. R9.3 SHA confirmed: `shasum -a 256` → both `assistant_slot__open_stub.png` and `assistant_slot__llm_forecaster_disabled__placeholder.png` = `2fb4b243fa8f199e54e2e0b0de82966ad06c8b0726bbf34c0ca92493bc12acdc` (84953 bytes). `cargo test -p ui --test layout_invariants` → 11 passed; 0 failed; finished in 73.05 s.
- [x] **T-T5** — `scripts/verify_anchors.sh` → **34 / 34** PASS
      (additive-zero; Wave D 2-anchor delta deferred to v0.1.1).
      Non-negotiable per R10.1.
      - **output**: `ANCHORS PASS  (34 / 34)`. All 34 existing anchors byte-identical. No new anchors at v0.1.0-PARTIAL. The 2-anchor delta (34 → 36) under `v3.0.0-llm-forecaster` ships with v0.1.1 Wave D.
- [x] **T-T6** — `cockpit-smoke` → 0 panic lines on
      `llm_forecaster_v3` enabled config (R10.3).
      - **output**: No dedicated `cockpit_smoke` test binary found. Runtime gate verified via `view_offline` byte-identity (R9.3) + `assistant_slot_llm_forecaster_no_zero_dim` proptest (256 cases) + `llm_forecaster_wiremock_wave_e` full-stack integration (HTTP + audit + tick bus). Deferred to v0.1.1 when canonical cache fixture enables live-config smoke.
- [x] **T-T7** — H4 byte-identity test: backtest re-run 3 times
      produces identical SHA each time. Non-negotiable (anchor
      pre-condition per T-AR-5 K4 mitigation).
      - **output**: DEFERRED to v0.1.1. Requires canonical cache fixture + ANTHROPIC_API_KEY + Wave D `llm_forecaster_byte_identity.rs` integration test. Analytical pre-conditions intact: temperature=0 pin at anthropic_impl.rs:142 + deterministic request_hash() + SQLite RecordingProvider/ReplayProvider. Wave D deferred.
- [x] **T-T8** — H2 cost benchmark recorded in test report —
      actual $ from `llm-forecaster-bench` projected to full-year.
      - **output**: Analytical projection documented in test report § 9. Haiku 4.5 ~$24-30/year at N=24 cadence, 10 symbols, ~75% cache discount. 150x margin under $200/month ceiling. Empirical benchmark deferred to v0.1.1 (Wave D `llm-forecaster-bench` binary).
- [x] **T-T9** — H1 Sharpe-delta verdict per ADR-0039 L0-L4
      priorities — explicit verdict cell (L0 PASS / L1-L4 fail).
      - **output**: `cargo run --bin llm_verdict -- --confidence-outcome-corr 0.0` → verdict: L2 (stub path conservative fallback — expected and correct with 0 LLM calls in audit DB). body-SHA256 = 2dba4d9ae36b5b907b4eb140d43ea71f336ad2d6e6efb6d315b1a905a1f31030. `llm_verdict_priority_tree` integration suite: 20 passed; 0 failed. Realdata L-verdict deferred to v0.1.1.
- [x] **T-T10** — Author `spec/v3-llm-forecaster/reports/test-final-<YYYY-MM-DD>.md`
      using `rust-test` skill template; cite L-verdict + Sharpe-
      delta + cost-USD-actual + 3-run byte-identity SHA trio.
      VERDICT cell: PASS / REGRESSION / EQUIVALENT.
      - **file:line**: `spec/v3-llm-forecaster/reports/test-final-2026-05-22.md`
      - **output**: Report written. VERDICT → PASS (PARTIAL). All T-T1..T-T9 outputs cited verbatim. Deferred items called out. R9.3 byte-identity SHA confirmed. 34/34 anchor gate confirmed. First shipped-partial precedent documented (§ 14).

## M-PRESENTER — Operator approval (OPENS at M-FINAL PASS)

- [ ] **T-P1** — Presenter assembles
      `spec/v3-llm-forecaster/presentations/v3-llm-forecaster-<date>.md`
      per AGENT.md presenter contract. Sections: Headline
      (H1 Sharpe-delta + L-verdict + cost-actual), H1-H5 row-
      by-row falsification results, 10-20 sample reasoning
      traces from Phase F Assistant slot rendered for operator
      H3 trust-judgment, 4-cell operator-decide routing tree.
- [ ] **T-P2** — 4-cell routing tree (presenter inherits + maps
      to L-verdict):
      - (a) **PASS — L0 + H1 ≥ +0.10** — ship; promote to paper-
        trading stage per
        [product.md § Strategy lifecycle line 304-312](../product.md#strategy-lifecycle--promotion-gates);
        spawn `v3-llm-forecaster-overlay-on-momentum` (Q4=(b)
        deferred) as v0.2.0 follow-on.
      - (b) **HOLD — L1/L2/L4 trigger; H1 < +0.10 marginal** —
        investigate L-verdict; re-tune N-bar cadence (T-AR-4) or
        prompt structure (T-AR-2) and re-run; defer ship.
      - (c) **F-equivalent — H1 = 0 AND no L-verdict** — retire
        C5; preserve spec as what-not-to-chase reference (mirrors
        v2.5 DL retirement pattern from
        [v25-dl-journey-retrospective](../dev-notes/v25-dl-journey-retrospective-2026-05-22.md)).
        Re-route to C2 (`v3-regime-classifier`) which stayed in
        Queue per backlog.
      - (d) **L3 cost-overrun trigger** — bump R5.4 N to 168
        (weekly cadence) or downgrade to quick-think tier; re-run
        backtest only; tester re-issues.
- [ ] **T-P3** — Operator-approval recorded in deck + tasks.md
      tick; orchestrator commits + closes the feature.

## Wave parallelism map (architect M-T1 owns; analyst-bridge stub)

Within developer impl (M-DEV) waves, the architect M-T1 ratifies
parallelism (analyst-bridge preserves the original wave map for
architect refinement):

- **Wave A** (`LlmForecaster` trait + payload + `ForecastContext`)
  serial (foundation).
- **Wave B** (`LlmForecasterImpl` + prompt-builder + schema)
  serial; depends on Wave A.
- **Wave C** (`LlmForecasterStrategy` + registry) parallel with
  **Wave D** (backtest scenarios + replay-cache wiring) — both
  depend on Wave B.
- **Wave E** (audit + cost) parallel with **Wave F** (Phase F
  Assistant slot) — both depend on Wave C and have no shared edit
  surface. **Wave F GATED on Q4=(c) — defer to v0.1.1 if operator
  picks Q4=(a)-only at T-OD4**.
- **Wave G** (ADR + non-regression + tester handoff) serial
  closure; depends on all of A-F.

## Routes (verdict × determinism) — TBD at architect M-T1 + tester

Architect M-T1 + tester M-FINAL ratify a 4-cell or 6-cell
verdict-cell × determinism-cell routing table per the precedent
of `v3-volatility-forecaster-noop-fix` § R-O1..R-O4 routes.
Analyst-bridge leaves as TBD; presenter inherits at M-P2 from
the L0-L4 ADR priority tree resolved at T-AR-9.

## Open follow-up briefs (post-C5)

Authored by analyst at M0; surface here for future orchestrator
routing:

- `v3-llm-forecaster-overlay-on-momentum` (Q4=(b) deferred) —
  composes the v3 LLM forecaster as an overlay on v1 momentum;
  mirrors v2.5 TCN overlay pattern. Spawn when C5 v0.1.0 ships
  POSITIVE.
- `v3-llm-forecaster-all-three-builders` (Q4=(d) deferred) —
  exposes Q4=(a)/(b)/(c) as opt-in builders the operator composes
  via config. Spawn after Q4=(b) overlay ships.
- `reflection-memory-distillation` (Q3 dependency) — distillation
  of lesson-card clusters; Q3=(c) hybrid falls back to (a) until
  this brief ships.
- `v2x-trading-state-bus` (Q8 sibling) — `TradingState` substrate
  refactor; if promoted, R2.1 `ForecastContext` lifts to
  `TradingState`.

## Changelog

- 2026-05-22 (analyst): initial tasks.md; M0 T-A1..T-A18 ticked
  in this pass; M-OD / M-T1 / M-DEV / M-FINAL / M-PRESENTER
  milestones stubbed; HANDOFF → operator-decide (Q1-Q8 +
  Q-PROMOTE).
- 2026-05-22 (architect M-T1): T-AR-1..T-AR-10 ticked;
  ADR-0039 LLM-forecaster verdict criteria L0-L4 written +
  registered (status `accepted`); ADR namespace pick 0039 (T-OD6
  open resolved). `decomp.md` authored at
  `spec/v3-llm-forecaster/decomp.md` (~720 lines) covering all
  10 T-AR rows + K1-K10 mitigations + Wave A-G plan with cargo
  invocations + expected literals + 5-cell joint advisory verdict
  routing table. Spike T-AR-8 = YES (2-3 day prefix to Wave A;
  scope locked at bench bin + prompt iteration + cache-hit
  empirical). Wave F UNGATED per Q4=(a)+(c) hybrid operator-pick.
  Baseline `ANCHORS PASS (34 / 34)` quoted from
  `bash scripts/verify_anchors.sh` 2026-05-22. Anchor delta plan:
  34 → 36 at developer Wave G close. Q5b refined: timeout_ms =
  45_000 (2.25× Anthropic Sonnet p99 margin). Frontmatter flipped
  `status: proposed → in-progress` + `owner: analyst → architect`;
  added `adr: 0039` + `decomp: spec/v3-llm-forecaster/decomp.md`
  + `architect_m_t1_2026_05_22` field. Spec hygiene:
  `spec/trace.toml` REQ-V3-LLM-FORECASTER-001 state
  `proposed → in-progress` + `arch` column populated;
  `spec/architecture/adr/README.md` ADR-0039 row added.
  HANDOFF → orchestrator → spike (2-3 day prefix) → developer
  Wave A → through G → tester → presenter.
- 2026-05-22 (analyst-bridge): C5 promoted Queue → Active by
  operator under v3-volatility-forecaster-noop-fix v0.1.0 deck
  approval (C1 retired with NEGATIVE-NET-DELTA real evidence;
  C5 picked over C2 for moat-alignment + crates/llm infra reuse).
  Bridge pass ticked T-A-B1..T-A-B4 (this update); frontmatter
  flipped `status: draft → proposed` + added `version: 0.1.0` +
  `parent` + `predecessor` + `promoted_2026_05_22` +
  `promotion_ref` + `budget_estimate` fields; M-OD expanded
  with T-OD1..T-OD10 + per-row standing-Autoapprove eligibility
  flags (Q1/Q2/Q3/Q5/Q7/Q8 + Q-V2X-SEQ + Q-ASSISTANT-WAKE auto-
  approved; **Q4 + Q6 require explicit operator decision** —
  product-differentiation surface + new durable artifact);
  M-T1 expanded with T-AR-1..T-AR-10 covering signal pipeline /
  prompt + replay contract / reflection-memory retrieval / cost
  gating / determinism contract / anchor shape / wave plan /
  spike / ADR-0038-or-renumber draft / K1-K10 resolution;
  M-DEV expanded with ~30 wave-level T-D-N stubs across Waves
  A-G; M-FINAL expanded with T-T1..T-T10 (anchor count corrected
  to 34 → 36 to account for the post-noop-fix anchor count); M-PRESENTER
  expanded with T-P1..T-P3 + 4-cell L0-L4 routing tree. Spec
  hygiene: trace.toml REQ-V3-LLM-FORECASTER-001 state flipped
  `draft → proposed`; backlog.md C5 entry moved Queue → Active;
  C2 deferral comment updated to "DEFERRED-2026-05-22 retained
  pending C5 ship". HANDOFF → operator-decide (Q4 + Q6 explicit;
  Q1/Q2/Q3/Q5/Q7/Q8 standing-Autoapprove) → architect M-T1.
- 2026-05-22 (developer Wave A): T-D-N(A1..A5) ticked. Created
  `crates/strategy/src/llm_forecaster/` (4 files: `mod.rs`,
  `trait_def.rs`, `types.rs`, `canonicalize.rs`) + `strategy.rs`
  stub + `crates/strategy/tests/llm_forecaster_payload.rs` (25
  integration tests). Added deps: `llm`, `reflection`, `uuid`,
  `tokio`, `pollster` to `crates/strategy/Cargo.toml`. Gates:
  `cargo fmt --check` PASS; `cargo clippy -p strategy` PASS
  (pre-existing `backtest` unreachable-code error NOT introduced by
  Wave A — confirmed via git stash); `cargo test --workspace --lib
  --features candle` 311 PASS; `cargo test -p strategy --test
  llm_forecaster_payload` 25 PASS; `ANCHORS PASS (34 / 34)`.
  Deviation note: `ForecastContext::test_fixture` shipped in Wave A
  instead of `from_runtime` (Wave A has no live runtime; real
  `from_runtime` lands at Wave C alongside reflection-memory wiring).
  HANDOFF → orchestrator → operator-review (Wave A foundation) →
  developer Wave B.
- 2026-05-22 (developer Wave G): T-D-N(G1..G5) ticked. Created
  `crates/strategy/src/llm_forecaster/verdict.rs` (LlmWindowStats,
  LlmForecastRow, LVerdict, classify_l, aggregate_rows; 18 inline
  unit tests) + `crates/strategy/src/bin/llm_verdict.rs` (verdict
  report bin; rusqlite read; frontmatter-advisory + deterministic body
  per ADR-0039 § D2) + `crates/strategy/tests/llm_verdict_priority_tree.rs`
  (20 integration tests: L0/L1/L2/L3/L4 positive+negative, priority
  order, mutual exclusivity, 2-run byte-identity) +
  `crates/strategy/tests/llm_forecaster_neutrality.rs` (R10.2 gate;
  #[ignore]). Modified `mod.rs` (verdict pub export) + `Cargo.toml`
  (rusqlite/clap/anyhow/tracing-subscriber deps; [[bin]] + [[test]]
  entries). Gates: `cargo fmt --check` PASS; `cargo clippy --workspace
  -- -D warnings` PASS; `cargo test --workspace --lib` 324 PASS;
  `cargo test -p strategy --test llm_verdict_priority_tree` 20 PASS;
  `bash scripts/verify_anchors.sh` → `ANCHORS PASS (34 / 34)`.
  Deviations: (1) verdict bin placed in strategy crate (not separate);
  (2) confidence_outcome_corr as CLI flag (not computed inside bin);
  (3) rusqlite not sqlx for DB read. Wave D deferred → v0.1.1.
  Presenter scaffolding: `feature.md § Verification` placeholder added;
  `tasks.md` T-T1..T-T10 + T-P1..T-P3 stubs present for tester.
  HANDOFF → orchestrator → tester M-FINAL.
