---
slug: v3-llm-forecaster
status: draft
owner: analyst
updated: 2026-05-22
---

# v3-llm-forecaster — tasks

> **Spec-only design exploration.** Per operator Q-SEQ HYBRID
> (2026-05-22), architect M-T1 + developer waves are DEFERRED
> until either (i) C1 (`v3-volatility-forecasting`) ships its
> verdict and operator promotes this slug, or (ii) operator
> explicitly promotes ahead of C1. Only the analyst T-A* lane
> ticks at this pass; M-T1 / M-DEV / M-FINAL / M-PRESENTER milestones
> below are stubbed for the architect/developer/tester to ratify
> at promotion time.

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

## M-OD — Operator-decide (DEFERRED until promotion)

Operator answers Q1-Q8 + decides Q-PROMOTE (promote Queue → Active
now vs after C1 ships). Tasks below are markers; orchestrator
toggles when operator routes.

- [ ] **T-OD1** — Operator answers Q1-Q8 (8 questions; defaults
      analyst-strawmanned).
- [ ] **T-OD2** — Operator decides Q-PROMOTE — promote now vs
      after C1 ships its verdict. Default per Q-SEQ HYBRID is
      "after C1".
- [ ] **T-OD3** — Operator decides Q-V2X-SEQ —
      `v2x-trading-state-bus` ordering vs C5. Default is "C5
      standalone v0.1.0; v2x-bus refactor v0.1.1 if both ship".
- [ ] **T-OD4** — Operator decides Q-ASSISTANT-WAKE — Phase F
      Assistant slot promotion default (runtime-gated, safe) vs
      unconditional (less safe). Default is runtime-gated per R9.3.

## M-T1 — Architect decomposition (DEFERRED until promotion)

Architect ratifies feature.md into `decomp.md` + ADR-0038 +
ordered Wave plan. Tasks below are stubs the architect refines.

- [ ] **T-T1.1** — Resolve K1 — reflection-store snapshot flag for
      backtest determinism. Decide: backtest binary takes
      `--reflection-store-snapshot <path>` (analyst-recommended).
- [ ] **T-T1.2** — Resolve K2 — LLM cost benchmark via
      `cargo run --bin llm-forecaster-bench`. Calibrate R5.4 N-bar
      batching default; document in `decomp.md`.
- [ ] **T-T1.3** — Resolve K4 — Anthropic drift policy. Analyst-
      recommended: anchor only after 3 back-to-back identical
      cache-build runs.
- [ ] **T-T1.4** — Resolve K5 — replay-cache checkout strategy.
      Analyst-recommended: check-in compressed cache
      (`crates/strategy/tests/fixtures/llm-forecaster-replay.db.gz`).
- [ ] **T-T1.5** — Draft ADR-0038 "LLM-forecaster verdict criteria
      L1-L4" per Q6=(b). Priorities: L1 bias collapse / L2
      calibration failure / L3 cost overrun / L4 reasoning trace
      degenerate / L0 PASS.
- [ ] **T-T1.6** — Resolve Q8 sequencing under operator's
      promotion decision. Default standalone v0.1.0; lifts to
      `TradingState` substrate if operator promotes
      `v2x-trading-state-bus` first.
- [ ] **T-T1.7** — Decompose R1-R10 into ordered T-D-N tasks per
      wave. Suggested wave map (subject to architect refinement):
      Wave A trait + payload (R1+R2); Wave B impl + prompt (R3);
      Wave C strategy + registry (R4.1); Wave D backtest +
      replay-cache (R8+R6); Wave E audit + cost (R7+R5); Wave F
      Phase F Assistant slot promotion (R9); Wave G ADR-0038 +
      non-regression + tester handoff.
- [ ] **T-T1.8** — Confirm net-new file count + crates touched.
      Initial estimate: `crates/strategy/src/llm_forecaster/` new
      subfolder (4-6 files); `crates/audit/migrations/011_llm_forecast.sql`
      additive; `crates/ui/src/assistant/view.rs` body
      promotion; `crates/forecast/src/bin/llm_forecaster_bench.rs`
      new; tests across strategy + audit + ui.
- [ ] **T-T1.9** — Bench H4 byte-identity falsification protocol
      (architect-level micro-bench: prompt-hash canonicalisation
      across 2 re-runs).
- [ ] **T-T1.10** — Bench H2 cost falsification protocol
      (architect-level token-count via Anthropic `count_tokens`
      endpoint on a 1-month slice; project to full-year).

## M-DEV — Developer waves (DEFERRED until M-T1 completes)

Developer waves A-G ratified by architect M-T1. Task numbering
T-D-N1..N{n} authored at architect M-T1 time.

- [ ] **T-D-N1..N{n}** — Architect M-T1 to enumerate per the
      Wave A-G map. Sketch:
      - Wave A — `LlmForecaster` trait (`trait_def.rs`) +
        `LlmForecast` payload + `ForecastContext`.
      - Wave B — `LlmForecasterImpl` over `Arc<dyn LlmProvider>` +
        prompt-builder wiring + tool-use schema + temperature pin.
      - Wave C — `LlmForecasterStrategy: Strategy` + registry
        entry (`crates/strategy/src/registry.rs`).
      - Wave D — Backtest scenarios
        (`top10-2023-fy-llm-forecaster-realdata` + 2024) + replay-
        cache wiring via `crates/llm::RecordingProvider/ReplayProvider`.
      - Wave E — Audit-ledger emission (additive migration 011) +
        `BudgetedProvider` wiring + cost-event integration.
      - Wave F — Phase F Assistant slot body promotion (gated by
        runtime config; R9.3 byte-identity guard for disabled state).
      - Wave G — ADR-0038 commit + non-regression tests +
        `crates/strategy/tests/llm_forecaster_neutrality.rs`
        (re-runs `top10-2023-fy-tcn-overlay-realdata` and asserts
        8fa47f49… unchanged) + tester handoff envelope.

## M-FINAL — Tester sweep (DEFERRED until M-DEV completes)

- [ ] **T-F1** — `cargo fmt --check` + `cargo clippy --workspace
      -- -D warnings` exit 0.
- [ ] **T-F2** — `cargo test --workspace --lib` 100% PASS.
- [ ] **T-F3** — Snapshot baselines:
      `assistant_slot__llm_forecaster_active__most_recent_trace` +
      `assistant_slot__llm_forecaster_disabled__placeholder` (the
      byte-identity guard).
- [ ] **T-F4** — `scripts/verify_anchors.sh` → 32 / 32 PASS (30
      existing + 2 new under `v3.0.0-llm-forecaster`). Non-
      negotiable per R10.1.
- [ ] **T-F5** — `cockpit-smoke` → 0 panic lines on
      `llm_forecaster_v3` enabled config (R10.3).
- [ ] **T-F6** — H4 byte-identity test: backtest re-run produces
      identical SHA. Non-negotiable (anchor pre-condition).
- [ ] **T-F7** — H2 cost benchmark recorded in test report.
- [ ] **T-F8** — H1 Sharpe-delta verdict per ADR-0038 L0-L4
      priorities.
- [ ] **T-F9** — Author `spec/v3-llm-forecaster/reports/test-final-<YYYY-MM-DD>.md`.

## M-PRESENTER — Operator approval (DEFERRED until M-FINAL PASS)

- [ ] **T-P1** — Presenter deck enumerates H1-H5 falsification
      results.
- [ ] **T-P2** — Presenter renders 10-20 sample reasoning traces
      from the Phase F Assistant slot for operator H3 trust-
      judgment.
- [ ] **T-P3** — Presenter renders Sharpe-delta + ADR-0038 verdict
      (L0 PASS / L1-L4 fail).
- [ ] **T-P4** — Operator-approval routes: (a) PASS — ship; promote
      to paper-trading stage per product.md § Strategy lifecycle;
      (b) HOLD — investigate L1-L4 verdict; (c) F-equivalent —
      retire; preserve spec as what-not-to-chase reference.

## Wave parallelism map (architect M-T1 owns; analyst stub)

Within developer impl (M-DEV) waves, the architect M-T1 ratifies
parallelism:

- **Wave A** (`LlmForecaster` trait + payload + `ForecastContext`)
  serial (foundation).
- **Wave B** (`LlmForecasterImpl` + prompt-builder + schema)
  serial; depends on Wave A.
- **Wave C** (`LlmForecasterStrategy` + registry) parallel with
  **Wave D** (backtest scenarios + replay-cache wiring) — both
  depend on Wave B.
- **Wave E** (audit + cost) parallel with **Wave F** (Phase F
  Assistant slot) — both depend on Wave C and have no shared edit
  surface.
- **Wave G** (ADR-0038 + non-regression + tester handoff) serial
  closure; depends on all of A-F.

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
