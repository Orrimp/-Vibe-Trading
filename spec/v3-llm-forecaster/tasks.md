---
slug: v3-llm-forecaster
version: 0.1.0
status: proposed
owner: analyst
updated: 2026-05-22
parent: strategy-reformulation-survey-2026-05-22 Candidate 5
predecessor: v2-llm-strategy v2.0.0
promoted_2026_05_22: Queue → Active by operator under v3-volatility-forecaster-noop-fix v0.1.0 deck approval (C1 retired with NEGATIVE-NET-DELTA evidence; C5 picked over C2 for moat-alignment + crates/llm infra reuse)
promotion_ref: spec/v3-volatility-forecaster-noop-fix/presentations/v3-volatility-forecaster-noop-fix-2026-05-22.md
analyst_bridge_2026_05_22: T-A-B1..T-A-B4 closed; M-OD opens with standing-Autoapprove eligible on Q1/Q2/Q3/Q5/Q7/Q8 and explicit-decision required on Q4 (Phase F Assistant slot promotion shape — product-differentiation surface) + Q6 (NEW ADR-0038 LLM-verdict criteria — codifies a new artifact)
budget_estimate: 6-8 weeks total wall-clock (analyst 1w done + architect 1-2w + dev 3-5w + tester 1w + presenter 1-2d per survey ranking Candidate 5 line 480; HIGH variance per K8 novel-territory risk)
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

## M-T1 — Architect decomposition (OPENS at M-OD close)

Architect ratifies feature.md into `decomp.md` + ADR (0038 or
renumber per T-OD6 open) + ordered Wave plan. Tasks below are
the architect's expected surface — analyst-bridge sized to ~10
T-AR rows. Architect refines + adds file:line citations at M-T1.

> **Critical M-T1 path** — architect MUST resolve K1 + K4 + K5
> + Q6 ADR shape before Wave A spawns. Q5 sub-decision Q5b
> timeout-ms is a Wave B refinement (lower-priority).

- [ ] **T-AR-1** — **Signal pipeline shape**. Lock where
      `LlmForecaster::forecast()` output feeds into a Strategy.
      Analyst-recommended: new `LlmForecasterStrategy: Strategy`
      at `crates/strategy/src/llm_forecaster/strategy.rs`
      (Q4=(a) default per R4.1); registered as
      `"llm_forecaster_v3"` in
      [`crates/strategy/src/registry.rs`](../../crates/strategy/src/registry.rs).
      Architect M-T1 cites file:line for the registry add + the
      Strategy::on_bar call sequence (forecast → Signal mapping
      via `Rating::to_signal_overlay`). Mirrors structurally the
      vol-target overlay's standalone-strategy pattern (now
      retired post-noop-fix), not the TCN-overlay's Signal-kind
      mutation pattern.
- [ ] **T-AR-2** — **Prompt + replay-cache contract**.
      Confirm `crates/llm::CachedSystemPromptBuilder` 2-cache-
      breakpoint structure (R3.2 project + role boundaries);
      lock the prompt fixture path + replay-cache namespace.
      Decisions:
      - Replay-cache path — `data/llm-forecaster-replay.db` (live
        recording) vs shared `data/llm-replay.db` with namespaced
        `(strategy_id, request_hash, response)` rows.
      - Fixture path for checkout-friendly tests —
        `crates/strategy/tests/fixtures/llm-forecaster-replay.db.gz`
        (analyst-recommended per K5; mirrors
        `crates/llm/tests/fixtures/llm-replay.db` precedent).
      - Canonical `ForecastContext::request_hash()` serialisation
        (R6.6) — analyst-strawman serde_json with sorted keys.
- [ ] **T-AR-3** — **Reflection-memory retrieval shape**.
      Resolve K1 (top_k determinism under backtest re-runs).
      Decisions:
      - Backtest binary `--reflection-store-snapshot <path>`
        flag (analyst-recommended): pins the store to a frozen
        sqlite dump for re-run byte-identity safety beyond just
        replay-cache hash-pinning.
      - `RetrievalQuery` derivation from `(symbol, regime_tag,
        recent_outcome)` per R2.3 — confirm
        `crates/reflection::RetrievalQuery` shape supports all 3
        inputs; if not, surface as architect-add via additive
        field.
      - K = 5 default per `REPORT_TIME_TOP_K` (existing constant)
        — confirm consumer doesn't need a strategy-specific override.
- [ ] **T-AR-4** — **Cost gating + budget kill-switch**.
      Resolve K2 (LLM cost blow-up). Bench actual token counts
      via `cargo run --bin llm-forecaster-bench` on a 1-month
      data slice; project to full year. Calibrate R5.4
      `fire_every_n_bars` default (analyst-strawman N=24
      once-per-day hourly cadence → ~$55/year cold-run on 10
      symbols, ~$10-15 warm-run with cache hits). Lock R5.3
      `cost_cap_usd_per_backtest` default (analyst-strawman
      $20). The kill-switch path: `BudgetExceeded` short-circuit
      to backtest binary explicit-error log.
- [ ] **T-AR-5** — **Determinism contract — replay-cache must
      serve byte-identical responses for backtest determinism**.
      Resolve K4 (Anthropic drift across server deploys) +
      R6.6 (canonical request_hash serialisation). Decisions:
      - Anchor only after **3 back-to-back identical cache-build
        runs** (analyst-recommended; tightest re-run gate beyond
        the H4 single-byte-identity falsification).
      - `cache_schema_version` field (R6.5) on
        `(request_hash, response)` rows — additive migration shape
        owned by architect.
      - Re-recording protocol (R6.4) — explicit
        `cargo run --bin llm-forecaster-rerecord` binary; emits a
        `MIGRATION` warning per v25-tcn-overlay precedent.
- [ ] **T-AR-6** — **Anchor shape — where LLM-forecast reports
      live + body-SHA contract**. Per Q7=(a) default new version
      pin `v3.0.0-llm-forecaster`; +2 anchors at ship:
      - `top10-2023-fy-llm-forecaster-realdata` — full-year 2023
        realdata on 10 USDT pairs.
      - `top10-2024-fy-llm-forecaster-realdata` — full-year 2024.
      Architect locks the report-body template (R8.2 mirror of
      v25a-patchtst-overlay R8) + the new columns (LLM cost
      USD total + per-call; cache hit ratio target ≥90% on
      re-runs; top-K lesson-card retrieval distribution;
      reasoning_trace_sha256 histogram).
- [ ] **T-AR-7** — **Wave plan A-G ratified**. Confirm the
      analyst-strawman wave map (preserved from the original
      analyst pass; subject to architect refinement at M-T1):
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
- [ ] **T-AR-8** — **Spike requirement**. C5 is novel-territory
      per K8 + survey LOW-MEDIUM prior on H1 +0.10 Sharpe-delta.
      Architect M-T1 decides: spike YES (recommended — 2-3-day
      Wave-A-prefix prompt-engineering + token-count + cache-hit-
      ratio bench on a 1-week realdata slice before committing
      to Wave A) vs spike NO (direct Wave A entry; higher
      schedule risk if prompt structure needs iteration).
      Analyst-recommended: **spike YES**.
- [ ] **T-AR-9** — **Draft ADR (0038 or 0039 — see T-OD6 open)
      "LLM-forecaster verdict criteria L1-L4"** per Q6=(b).
      Priority tree (analyst-strawman; architect refines):
      L1 = bias collapse (≥95% HOLD ratings); L2 = calibration
      failure (confidence-outcome correlation < threshold);
      L3 = cost overrun (actual / projected > 2×); L4 =
      reasoning trace degenerate (< 50 chars OR > 50% duplicate);
      L0 = PASS (none of L1-L4 trigger AND Sharpe-delta ≥ +0.10).
      Codifies LLM-verdict shape for all future LLM-strategy
      ships.
- [ ] **T-AR-10** — **K1-K10 resolution + decomp.md write**.
      Pin resolutions for K1 (T-AR-3 store snapshot flag) + K2
      (T-AR-4 bench), K4 (T-AR-5 3-back-to-back), K5 (T-AR-2
      check-in compressed cache), K6 (T-AR-9 ADR ≤2 new
      priorities cap), K7 (R9.3 runtime gate confirmed), K8
      (architect refines cost estimate from analyst-strawman),
      K9 (resolved by 2026-05-22 promotion), K10 (T-OD9 default
      C5-standalone-regardless). Write to
      `spec/v3-llm-forecaster/decomp.md` + tick frontmatter
      `owner: analyst → architect`.

## M-DEV — Developer waves (OPENS at M-T1 close)

Developer waves A-G enumerated per architect M-T1 decomp.md.
Analyst-bridge provides wave-level T-D-N stubs (architect M-T1
refines into ordered, sized rows with file:line citations + dep
graphs). Expected: ~30-40 T-D-N rows total across 7 waves; ~3-5
weeks wall-clock per H5.

### Wave A — Foundation (`LlmForecaster` trait + payload)

> Sequential, ~1-3 days. Foundation for Waves B-G.

- [ ] **T-D-N(A1)** — Create `crates/strategy/src/llm_forecaster/`
      module + `trait_def.rs`. Define `LlmForecaster: Send + Sync
      + 'static` async trait with `name(&self)` + `forecast(&self,
      ctx: ForecastContext) -> Result<LlmForecast,
      LlmForecasterError>` signature (R1.1).
- [ ] **T-D-N(A2)** — Define `LlmForecast` payload (R1.2) +
      `LlmForecasterError` enum (R1.4) + `Rating` /
      `Confidence` / `Horizon` / `LessonCardRef` /
      `CostEventRef` value types.
- [ ] **T-D-N(A3)** — Define `ForecastContext` payload (R2.1):
      symbol + now + recent_bars + indicators + top_k_lessons +
      recent_decisions + correlation_id. Implement
      `ForecastContext::from_runtime(symbol, now, runtime)`
      deterministic builder.
- [ ] **T-D-N(A4)** — Implement `ForecastContext::request_hash()`
      canonical SHA-256 over the prompt body (R6.6) — serde_json
      with sorted keys per architect M-T1 lock.
- [ ] **T-D-N(A5)** — Unit tests: `Rating::to_signal_overlay()`
      round-trip; `LlmForecast` serde round-trip; deterministic
      `ForecastContext::from_runtime` (architect M-T1 acceptance
      gate for Wave A close).

### Wave B — Impl over LlmProvider + prompt + schema

> Sequential after Wave A, ~3-7 days.

- [ ] **T-D-N(B1)** — `crates/strategy/src/llm_forecaster/anthropic_impl.rs`
      — `LlmForecasterImpl` struct over `Arc<dyn llm::LlmProvider>`
      + `Arc<dyn reflection::ReflectionStore>` +
      `Arc<llm::CachedSystemPromptBuilder>` + `llm::ToolSchema` +
      `LlmForecasterConfig` (R3.1).
- [ ] **T-D-N(B2)** — System-prompt composition via
      `CachedSystemPromptBuilder` — 2 cache breakpoints (project
      ~800 tokens, role ~1200 tokens) + per-call dynamic block
      (R3.2; architect M-T1 lock on JSON vs markdown).
- [ ] **T-D-N(B3)** — `ToolSchema::propose_forecast` definition +
      JSON-schema validation via `llm::tools::ToolSchema` (R3.3).
- [ ] **T-D-N(B4)** — `temperature = Some(0.0)` pin (R3.4); seed
      pin where supported (OpenAI; Anthropic NA). Config constant.
- [ ] **T-D-N(B5)** — Unit tests: composed `ChatRequest` has
      exactly 2 cache breakpoints; schema validates known-good +
      rejects known-bad; `wiremock` integration test mocks
      Anthropic `propose_forecast` tool-use response → round-trips
      through `LlmForecast` decode.

### Wave C — Strategy registry + Signal mapping

> Parallel-safe with Wave D, depends on Wave B, ~2-4 days.

- [ ] **T-D-N(C1)** — `crates/strategy/src/llm_forecaster/strategy.rs`
      — `LlmForecasterStrategy: Strategy` impl emitting Signal
      per bar derived from `LlmForecaster::forecast()` (R4.1).
- [ ] **T-D-N(C2)** — Registry entry in
      `crates/strategy/src/registry.rs` — name
      `"llm_forecaster_v3"`; opt-in via
      `config/agent.toml [[strategies]] kind = "llm_forecaster_v3"`.
- [ ] **T-D-N(C3)** — Signal carry-forward between fire ticks
      (R5.4 — fire every N bars; default N=24); strategy state
      holds the last `LlmForecast`.
- [ ] **T-D-N(C4)** — Unit tests: strategy fires exactly 1
      LLM call per 24-bar window; carry-forward signal between
      fires.

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

- [ ] **T-D-N(E1)** — Additive migration
      `crates/audit/migrations/011_llm_forecast.sql` (architect
      M-T1 owns the row shape) — `JournalEntry { kind:
      "llm_forecast", payload: ... }` (R7.1.2).
- [ ] **T-D-N(E2)** — `CostEvent::Llm` row emission via
      `BudgetedProvider` (R7.1.1 — already wired in v2-llm-strategy;
      this confirms C5's call site emits).
- [ ] **T-D-N(E3)** — `Message::AuditTick` ride-along emission
      (R7.1.3) — Phase D + F audit-tick stream consumers see
      the row live.
- [ ] **T-D-N(E4)** — `BudgetedProvider` 80% auto-degrade +
      100% block (R5.5; inherits v2-llm-strategy R4.1+R4.4).
- [ ] **T-D-N(E5)** — `cost_cap_usd_per_backtest` enforcement
      (R5.3) — `BudgetExceeded` short-circuits backtest binary
      with explicit error log.
- [ ] **T-D-N(E6)** — Integration test: 1 `forecast()` call →
      exactly 1 `CostEvent` row + 1 `JournalEntry` row + 1
      `AuditTick` broadcast.

### Wave F — Phase F Assistant slot body promotion

> Parallel-safe with Wave E, depends on Wave C, ~3-5 days.
> **GATED ON Q4=(c)** operator decision at T-OD4. If Q4 = (a)
> only, this entire Wave F is deferred to v0.1.1.

- [ ] **T-D-N(F1)** — `crates/ui/src/assistant/view.rs` body
      promotion — add `AssistantMode { Offline, ReasoningTrace }`
      enum + body composition logic (R9.1).
- [ ] **T-D-N(F2)** — Body composition (R9.2) — header
      (`symbol · rating · conf=…`), cost line, reasoning trace
      card, cited lesson cards section (reuses
      `crates/ui/src/memory/` components), scrollable history,
      chevron → `Message::OpenTrailFor(audit_id)`.
- [ ] **T-D-N(F3)** — `Message::AssistantReasoningTraceUpdate(
      payload)` cockpit-message-bus variant wiring.
- [ ] **T-D-N(F4)** — Runtime gate (R9.3) — strategy-enabled
      flag flips `AssistantMode`; default-disabled config keeps
      Phase F placeholder + byte-identity guard.
- [ ] **T-D-N(F5)** — Snapshot baselines —
      `assistant_slot__llm_forecaster_active__most_recent_trace`
      (Q4=c active) + `assistant_slot__llm_forecaster_disabled__placeholder`
      (R9.3 byte-identity).
- [ ] **T-D-N(F6)** — Layout-invariants proptest:
      `assistant_slot_llm_forecaster_no_zero_dim` (256 cases).

### Wave G — ADR + non-regression + tester handoff (serial closure)

> Depends on Waves A-F, ~2-3 days.

- [ ] **T-D-N(G1)** — Commit ADR
      `spec/architecture/adr/0038-llm-forecaster-verdict-shape.md`
      (or renumber per T-OD6 open — coordinate with architect
      T-AR-9 final pick) with L0-L4 priority tree.
- [ ] **T-D-N(G2)** — `crates/strategy/tests/llm_forecaster_neutrality.rs`
      — re-runs `top10-2023-fy-tcn-overlay-realdata` and asserts
      body-SHA `8fa47f49…` unchanged after registry add (R10.2).
- [ ] **T-D-N(G3)** — Non-regression sweep —
      `scripts/verify_anchors.sh` → 36 / 36 PASS
      (34 existing + 2 new at `v3.0.0-llm-forecaster`).
- [ ] **T-D-N(G4)** — `spec-lint` contribution = 0 (R10.6).
- [ ] **T-D-N(G5)** — Frontmatter flip + tester handoff envelope
      per AGENT.md § Communication contract.

## M-FINAL — Tester sweep (OPENS at M-DEV close)

- [ ] **T-T1** — `cargo fmt --check` + `cargo clippy --workspace
      -- -D warnings` exit 0.
- [ ] **T-T2** — `cargo test --workspace --lib` 100% PASS.
- [ ] **T-T3** — `cargo test --workspace --features candle,realdata`
      100% PASS (incl. `llm_forecaster_neutrality.rs` from G2).
- [ ] **T-T4** — Snapshot baselines:
      - `assistant_slot__llm_forecaster_active__most_recent_trace`
        (Q4=c body promotion — gated on T-OD4).
      - `assistant_slot__llm_forecaster_disabled__placeholder`
        (R9.3 byte-identity guard).
- [ ] **T-T5** — `scripts/verify_anchors.sh` → **36 / 36** PASS
      (34 existing + 2 new under `v3.0.0-llm-forecaster`).
      Non-negotiable per R10.1.
- [ ] **T-T6** — `cockpit-smoke` → 0 panic lines on
      `llm_forecaster_v3` enabled config (R10.3).
- [ ] **T-T7** — H4 byte-identity test: backtest re-run 3 times
      produces identical SHA each time. Non-negotiable (anchor
      pre-condition per T-AR-5 K4 mitigation).
- [ ] **T-T8** — H2 cost benchmark recorded in test report —
      actual $ from `llm-forecaster-bench` projected to full-year.
- [ ] **T-T9** — H1 Sharpe-delta verdict per ADR-0038 L0-L4
      priorities — explicit verdict cell (L0 PASS / L1-L4 fail).
- [ ] **T-T10** — Author `spec/v3-llm-forecaster/reports/test-final-<YYYY-MM-DD>.md`
      using `rust-test` skill template; cite L-verdict + Sharpe-
      delta + cost-USD-actual + 3-run byte-identity SHA trio.
      VERDICT cell: PASS / REGRESSION / EQUIVALENT.

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
