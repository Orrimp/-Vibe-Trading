---
slug: backlog
status: living
owner: orchestrator
updated: 2026-06-13
---
<!-- updated 2026-05-29 (analyst, pick-c-orchestrator-hygiene-compounder-trio M0 close) —
     promoted Queue → Active THREE features under Pick C Wave 1 of the
     architect's process-tooling survey at
     `spec/dev-notes/process-tooling-survey-2026-05-29.md` § Pick C
     (Top-5 Rank 5): `queue-staleness-reconciliation` (~1 dev day;
     scripts/queue_staleness_check.py orchestrator pre-flight
     catching status-mismatch drift class surfaced 3× in 3 weeks) +
     `adr-registry-atomic-lint` (~0.5 dev day; scripts/adr_registry_check.py
     pre-commit hook enforcing the 2026-05-29 codified architect.md
     atomic-write contract on every commit touching
     spec/architecture/adr/) + `operator-ledger-schema-lint` (~0.5
     dev day; scripts/operator_ledger_check.py upgrading the
     2026-05-29-created operator-side-pending-ledger.md from
     convention to schema-enforced living document with stale-FAILED
     escalation at 7-day threshold). Strategic direction at
     `spec/dev-notes/pick-c-orchestrator-hygiene-2026-05-29.md`
     frames the trio (durable-over-quick per AGENT.md 2026-05-28).
     Cheapest of architect's Month-1 picks (~2 dev days combined vs
     Pick A trifecta ~5-7d + Pick B duo ~2d). Operationalises
     retro fix-improve #1 (Queue staleness) + #6 (ADR registry
     drift) + #5 (operator pending ledger). All three are
     Python-stdlib-only scripts under `scripts/` — zero Cargo touch,
     zero new external deps, zero anchor delta. One bundle-level
     operator-decide Q-HYG-EMIT (Recommended DURABLE = markdown
     table + per-violation context lines) locked shared diff
     dialect across all three scripts. K4 amendment ownership-
     table: each feature OWNs a specific AGENT.md / architect.md /
     ledger-frontmatter section to prevent cross-feature contract
     drift. All per-feature Qs bias DURABLE per AGENT.md 2026-05-28.
     Three new trace rows opened proposed:
     `REQ-QUEUE-STALENESS-RECONCILIATION-001` +
     `REQ-ADR-REGISTRY-ATOMIC-LINT-001` +
     `REQ-OPERATOR-LEDGER-SCHEMA-LINT-001`. No new ADRs. PARALLEL-
     SAFE with all in-flight agents (Bug #64 dev, v5 + v2.1
     presenters, v5 cleanup dev) per the bundle direction
     § Sequencing conflict matrix — disjoint file scopes throughout. -->

<!-- updated 2026-05-29 (analyst, pick-b-cross-cutting-safety-duo M0 close) —
     promoted Queue → Active TWO features under Pick B Wave 1 of the
     architect's process-tooling survey at
     `spec/dev-notes/process-tooling-survey-2026-05-29.md` § Pick B
     (Top-5 Rank 3 + Rank 4): `v2-1-tracing-layer-redactor` (~1.5
     dev days; cross-cutting safety net at audit/llm boundary) +
     `ui-contrast-asserter` (~0.5 dev days; WCAG 2.1 (fg, bg) pair
     assertion). Strategic direction at
     `spec/dev-notes/pick-b-cross-cutting-safety-duo-2026-05-29.md`
     frames the bundle (durable-over-quick per AGENT.md 2026-05-28).
     v2.1-tracing-Layer-redactor portion SPLIT OFF from the
     `v2-llm-strategy-v21-followups` Queue entry (#3); LLM-budget
     tile + clippy items stay Queue per process-tooling-survey
     § What's NOT a compounder (defer with v2 LLM lane activation).
     Two new trace rows opened proposed:
     `REQ-V2-1-TRACING-LAYER-REDACTOR-001` +
     `REQ-UI-CONTRAST-ASSERTER-001`. Both biased DURABLE on all
     operator-decide Qs. One bundle-level operator-decide
     Q-DUO-WARN (Recommended DURABLE = 2-week WARN per feature
     before v0.2.0 gate flip) surfaced in the strategic dev-note. -->

<!-- updated 2026-05-22 (orchestrator, audit-2026-05-22 P2.5 cleanup) —
     v25-tcn-alpha-investigation shipped 2026-05-19; v25-tcn-overlay
     parent flipped shipped 2026-05-22 (F4 disposition); v25a-patchtst-overlay
     shipped 2026-05-22 + entire 4-phase DL roadmap retired per operator
     routing (a). Stale Active rows reflecting in-flight state removed;
     historical HTML-comment changelog entries left in place as
     archeology. -->

<!-- updated 2026-05-21 (analyst, v25a-patchtst-overlay activation pass) —
     see Active section for the v25a-patchtst-overlay v0.1.0 brief
     activated by operator's Q1=(b) RETIRE v2.5 TCN decision at
     v25-tcn-horizon-bump-or-retire M-OD 2026-05-21. [SUPERSEDED 2026-05-22
     — v25a shipped + DL roadmap retired] -->

<!-- updated 2026-05-20 (analyst, ui-rethink-phase-f-memory-models-assistant M0
     close) — Analyst pass landed for the sixth and final phase of the UI
     rethink. Brief at
     `spec/ui-rethink-phase-f-memory-models-assistant/feature.md` (status:
     draft, owner: analyst, version: 0.1.0, predecessor:
     `ui-rethink-phase-e-compare v0.1.0`) carries R1-R8 + Q1-Q8 + K1-K8 +
     H1-H6 + 8-item non-regression contract. Three independently-shippable
     surfaces: (i) `screens::memory` over `crates/reflection` lesson-cards
     store (J7); (ii) `screens::models` over `crates/forecast/checkpoints/anchors/`
     (J8 — BS-1 + BS-2 TCN checkpoints confirmed on disk at 2026-05-20);
     (iii) right-rail Lumen Phase 6 Assistant slot wake (v2-llm-strategy
     v2.0.0 shipped 2026-05-13 — wake condition met). **Eight operator-
     decide Qs surfaced**, all with analyst-recommended defaults; analyst-
     recommended Q4 = a (stub-only Assistant slot wake — light the slot
     structurally without scope-creeping into LLM plumbing; v0.2.0 wires
     the body). K4 (Assistant-slot + Memory-drawer right-side coexistence)
     + K6 (RIGHT_RAIL_WIDTH_PX constant semantic change) flagged as the
     load-bearing cross-feature traps; both with analyst-recommended
     fallback paths (K4 → Q5=c route fallback; K6 → Option A
     RIGHT_RAIL_OPEN_WIDTH_PX additive constant). Anchor risk zero by
     construction (no backtest binary changes, no anchored renderer touch,
     no audit writer touch, no reflection writer touch). Predecessors
     audited: Phase C sidebar IA reserves both `Screen::Memory` +
     `Screen::Models` (theme.rs:741-750); shell.rs:98-99 placeholder routes;
     shell.rs:47-49 right-rail reservation. Trace row
     `REQ-UI-RETHINK-PHASE-F-001` opened draft. Cost estimate: ~3-4 weeks
     (~4-5 weeks if Q4=(b) full v2 LLM wire chosen). HANDOFF → operator-
     decide → architect for M-T1. Per dev-note §6 line 1110 ("Final sweep
     — anything missing?") Phase F closes the rethink unless operator
     review surfaces a J9+ gap. -->
<!-- updated 2026-05-20 (analyst, ui-rethink-phase-e-compare M0 close) —
     Analyst pass landed for the next phase of the UI rethink. Brief at
     `spec/ui-rethink-phase-e-compare/feature.md` (status: draft, owner:
     analyst, version: 0.1.0, predecessor:
     `ui-rethink-phase-d-trail-followup v0.1.1`) carries R1-R8 + Q1-Q8 +
     K1-K8 + H1-H5 + 8-item non-regression contract. Read-only matrix
     surface: `screens::compare` + `widgets::matrix` over the existing
     report-cache (`spec/<strategy>/reports/` frontmatter); cell-click
     → Lab seeded via new `Message::OpenLabFromCompare` (mirrors Phase C
     `OpenStrategyInLab` and Phase D `OpenTrailFor` compound-dispatch
     precedents). **Eight operator-decide Qs surfaced**, all with
     analyst-recommended defaults; analyst-recommended Q2 = c
     (report-cache only with manual recompute via Lab — no new
     orchestration at v0.1.0; the dev-note §1154 background-cadence
     resolution applies only IF/WHEN orchestration ships, which we defer
     to v0.2.0). **Anchor risk zero by construction** — no backtest
     binary changes, no anchored renderer touch; sidebar entry already
     reserved by Phase C IA (`SIDEBAR_GROUPS_PHASE_C` Work zone). Cost
     ~2-3 weeks per dev-note §6 line 1096; no cliffs; independently
     shippable. Trace row `REQ-UI-RETHINK-PHASE-E-001` opened in
     `draft`. K6 (Compare/Lab range divergence) + K7 (universe-aggregate
     semantic confusion) surfaced as load-bearing UX traps. Promoted
     Queue/UI (implicit — Phase E was next per dev-note ordering
     A→B→C→D→E→F) → Active. HANDOFF → operator-decide (Q1-Q8) →
     architect for M-T1 decomposition. -->
<!-- updated 2026-05-20 (orchestrator, chart-x-axis-local-time ship) —
     `chart-x-axis-local-time v1.11.0` operator-approved via "Autoapprove
     all" directive (overnight session ship). Moved Queue/UI → Recent.
     Trivial direct ship per CLAUDE.md — no analyst/architect cycle: 1-line
     Cargo.toml flip + ~10 LOC body swap + 1 unit test + env-var override
     in 2 integration test runners. 22/22 anchors byte-identical; 279
     workspace tests PASS; cockpit-smoke 0 panics; spec-lint contribution
     0. Closes the v1.10.0 Q-revised-1 deferral; chart bottom axis now
     renders in operator's local time zone in production. -->
<!-- updated 2026-05-19 (orchestrator, ui-rethink-phase-b-lab-run ship) —
     `ui-rethink-phase-b-lab-run v0.2.0` operator-approved via "Autoapprove
     all" directive against presenter deck (VERDICT → READY at agentId
     a63f66d04292d069c). Moved Active → Recent. 22/22 anchors byte-
     identical; main.rs collapsed 3417 → 1447 LOC; engine::run_scenario
     dispatches via mapping layer; new widgets::run_delta_badge widget
     + LabState.last_run_report/prev_run_report rotation; ADR-0035
     (scenario-dispatch extraction pattern) landed. Tester re-gate PASS
     at aabb761c2039c855e after mechanical fmt+clippy cleanup at
     a09f2e3a1a02d18de (77 → 0 clippy errors). Known deviation: cancel
     uses wrap-and-abort, bar-level deferred to Phase C. Unblocks the
     operator's J2 workflow end-to-end. -->
<!-- updated 2026-05-19 (analyst, ui-rethink-phase-b-lab-run M0 close) —
     Analyst pass landed. feature.md (status: draft, owner: analyst,
     version: 0.1.0) carries the full R1-R10 + K1-K8 + H1-H5 + Q1-Q5
     registers. Critical architecture finding: `crates/backtest` is
     already library-callable at the type-surface level (`lib.rs`
     re-exports `run_scenario`, `RunReport`, `ScenarioConfig`,
     `DateRange`, `ParamSheet`, `BacktestKpis`, `MatchingEngine`,
     `RunError`) — Phase B work is BODY extraction from `main.rs`
     (3417 LOC, 7 scenarios, 4 backtest paths), not API extraction.
     `engine::run_scenario` body is currently a stub
     (`engine.rs:236-240` returns `Err(RunError::NotImplemented)`).
     All 22 anchors preserved by construction (H2 is the gate).
     Cancellation pattern mirrors Phase A's mpsc-disconnect
     (`runner.rs:108-111`), NOT trainer's subprocess SIGKILL. Q1-Q5
     defaults locked: Q1=A in-memory return, Q2=A ThrottledSpinner
     only, Q3=A disabled-while-running + internal cancel poll,
     Q4=A session-local diff, Q5=A preserve all 22 anchors.
     HANDOFF → operator-decide on Q1-Q5, then → architect for
     M-T1 decomposition. -->
<!-- updated 2026-05-19 (orchestrator, ui-rethink-phase-b-lab-run promotion) —
     Phase B of the UI rethink promoted Queue (implicit) → Active.
     Operator direction "1 then 2" — #1 (cockpit-performance-and-input-
     responsiveness) was already shipped 2026-05-15 (stale backlog
     entry corrected in same sweep, moved Queue → Recent); #2 (Phase B)
     stub authored at `spec/ui-rethink-phase-b-lab-run/feature.md`
     (status: proposed, owner: pending-analyst, version: 0.1.0,
     predecessor: ui-rethink-phase-a-lab v0.2.0). 5 analyst-surfaced Qs
     (Q1 library-call shape; Q2 spinner UX; Q3 cancel semantics; Q4
     compare target; Q5 anchor surface). Trace row
     REQ-UI-RETHINK-PHASE-B-001 opened in proposed. HANDOFF → analyst. -->
<!-- updated 2026-05-19 (orchestrator, cockpit-training-control ship) —
     `cockpit-training-control v0.2.0` operator-approved via "Autoapprove
     all" directive (presenter deck + 3 manual `[orchestrator]` acceptance
     rows cleared in one tick). Moved Active → Recent. 22/22 anchors
     byte-identical; cockpit-smoke 0 panics; own spec-lint contribution 0.
     Predecessor: `ui-rethink-phase-a-lab v0.2.0`. Tier 1 (Lab Train
     sub-panel + subprocess + log tail + cancel) + Tier 2 (additive SQLite
     migration 010 `training_events` + opt-in `--audit-db` flag + 1-Hz
     subscription + loss-curve plot + axis primitive + orphan-detect) all
     shipped. Unblocks the v2.5 retraining cycle that the
     v25-tcn-alpha-investigation F4 verdict triggers. -->
<!-- updated 2026-05-19 (analyst, cockpit-training-control) — analyst pass
     landed. Brief at `spec/cockpit-training-control/feature.md` (status:
     draft, owner: analyst, version: 0.1.0, predecessor:
     `ui-rethink-phase-a-lab v0.2.0`). Tasks skeleton at
     `spec/cockpit-training-control/tasks.md` with M0 (architect) →
     M-T1 (Tier 1 — launch button + log tail) → M-T2 (Tier 2 — audit
     train_events + live loss curves) → M-FINAL (tester). Per-task
     `T-D-N` decomposition deferred to architect (M0). R1-R10 lock the
     defaults: Lab sub-panel (collapsible, bottom of Lab column, R1);
     new `lab::trainer::spawn_training_run` mirroring `lab::runner`
     cancellation-handle shape but spawning via tokio::process::Command
     (R2); 200-line ring-buffer log widget (R3); NEW `training_events`
     SQLite table via additive migration 010 (R4); opt-in `--audit-db`
     flag on `train_tcn` — default omitted so existing CI / manual
     runs stay byte-identical (R5); REUSE chart widget for loss-curve
     plot at fixed 160-px panel-internal slot (R6); NEW 1-Hz audit-DB
     poller iced::Subscription recipe (R7); cold-start: only
     `training_panel_collapsed` survives, the audit DB IS the training-
     history persistence (R8); R9 error surface per failure mode;
     R10 non-regression contract — 19 anchors byte-identical, zero new
     anchors (training is non-replayable due to wall-clock + UUID
     inputs). 4 operator-decide Qs surfaced: Q1 (new training_events
     table vs. extend strategy_events — analyst-recommended **new
     table**), Q2 (SIGKILL-immediate vs. SIGTERM-graceful 30s on
     Cancel — analyst-recommended **SIGKILL**), Q3 (panel
     hyperparameter editing — analyst-recommended **no, defer to
     follow-on**), Q4 (auto-focus Train panel on orphan-detect —
     analyst-recommended **no, status-strip annotation only**). K1-K6
     risk register; ZERO backtest scenarios; ZERO new anchors. Trace
     row `REQ-COCKPIT-TRAIN-001` opened in proposed state. HANDOFF →
     operator-decide (4 Qs) → architect. -->
<!-- updated 2026-05-19 (orchestrator, cockpit-training-control queue) —
     Operator queued `cockpit-training-control` as the next task while the
     v25-tcn-alpha-investigation BS-1 determinism re-run was finishing.
     Scope locked at spawn: Tier 1 (launch button + log tail, ~1-2 days)
     + Tier 2 (audit train_event stream + live loss-curve plot, ~1 wk
     additional). Explicit anti-recommendation against Tier 3 (in-process
     training). Total estimate ~2 wk. Analyst spawn in flight, parallel
     with alpha-investigation. Will pair naturally with the v2.5
     retraining follow-on the alpha-investigation's F4 verdict triggers. -->
<!-- updated 2026-05-18 (analyst, v25-tcn-alpha-investigation) — analyst
     pass landed for the just-promoted `v25-tcn-alpha-investigation`
     feature. Brief at `spec/v25-tcn-alpha-investigation/feature.md`
     (status: draft, owner: analyst, predecessor:
     `backtest-real-binance-data v0.1.0`, parent: `v25-tcn-overlay
     v2.5.0 (in-progress)`). Read-only investigation: forensic look at
     why the BS-1 / BS-2 TCN checkpoints emit `r_hat` inside the
     ε=0.0005 deadband on REAL Binance OHLCV (not just synthetic GBM),
     producing `dampened=0` across all four v2.6.0-realdata anchor
     scenarios. R1-R6 lock the deterministic-analysis bits: histogram
     report family (R1) at `forecast-distribution-bs{1,2}-realdata`,
     ≤3 new anchors under version `v2.6.0-alpha-investigation` (R2),
     a new read-only `forecast_distribution` bin under
     `crates/forecast/src/bin/` (R3, architect may relocate),
     deterministic F1/F2/F3/F4 failure-mode taxonomy (R4) that maps
     each verdict to a named follow-on feature, R5 Sharpe-comparison
     table over the four `-realdata` anchors (closes bucket (d) of
     the original 4-bucket framing), R6 anchor-neutrality contract
     (19 originals stay byte-identical → 21 or 22 at ship). ONE
     operator-decide Q: scope (minimal / diagnostic / full).
     **Analyst-recommended: MINIMAL** (buckets (a) + (d) only, no
     re-training, no checkpoint-internal inspection). Reason: bucket
     (a)'s histogram cheaply distinguishes whether (b) or (c) are
     even worth funding before paying for them. Promoted Queue
     /Strategy → Active. Trace row `REQ-V25-TCN-ALPHA-001` opened
     in proposed state. HANDOFF → operator-decide (1 Q) → architect. -->
<!-- updated 2026-05-18 (analyst, backtest-real-binance-data) — analyst
     pass landed for the just-promoted `backtest-real-binance-data`
     feature (Active row above). Brief at
     `spec/backtest-real-binance-data/feature.md` carries R1-R10
     closing Q1-Q8 with strong analyst defaults; tasks skeleton at
     `spec/backtest-real-binance-data/tasks.md` is M0-M-FINAL with
     per-task `T-D-N` decomposition deferred to architect (T-AR-2).
     Recommended direction: ADD a parallel `-realdata` scenario
     family rather than replacing synthetic in-place (preserves the
     15 existing anchors as a CI-on-empty-disk floor; new scenarios
     anchor under version `v2.6.0-realdata`). Determinism contract
     pinned via a new `data/binance/REVISION.toml` per-file-SHA
     manifest (R7); data gaps > 0.5% hard-fail (R3); universe pinned
     to the 10 USDT pairs currently on disk (R4); scope wire-only —
     alpha verdict (Sharpe table vs v1 baseline) deferred to a
     follow-on v25-tcn-overlay tester re-spawn (R8). Three operator-
     decide Qs (Q1 in-place-vs-parallel, Q4 universe-snapshot, Q8
     wire-only-vs-combined-alpha) surfaced — analyst-recommended
     defaults documented for all three. Trace row
     `REQ-BACKTEST-REALDATA-001` opened in proposed state. Non-
     regression contract: 15 anchors stay byte-identical, 4 new
     `-realdata` anchors lock at M-FINAL → 19 total at ship.
     HANDOFF → operator-decide (3 Qs) → architect. -->
<!-- updated 2026-05-18 (orchestrator, backtest-real-binance-data ship) —
     `backtest-real-binance-data v0.1.0` operator-approved (commit `664bb59`).
     Moved Active → Recent. Real Binance hourly parquet now flows through the
     backtest harness via a new opt-in `realdata` cargo feature; 4 new anchors
     locked under `v2.6.0-realdata` → 19/19 verify_anchors. 15 originals stay
     byte-identical. Open finding the operator approved with: TCN real-weights
     produces `dampened=0` on real OHLCV too (not just synthetic), so the M3
     alpha-evaluation gap is now diagnosable but unresolved. New
     `v25-tcn-alpha-investigation` queued under Strategy as the next pre-v2.6
     prerequisite (4 investigation buckets: ε/confidence tuning, horizon
     mismatch, training-pathology hypothesis, Sharpe-table author). -->
<!-- updated 2026-05-18 (orchestrator, m3-finish-pass) — TCN M3 shipped
     T-D-11/T-D-12: two LFS checkpoints, two `-weights` scenarios, two
     new anchors under version `v2.5.0-tcn-weights`, 15/15 verify_anchors.
     M3 also surfaced a previously-hidden gap: the backtest harness uses
     synthetic ChaCha20Rng GBM bars, so real-weights output == passthrough
     on synthetic data (dampened=0). New `backtest-real-binance-data`
     proposal queued under Strategy as the next prerequisite for v2.6
     forecast bake-off. Tester gate next, then presenter. -->
<!-- updated 2026-05-18 (orchestrator, operator-approval-pass) — operator
     approved both presenter decks at commit `ef8fb3c`.
     `ui-rethink-phase-a-lab v0.2.0` → SHIPPED (Active → Recent).
     `v25-tcn-overlay v2.5.0` CI-baseline gate closed (passthrough path);
     status stays _in-progress_ — M3 full TCN training (T-D-11/T-D-12) +
     real-weights anchor lock under version `v2.5.0-tcn-weights` is the
     next milestone. Spec-lint normalised at 733/2 (improvement of -1 vs
     audit-2026-05-18 baseline). -->
<!-- updated 2026-05-17 (analyst, ui-rethink-phase-a-lab) — promoted
     `ui-rethink-phase-a-lab` v0.1.0 from dev-note → Active. First
     concrete carve-out of the broader UI rethink at
     `spec/dev-notes/ui-rethink-2026-05-17.md`. Operator-locked
     direction (chart-as-door, XRP-first pair ordering, three overlay
     layers, read-only at Phase A) ratified in the dev-note addendum
     2026-05-17. Brief authored: `Charts` → `Lab` rename + default-
     route flip, buy/sell markers (already shipped at v1.9.0) wired
     into Lab, new equity-curve overlay (cached reports), new
     ≤4-strategy comparison overlay, pair-chip / strategy-chip /
     date-range widgets, Lab tuple persistence. Eleven requirements
     (R1–R11); zero new Lumen tokens; zero touched anchors. Three
     operator-decide questions surfaced (Q-A1 palette / Q-A2
     cached-only / Q-A3 cold-start tuple) — each with a default the
     developer can ship against. Trace row `REQ-UI-RETHINK-PHASE-A-001`
     created in proposed state. HANDOFF → architect. -->
<!-- updated 2026-05-17 (analyst, v25-tcn-overlay) — phase 1 of 4 DL
     roadmap. Analyst pass landed on `v25-tcn-overlay`: Q1-Q8 closed
     with defaults (R1-R12 in feature.md). Status flip _draft_ →
     _in-progress_. Two operator-decide Qs surfaced (anchor checkpoint
     storage; two-checkpoint backtest split). HANDOFF → operator-decide
     → architect. Sources cited per Kronos-pivot lesson:
     BKK18 (https://arxiv.org/abs/1803.01271), locuslab/TCN,
     Keras-TCN, candle-transformers. -->
<!-- updated 2026-05-16 (orchestrator) — Kronos→DL pivot. v2.5
     `v25-kronos-forecast-overlay` dropped after Wave A bootstrap
     surfaced: (a) Kronos lives outside `transformers` so requires
     vendoring upstream Python code; (b) two-model architecture
     (KronosTokenizer + Kronos) requires reimplementing the
     autoregressive sampling loop in Rust; (c) crypto-fit was never
     validated. Reframed project goal locked: "real, working,
     auditable agent architecture; operator learns by building it"
     — and a pre-trained black box scores poorly on the learning
     axis. Pivoted to `v25-dl-forecast-overlay`: train a small
     custom Transformer/TCN in `candle` (the project's named
     prototyping ML framework per CLAUDE.md). See ADR-0028 (which
     supersedes ADR-0027). Wave A crates (forecast, replay-cache,
     core forecast types) are model-agnostic and preserved; only
     Kronos-specific files removed. New brief stub at
     `spec/v25-dl-forecast-overlay/feature.md` (status: draft);
     analyst pass spawned for model family / size / tokenisation /
     data / loss / horizon / success criterion / checkpoint storage
     / audit integration. -->
<!-- updated 2026-05-16 (analyst, Wave 2a spec-hygiene) — three
     stalled strategy features (v0-paper-sma, v05-composed-strategies,
     v1-cross-sectional-momentum) flipped to shipped (bookkeeping;
     code + reports on disk weeks earlier). ui-gallery-bin
     reclassified as shipped-partial-terminal (v0.1-partial); successor
     brief `ui-gallery-table-cell` opened in draft to own the V5+
     tiny-skia table-cell bounds fix. Full breakdown in the
     Changelog entry below. Source: `spec/dev-notes/feature-triage-2026-05-16.md`. -->
<!-- updated 2026-05-15 (analyst, ui-testability deep-dive) — added
     8 candidate features under Queue ## Process / tooling pointing
     into the new `spec/dev-notes/ui-testability-deep-dive-2026-05-15.md`
     dev-note (ui-contrast-asserter, ui-update-proptest, ui-gallery-bin,
     ui-a11y-shadow, ui-vlm-judge, ui-inspect-mcp, ui-session-journal,
     ui-mutants-pass, plus a tester agent-contract addition for the
     visual-fail HTML artifact). Annotated the existing
     ui-test-harness-ci candidate with a cross-platform falsifier
     recommendation to revisit operator decision D3. -->
<!-- updated 2026-05-12 (analyst, second pass) — promoted
     `ui-test-harness-bootstrap` v0.1 from Queue ## Process / tooling
     → Active. First feature under the new
     `AGENT.md ## Capability boundaries` regime (committed 2026-05-12).
     Scope locked to week-1 only of the dev-note 4-week plan:
     `iced_test` smoke + `insta` binary snapshots + canvas hit-test
     grid sweep at three viewport sizes. Operator decisions D1-D5 from
     `spec/dev-notes/archive/2026-Q2/ui-testing-direction-2026-05-12.md ## Section 9`
     LOCKED (analyst does not revisit). Brief at
     `spec/ui-test-harness-bootstrap/feature.md` (status: in-progress,
     version: 0.1.0, predecessor: chart-canvas-overhaul v1.10.0). 10
     open Qs for architect / operator routing. H1 (tiny-skia byte
     determinism) seeded in the new `Hypothesis register` section.
     Non-regression contract: 11 anchors stay byte-identical, 818
     tests stay green, zero changes to non-UI crates. Closes
     chart-canvas-overhaul V15 via the week-1 grid sweep at 3360×1890
     (operator decision D4). HANDOFF → architect. -->
<!-- updated 2026-05-12 (analyst) — promoted `chart-canvas-overhaul`
     v1.10.0 → Active. Opened in response to operator's
     2026-05-12 second visual-verification pass at native
     3360×1890 Retina: tooltip still invisible, chart still
     cropped, no SVG-style scaling, no legend, no axes,
     "not centered". Three items (tooltip, cropping, scaling)
     are regressions vs. v1.9.0's M6.2 hardening pass — the
     v1.9.0 PASS verdict was issued against a 1280×720 tester
     capture, missing the operator's actual hardware shape.
     Brief at `spec/chart-canvas-overhaul/feature.md`; tasks
     stubbed at `spec/chart-canvas-overhaul/tasks.md` (T3001+).
     Three operator-decide questions (Q1 "not centered" reading,
     Q4 UTC-vs-local time axis, Q7 viewer parity) need operator
     answers before architect spawn. -->
<!-- updated 2026-05-04 (analyst, post-Phase-1-ship roadmap revision) —
     `lumen-phase-1-foundation` shipped (tester third-pass PASS) → moved
     Active → Recent. Lumen master roadmap revised from 4 phases to 6
     at operator request: new Phase 2 Shell IA + Charts and Phase 3
     Detail screens insert ahead of original phases. Old Phase 2
     (viewer backtest) → Phase 4. Old Phase 3 (HumanControl + AgentFeed)
     → Phase 5. Old Phase 4 (Assistant slot) → Phase 6 (still reserved
     for v2 LLM). Five new feature brief stubs queued. UI / cockpit
     queue subsection rewritten for the 6-phase plan. -->
<!-- updated 2026-05-03 (analyst) — `v1.5b multi-venue` moved Active →
     Recent (shipped 2026-05-03). New `lumen-design-adoption` master
     roadmap + `lumen-phase-1-foundation` first-phase brief promoted to
     Active. `lumen-phase-2-viewer-backtest` and
     `lumen-phase-3-human-control-agent-feed` queued.
     `lumen-phase-4-assistant` reserved for v2 LLM. Three operator-locked
     constraints (no brand adoption, no voice rewrite, sequential
     phasing) documented in the master roadmap. -->


# Backlog

Queued ideas the operator has surfaced but hasn't promoted to a
feature brief yet. One line each + a note on cost or blockers. This
file is editable churn — nothing here is a commitment.

Promote an item to real work by spawning the **analyst**, who turns it
into a `spec/<slug>/feature.md` brief and removes the entry here.

## Active

> **📋 WIND-DOWN RECONCILIATION (2026-06-15).** The research program concluded
> 2026-06-08 (ship passive); most entries below are SHIPPED or CONCLUDED, not live
> work — retained inline for archaeology per this backlog's convention. Full per-entry
> reconciliation: [`spec/dev-notes/backlog-staleness-audit-2026-06-15.md`](dev-notes/backlog-staleness-audit-2026-06-15.md)
> (audit said 14 stale / 5 accurate / 9 genuinely-open; a 2026-06-16 re-verify found
> `lab-polish-round-2` was ALSO already shipped — built ad-hoc 2026-05-25 (issue #62),
> now registered, and `lab-yahoo-realdata-v0.1.4` RETIRED 2026-06-16 (operator: non-load-bearing
> completeness; research concluded). The genuinely-open set is shrinking as the operator's
> 1-5 close-out lands — 2026-06-16: #1 lab-polish + #2 tracing-redactor + #3 visual-fail-html-reporter
> SHIPPED, #7 retired; #4 viewport-matrix reconciling, #5 subscription-pipe in dev. Remaining
> genuinely-open:
> `subscription-pipe-server-time-template` (in-progress),
> `ui-test-harness-viewport-matrix` (dev-done → tester),
> `lab-recipe-test-harness v0.3.0+` (awaiting analyst), `cockpit-cross-platform`
> (dev-done; CI deferred to near-done), and `v2-llm-strategy-v21-followups a+c`
> (deferred indefinitely). Everything else in Active is done.

<!-- ═══════════════════════════════════════════════════════════════════════════
     PROGRAM CONCLUDED 2026-06-08 — ACTIVE-EDGE SEARCH CLOSED, SHIP PASSIVE.
     This block is the AUTHORITATIVE terminal state and SUPERSEDES the two
     on-chain entries immediately below (the fork decision-support + the spike
     verdict), which are retained as archaeology of how the conclusion was reached.
     ═══════════════════════════════════════════════════════════════════════════ -->

<!-- updated 2026-06-08 (analyst, TERMINAL VERDICT — active-vs-passive search
     CONCLUDED; SHIP PASSIVE) — the pre-committed on-chain hard-stop FIRED and the
     program concludes as pre-registered. Decision of record:
     `spec/product.md` § Strategy library (terminal verdict). Operational artifact:
     `spec/runbooks/passive-baseline.md` (NEW).

     THE VERDICT: across the THREE reachable information channels — price/OHLCV,
     derivatives-positioning, and on-chain — no active strategy beats passive
     buy-and-hold (+1.74/2023, +1.10/2024) net of cost under the frozen
     block-bootstrap MC § 0 rule. The recommended/shipped approach is PASSIVE.
     On-chain (the highest-prior remaining orthogonal channel) got its fair test:
     exchange net-flows are PIT-INFEASIBLE (no free immutable past-only series;
     CryptoQuant disclaims point-in-time accuracy) and the cleaner-PIT
     stablecoin-supply fallback is FRAGILE (sign flips year-over-year under the
     same live-bar that CERTIFIED the basis signal). Full spike:
     `spec/dev-notes/onchain-netflow-spike-2026-06-08.md`.

     SCOPE (honest, NOT overclaimed): this licenses "active ≤ passive in the
     REACHABLE universe (price + positioning + on-chain), net of cost, on the
     2023-24 large-cap perp sample" — NOT "active trading is impossible." Untested
     lower-prior/infeasible channels remain (options/DVOL, macro, social) and are
     OUT OF SCOPE for this program (a future FRESH program, not a continuation).
     The +1.74 BH bar is partly a structural bull-leg artifact of the sample.

     "SHIP PASSIVE" = promotion of already-built+anchored code + documentation,
     NOT a build. Produces: (1) BH control marked the canonical production baseline
     in product.md; (2) `spec/runbooks/passive-baseline.md` (baseline = BH on the
     configured universe; rebalance cadence — monthly/equal-weight proposed default,
     operator-confirmable; paper-mode run recipe; BH anchor scenarios). Requires NO
     new strategy crate, NO new ScoreSource, NO new sweep arm, NO new anchor, NO
     further domain hunt. The hard-stop BINDS.

     ───────────────────────────────────────────────────────────────────────────
     WIND-DOWN STATE — residual items to a clean close (no new active work):

     ▸ STATUS 2026-06-08 (orchestrator) — WIND-DOWN ESSENTIALLY COMPLETE:
       • A (ratification): the program capstone + BOTH close-out decks are
         OPERATOR-RATIFIED ("I approve", 2026-06-08) — approval blocks ticked +
         logged. REMAINING: the rebalance (cadence, weighting) confirmation only.
       • B (status-flip hygiene): DONE — 6 feature.md statuses advanced (0ec68a2).
       • C (foreclosed lanes): DONE — v3-xgboost retired/foreclosed (0ec68a2).
       • Audit P1/P2 doc-hygiene: DONE — stale-count, RNG reconciliation, carry
         trace row, LLM anchor-disposition, bps cross-ref (6cb9b3a).
       Only the rebalance-cadence confirmation remains open; all else closed.
       (The detailed A–D list below is retained as the wind-down record.)

     A. OPERATOR RATIFICATION (one decision, two ratifications):
        - Confirm the rebalance `(cadence, weighting)` for the passive baseline
          (proposed: monthly / equal-weight) → record in the runbook changelog.
        - Ratify the TWO un-approved presenter close-out decks (both `status: draft`
          with empty operator-approval sections — byte-immutable, do NOT edit):
            · `spec/perp-basis-mn-spread/presentations/perp-basis-mn-spread-2026-06-08.md`
              (domain-close retrospective; VERDICT PASS)
            · `spec/perp-basis-signal-robustness/presentations/perp-basis-signal-robustness-2026-06-06.md`
              (long-only close-out; VERDICT PASS)
          A program-level wind-down deck (active-vs-passive close, all three
          channels) is OPTIONAL at operator discretion — the presenter authors it
          only on greenlight; it is NOT required to close the program.

     B. STATUS-FLIP HYGIENE (audit-2026-06-08 § Status drift, P1 — 4th consecutive
        recurrence; orchestrator-owned, mechanical). FIVE feature.md files lag their
        actual pipeline state and should advance:
            · carry-strategy                  arch-done   → retired / closed
            · horizon-retest-robustness       arch-done   → presenter-done
            · time-series-momentum-robustness tester-done → presenter-done
            · perp-basis-signal-robustness    arch-done   → presenter-done
            · perp-basis-mn-spread            arch-done   → presenter-done
        Trace rows + anchors (119/119) were maintained correctly each cycle; only
        the feature.md mirror lagged. The audit recommends a pre-commit enforcement
        hook (sibling to `adr_registry_check.py`) — that hook is a SEPARATE process-
        tooling item, NOT part of the active-edge search.

     C. FORECLOSED ACTIVE-STRATEGY LANES (closed by the terminal verdict; remove
        from any "live lane" reading):
            · `v3-xgboost-cheap-classifier` (status: draft, Queue pre-position) —
              an OHLCV-domain regime classifier. The OHLCV channel is exhausted and
              the hard-stop forecloses further OHLCV active bets under this program.
              Mark FORECLOSED (re-openable only as a fresh program, not this hunt).
            · All other v2.5/v3 predictive lanes already RETIRED (TCN, PatchTST,
              GARCH-σ, regime-classifier) or shipped-partial (LLM-forecaster) — no
              change; recorded here only so the wind-down is complete.

     D. HARNESS DISPOSITION: keep the robustness harness, the anchored surfaces, the
        fetchers, and the new read-only `crates/data/examples/stablecoin_diag.rs`
        probe WARM BUT IDLE — reusable by any future fresh program, but no further
        domain is pursued under THIS hunt. `perp-basis-mn-spread` (tester-done PASS)
        and the rest of the program wave move to Recent on operator ratification.

     The durable deliverable shipped is the ROBUSTNESS MACHINE + the auditable
     negative across three orthogonal channels — a complete, honest product and a
     SUCCESS of the "measured robustness, not asserted alpha" thesis. -->

<!-- updated 2026-06-08 (analyst, on-chain-vs-conclude fork — DECISION-SUPPORT,
     awaiting operator pick) [SUPERSEDED 2026-06-08 by the TERMINAL VERDICT block
     above — the on-chain hunt RAN and the hard-stop fired; retained as archaeology]
     — `perp-basis-mn-spread` MN v0.2.0 closed PASS /
     FAMILY-UNIFORM-FRAGILE in all 3 arms (HEAD `8c2e6c4`), retiring the entire
     DERIVATIVES-POSITIONING domain with finality (k2: mn-basis ≡ mn-funding
     BYTE-IDENTICAL surfaces — basis IS funding on this universe; basis⊥funding
     residual NEGATIVE median Sharpe + 100% tail-DD → no orthogonal alpha). This
     is the SECOND full data domain exhausted with uniform negatives under the
     frozen block-bootstrap MC § 0 rule (after OHLCV/price: 4 families × 3
     horizons × universe axis). Passive buy-and-hold remains undefeated.

     **STRATEGIC FORK now at the operator** (NOT auto-resolved by pre-registration
     despite the "route to on-chain IFF MN fragile" pre-commit — two exhausted
     domains warrant an explicit conclude-vs-continue call). Full decision-support:
     `spec/dev-notes/onchain-vs-conclude-fork-2026-06-08.md`.

     ANALYST RECOMMENDATION: **ON-CHAIN (one bounded hunt, then conclude) — the
     durable choice (Recommended per durable-over-quick).** The two-domain negative
     is STRONG-but-DOMAIN-LIMITED: it licenses "no harvestable edge in PRICE or
     DERIVATIVES-POSITIONING data on these large-caps net of cost," NOT "no edge
     anywhere" — in information space the program has tested ~1.5 distinct channels
     (price + a positioning signal that collapsed onto its own funding mirror), not
     2. On-chain is the FIRST genuinely-orthogonal channel (settlement-layer flows:
     exchange net-flows / stablecoin supply / miner flows — different substrate +
     population, not a price/positioning transform). Prior LOW-to-MEDIUM (NOT
     inflated); cost ~5-8 dev-days (no on-chain plumbing exists; daily ~730 pts/yr
     = 12× thinner tail than the hourly domains; PIT hygiene HARD — on-chain
     revisions/address-relabeling rewrite history). Routes WITH a PRE-COMMITTED
     HARD-STOP: FRAGILE on-chain under the frozen rule → CONCLUDE + ship passive,
     NO further domain hunt (not options, not macro, not on-chain sub-signals).
     Highest-prior first signal: **EXCHANGE NET-FLOWS** (clearest causal price link
     + strongest orthogonality + free daily history; PIT leak-check falsifier is
     THE gate; stablecoin-supply = cleaner-PIT fallback).

     If-budget-tightens (the cheaper lane that is NOT conclude-now): **on-chain
     research spike first** (~1-2 dev-days; clone `basis_diag.rs` → `netflow_diag.rs`,
     one series BTC+ETH, daily rank-IC + sign-persistence + PIT leak-check, NO
     ScoreSource); gate the full ~5-8d build on a non-zero sign-stable IC.

     FALLBACK (named, fully defensible, the CHEAPER choice): **conclude now / ship
     passive** — zero more dev-days; "ship passive" = promote the already-built +
     anchored BH control from "benchmark" to "the strategy the paper-agent runs"
     (a PROMOTION, not a build) + a product.md thesis-doc update (landed alongside
     the fork note). Defensible if the operator weights the ~10-consecutive-negatives
     base rate above the orthogonality-diversity argument.

     Remaining-domain map (on-chain ranks #1): options/DVOL #2 (orthogonal but
     2-symbol + retired-vol skepticism), cross-asset/macro #3 (exogenous but
     harness-adapter-blocked + unstable beta), social #4 (feasibility-blocked),
     OI/LSR #5 (in the just-closed domain + paid-for-history), cross-exchange #6
     (HFT), non-crypto #7 (off-mandate). On-chain is the BEST next bet but NOT the
     last reasonable domain — concluding now forecloses options + macro too.

     NO on-chain feature brief authored yet (deferred to operator greenlight per
     trace.toml ownership rule); NO `[[req]]` row opened; NO code; NO commit.
     product.md thesis sharpened (two-domains-exhausted record +
     passive-may-be-terminal note + on-chain-as-final-probe). `perp-basis-mn-spread`
     is tester-done PASS and moves to Recent on operator fork-resolution. -->

<!-- updated 2026-06-08 (analyst, on-chain spike RAN — VERDICT: HARD-STOP →
     CONCLUDE + ship passive) [SUPERSEDED 2026-06-08 by the TERMINAL VERDICT block
     above, which folds this verdict into the program-conclusion + wind-down state;
     retained as the detailed spike record] — the operator greenlit the bounded
     on-chain hunt (the if-budget-tightens spike-first lane) and it has now RUN.
     Full spike:
     `spec/dev-notes/onchain-netflow-spike-2026-06-08.md`. Both pre-registered
     branches landed on the pre-committed FUSE:

     (1) EXCHANGE NET-FLOWS — KILLED at the data-feasibility / PIT gate (run FIRST,
     as mandated). The canonical free net-flow source (CryptoQuant) fails BOTH
     sub-gates: PAID API key required, AND the vendor's own docs DISCLAIM
     point-in-time accuracy ("does not support PIT accuracy due to periodic updates
     to wallet address clustering; historical data may change as new exchange
     wallets are discovered") — the exact address-relabeling look-ahead the fork
     note pre-registered as the net-flow killer, confirmed verbatim. No free source
     serves an immutable past-only net-flow series. FEASIBILITY verdict → fuse.
     → PIVOTED to the pre-named cleaner-PIT fallback: STABLECOIN SUPPLY.

     (2) STABLECOIN SUPPLY — PIT-clean + free, but FRAGILE. Cleared the data/PIT
     gates cleanly (DefiLlama, free/no-auth/daily/full-2023-2024; forward-recorded —
     verified Base chart begins 2023-08-15 at mainnet launch, zero pre-launch
     backfill; leak-check PASSES causal≠leaked every horizon). But fails the basis
     spike's LIVE bar at EVERY horizon: no cell jointly sign-stable across 2023 AND
     2024 with |IC|≥0.05 (per-chain TS L=7d +0.011→−0.086, L=14d +0.036→−0.130 —
     signs FLIP; aggregate→BTC same-sign cells ALL inside 2σ noise bands, n=25-51).
     Orthogonal to momentum (|corr|<0.07) but moot without a replicating signal.
     Universe reality: only ETH/BNB/SOL/AVAX carry usable per-chain supply (4 names,
     too thin for a rank-IC). Calibration: the basis was LIVE *because* it held the
     same sign both years; this flips → same rule, opposite verdict. Confidence HIGH.

     → VERDICT: HARD-STOP. The most-orthogonal remaining channel got its fair test
     on the cleanest free PIT-clean series and FAILED → "active ≤ passive in the
     reachable universe" is now ASTERISK-FREE across THREE channels (price +
     positioning + on-chain). Per the pre-committed fuse the program CONCLUDES the
     active-vs-passive search and SHIPS PASSIVE. The hard-stop BINDS: NO options
     hunt, NO macro hunt, NO on-chain sub-signal mining (miner flows / active
     addresses are OUT — the channel got its representative test via its two
     strongest, cleanest-PIT signals). "Ship passive" = promote the already-built +
     anchored BH control "benchmark → production strategy" (a PROMOTION, not a build)
     + a product.md terminal-thesis update. NEXT: operator ratifies the conclusion →
     a `ship-passive-baseline` promotion feature (analyst authors the brief +
     `[[req]]` row on greenlight). Artifacts: read-only probe
     `crates/data/examples/stablecoin_diag.rs` + banked DefiLlama series under
     `data/defillama-stablecoins/` (REVISION pin `782148bd…`; parquets gitignored,
     manifest tracked). NO feature brief (spike said HARD-STOP, not BUILD); NO
     `[[req]]` row; NO strategy/ScoreSource/run_path/anchor surface; NO commit. -->

<!-- updated 2026-05-30 (analyst, monte-carlo-robustness-lane M0 close) —
     **PROMOTED Idea → Active 2026-05-30**. Opens the Monte-Carlo robustness
     lane under the operator's 4 locked strategic decisions (2026-05-30):
     Q1 = stationary block bootstrap first (Politis–Romano; resamples REAL
     crypto returns → fat tails + vol clustering free; GBM at main.rs
     synthetic_bars DEMOTED to smoke-test); Q2 = seed the ensemble (ChaCha20
     from one master seed) → byte-identical N-path SET → anchor ONE
     distribution summary (NOT N per-path anchors); Q3 = robustness harness
     FIRST, deterministic learning loop (C4) LAST; Q4 = LLM ratified as a
     SUPPORT pillar (regime narration / lesson summarization / robustness-report
     explanation / statistical tie-break — NOT the alpha source), now in
     product.md § Pillar stack — core vs support.

     **Minimum coherent first slice = C1 + C2 (TWO features).** Decomposition
     justified in C1's feature.md § Two-feature decomposition: C1 is a reusable
     crates/data primitive (consumed by C2 AND the C3/C5 Queue follow-ons); C2
     is a crates/backtest consumer with its own anchor + ADR obligation.

     - **C1 — `monte-carlo-bootstrap-path-generator` v0.1.0** (crates/data).
       Stationary-block-bootstrap path generator: pure function of (real return
       series, revision SHA, path seed, N, block-length policy) → N synthetic
       paths via ChaCha20. Auto-tunable block length per Politis–White (2004) +
       Patton–Politis–White (2009). GBM lifted behaviour-preserving into
       data::synth::gbm as the demoted smoke-test. R1-R4 + R-NR(6) + K1-K4 +
       H1-H3 + Q-MCB-1/2/3 + 4-cell verdict tree. Trace
       `REQ-MC-BOOTSTRAP-PATH-GENERATOR-001` opened proposed. Adds NO anchor;
       existing synthetic anchors MUST stay byte-identical post-lift (the K4
       blast-radius risk → Q-MCB-3 carries the Recommended=durable EXCEPTION:
       anchor byte-immutability may make the cheap GBM-wrap the honest ship —
       architect resolves with lift-safety evidence at M-T1).

     - **C2 — `strategy-robustness-harness` v0.1.0** (crates/backtest). Runs a
       strategy over C1's N paths (reuses the threshold_sweep sweep+aggregate
       seam the architect found ~80% built), reduces to a DISTRIBUTION SUMMARY
       (Sharpe p5/p50/p95, max-drawdown tail, prob-of-loss, P(Sharpe>0/1)),
       emits ONE anchored summary report under a new namespace. R1-R3 + R-NR(6,
       incl the MANDATORY adapted CLAUDE.md gate R-NR.6 = distribution diverges
       from single-path baseline by a testable epsilon AND is byte-identical
       across two seeded runs) + K1-K4 + H1-H3 + Q-RH-1/2 + 4-cell verdict tree.
       Trace `REQ-STRATEGY-ROBUSTNESS-HARNESS-001` opened proposed. Depends on
       C1. **ADR-0051 FLAGGED for the architect** ("Monte-Carlo robustness:
       synthetic-path ensembles, distribution-report shape, and anchor
       determinism" — next free number confirmed 0051; locks D1 sub-seed rule /
       D2 reduction order + percentile rule / D3 report FM-body split + fixed
       precision / D4 anchor unit = 1 summary report / D5 determinism scope =
       Apple-Silicon canonical box inheriting ADR-0043 verbatim). The analyst
       does NOT write the ADR — architect M-T1 deliverable.

     C3 (param-sweep) / C4 (learning loop — Q3 LAST) / C5 (CPCV/Deflated-Sharpe)
     are Queue follow-ons (see Queue § Strategy), NOT promoted in this slice.
     Direction: spec/dev-notes/strategy-robustness-monte-carlo-direction-2026-05-29.md.
     Architecture readiness:
     spec/dev-notes/monte-carlo-robustness-architecture-readiness-2026-05-29.md.
     PARALLEL-SAFE with in-flight lanes — C1/C2 touch crates/data (new synth/)
     + crates/backtest (new montecarlo scenario + monte_carlo bin); disjoint
     from the Yahoo/Lab UX lane + the Pick C scripts/ Python lane. HANDOFF →
     architect (M-T1 on C1+C2 + ADR-0051). -->

<!-- updated 2026-05-30 (analyst, lab-yahoo-empty-range-ux M0 close) —
     **PROMOTED Idea → Active 2026-05-30**. Small (~1-2 dev-days),
     operator-motivated UX-polish. Discharges the Bug #64 D.1.1
     attempt-3 presenter-deck FYI #2 carry-forward
     (`spec/bug-64-d11-attempt-3-yahoo-run-runtime-context/presentations/bug-64-attempt-3-2026-05-29.md`
     § Notes/feedback FYI #2): under the 2026 future-dated test clock,
     `Last 30d`/`Last 90d` Yahoo presets compute future-dated windows
     (e.g. 2026-04-29..2026-05-29) for which the real Yahoo API has NO
     data; the cockpit today renders a confusing red ⚠ error referencing
     an internal CacheMiss/MissingData variant + a misleading "Check
     network connectivity" hint, so the operator cannot tell "broken"
     from "no data exists". Scope: (R1) classify empty-vs-error at the
     `preload_yahoo_bars` boundary; (R2) surface a distinct, plain-language
     no-data message naming the ticker + resolved window; (R3) run
     terminates cleanly (no spinner hang); (R4) date-preset guard
     (clamp/warn/none per Q2); R-NR synthetic byte-identical + zero
     anchor delta. 4R / R-NR / K1-K4 / H1-H3 / Q1-Q3 + 4-cell verdict
     tree, all Qs biased DURABLE per AGENT.md 2026-05-28. Q1 LOAD-BEARING
     (classification): (a) [Recommended — DURABLE] explicit fetch-outcome
     classification (~1.5d, correct under K1 Yahoo-outage, extends to
     v0.2.0 equities sparse-ranges) vs (b) [cheap fallback] bar-count
     heuristic (~0.5d but fragile + spawns v0.2.0 cleanup). Q2 preset:
     (a) [Recommended — DURABLE] clamp end_ms to now when future-dated
     (clamp ONLY when end>now, past ranges byte-identical). Q3 surface:
     (a) [Recommended — DURABLE] distinct NOTICE style (not red error)
     so the operator visually distinguishes no-data-expected from
     broken-act-now. M-T1 likely ADR-0040 Changelog amendment only (no
     new ADR; no ADR-0050 rt.spawn touch). New trace row
     `REQ-LAB-YAHOO-EMPTY-RANGE-UX-001` opened proposed. PARALLEL-SAFE
     with 3 Pick C architects + lab-recipe Wave C dev (disjoint scopes:
     Pick C is scripts/ Python; this is crates/data + crates/ui
     Yahoo/Lab paths). HANDOFF → architect (M-T1 design pass). -->

<!-- updated 2026-05-29 (analyst, lab-yahoo-realdata-v0.1.4-bulk-ticker-re-emit
     M0 close) — **PROMOTED Idea → Active 2026-05-29**. Closes the v0.1.3
     deck's explicit owned-debt commitment (presented at v0.1.3
     M-PRESENTER 2026-05-29 § What's deferred to v0.1.4): re-emit 9
     remaining Yahoo crypto-mirror tickers (BNB, SOL, XRP, ADA, DOGE, AVAX,
     DOT, LINK, MATIC) AND re-emit ETH-daily row 70 under the v0.1.3
     helper-extracted emit shape (`revision_sha:` in front-matter, no `rev=`
     in body). After ship: all 10 crypto-mirror tickers carry anchored
     2024 SMA(20,50) backtests under the v0.1.3 canonical helper; aggregate
     `cache_state_summary_badge` flips from "Yahoo cache: 2 tickers" →
     "Yahoo cache: 10 tickers" — badge becomes meaningful at full universe
     coverage. Scope: (R1) operator-side cache populate (BLOCKER for M-DEV
     — explicit `fetch_yahoo_klines --tickers BNB-USD,SOL-USD,XRP-USD,ADA-USD,DOGE-USD,AVAX-USD,DOT-USD,LINK-USD,MATIC-USD --interval 1d --start 2024-01-01 --end 2024-12-31`);
     (R2) bulk re-emit of 10 tickers + ETH-daily via v0.1.3 helper; (R3)
     anchor cascade 71 → 80 (row 70 in-place SHA UPDATE under preserved
     namespace `lab-yahoo-realdata-v0.1.2` per v0.1.3 D-V0.1.3-4 in-place
     precedent; rows 72-80 append under single new namespace
     `lab-yahoo-realdata-v0.1.4` per Q2=(a)); (R4) aggregate badge
     meaningfulness (2 → 10 tickers; zero UI code change — automatic on
     `REVISION.toml` populated-count); (R5) non-regression — durable
     boundary FROZEN (zero diff in `crates/backtest/src/report/yahoo.rs`,
     `report/sma.rs`, `report/mod.rs`, `run_yahoo_sma.rs` — v0.1.3
     helper-extraction is FROZEN per D-V0.1.3-1); (R-NR) zero new design
     tokens / strings.rs adds — backend + scenario-reg only. **5 R / R-NR /
     4 K / 3 H / 2 Q** + non-regression contract + pre-drawn 2-cell verdict
     tree + cost framing (~5-7 days dev + 1 day tester + 0.5 day presenter
     ≈ 1 week wall-clock Q1=(a)+Q2=(a) Recommended; ~1.5-2.5 days Q1=(b)
     cheap but +5-7 days deferred across 8 v0.1.5+ cleanup briefs + 8 H1
     carve-outs in deck). Q1 LOAD-BEARING (per AGENT.md 2026-05-29
     durable-over-quick contract): (a) [Recommended — DURABLE] register
     `{ticker-lc}-2024-h1-sma-cross` Binance hourly scenario per ticker (9
     additions, 3 match-arm sites each, mirroring v0.1.3 D-V0.1.3-5 ETH-H1
     template) — H1 discharges DIRECTLY per ticker, zero K1 Yahoo-to-Yahoo
     fallbacks ship, uniform H1 contract across all 10 crypto-mirror
     tickers; (b) [cheap fallback] BNB-only Binance H1 + 8 K1 Yahoo-to-Yahoo
     fallbacks + 8 v0.1.5+ per-ticker cleanup briefs + 8 H1 carve-outs in
     v0.1.4 deck (the silent-deferral pattern operator dislikes). Q2
     namespace: (a) [Recommended — DURABLE] single `lab-yahoo-realdata-v0.1.4`
     for all 9 new (ETH-daily row 70 stays `lab-yahoo-realdata-v0.1.2` with
     in-place SHA update per v0.1.3 precedent); (b) per-ticker namespaces
     (fragments tracking). M-T1 likely fast-skips (ADR-0040 § Changelog
     amendment only, per v0.1.3 precedent — no new ADR). M-DEV-UI lane
     DOES NOT EXIST at v0.1.4 (backend-only ship). K1 falsifier: any
     ticker returns < 366 bars for 2024 (e.g. AVAX mid-year listing edge
     or MATIC rebrand) → route back analyst with drop-or-widen-threshold
     decision (95% threshold per ADR-0040 § R3). K2 `REVISION.toml` grows
     ~60 → ~177 file rows (acceptable; ADR-0040 § D3 schema unchanged).
     K3 Yahoo throttling at 9 successive fetches → H2 ≥ 95% gate +
     ADR-0040 § D5 `YahooError::RateLimited` + backoff. K4 ticker
     scenario fails to converge → per-ticker-skip route. H1 expected
     5-15% Yahoo-daily vs Binance-hourly delta per ticker (mirrors BTC
     9.03% / ETH 6.78%). Trace row `REQ-LAB-YAHOO-REALDATA-V0-1-4-001`
     opened at `proposed` state. Frontmatter `owner: analyst`,
     `status: draft` — M-OD is next. HANDOFF → architect (M-T1 fast-skip
     ratifies Q1/Q2 + amends ADR-0040 § Changelog + emits dev handoff). -->
- **lab-yahoo-realdata-v0.1.4-bulk-ticker-re-emit v0.1.0** — closes
  v0.1.3's explicit owned-debt commitment: re-emit 9 remaining Yahoo
  crypto-mirror tickers (BNB, SOL, XRP, ADA, DOGE, AVAX, DOT, LINK,
  MATIC) + ETH-daily row 70 under v0.1.3 helper shape; aggregate
  `cache_state_summary_badge` flips "2 tickers" → "10 tickers".
  Anchor cascade 71 → 80 (1 in-place row 70 update + 9 appends).
  Q1 LOAD-BEARING (durable-over-quick per AGENT.md 2026-05-29): (a)
  **Recommended DURABLE** register 9 new Binance hourly scenarios for
  direct per-ticker H1 verification (~+5 days dev); (b) cheap fallback
  BNB-only + 8 K1 Yahoo-to-Yahoo carve-outs + 8 v0.1.5+ cleanup briefs
  deferred. Q2 namespace: (a) **Recommended DURABLE** single
  `lab-yahoo-realdata-v0.1.4`; (b) per-ticker fragments. R1 operator-
  side fetch is BLOCKER for M-DEV — verbatim command in feature.md R1.1.
  Frozen-boundary contract (R5.7): zero diff in
  `crates/backtest/src/report/yahoo.rs`, `report/sma.rs`, `report/mod.rs`,
  `run_yahoo_sma.rs`. Backend-only ship (no M-DEV-UI lane). M-T1 likely
  fast-skips (ADR-0040 § Changelog amendment). Cost ~1 week Q1=(a)+Q2=(a);
  ~1.5-2.5 days Q1=(b)+Q2=(b) but +5-7 days deferred + 8 H1 carve-outs
  in deck. Trace `REQ-LAB-YAHOO-REALDATA-V0-1-4-001` proposed.
  Brief: [`spec/lab-yahoo-realdata-v0.1.4-bulk-ticker-re-emit/feature.md`](lab-yahoo-realdata-v0.1.4-bulk-ticker-re-emit/feature.md).
  HANDOFF → architect (M-T1 fast-skip).
  **2026-05-29 architect M-T1 fast-skip CLOSE:** § Design ratified
  (D-V0.1.4-1 through D-V0.1.4-9). M-OD picks DURABLE at both rows
  (Q1=(a) 9 Binance H1 regs, Q2=(a) single namespace
  `lab-yahoo-realdata-v0.1.4`). ADR-0040 § Changelog amended (no new
  ADR). K1 pre-flight CONFIRMED: `bar_count: 262_800` mirrors v0.1.3
  BTC+ETH-H1 verbatim (real-parquet auto-detect overrides per v0.1.3
  T-D4). K3 AVAX/MATIC: 95% threshold uniform; operator-side R1 fetch
  surfaces K1 BEFORE M-DEV; default drop-on-fire. Anchor cascade
  ratified 71 → 80. Wave decomposition: Wave A bulk re-emit (~1.5d)
  ‖ Wave B 9 H1 regs (~3-4d) → Wave C per-ticker dev-notes (~0.5d)
  → Wave D gates. Operator R1 fetch is M-DEV start gate. R5.7 FROZEN
  boundary contract: zero diff in 4 files. Trace `arch` populated;
  state `proposed → arch-done`. HANDOFF → developer.

<!-- updated 2026-05-29 (analyst, visual-fail-html-reporter M0 close) —
     **PROMOTED Queue → Active 2026-05-29** under Pick A Wave 1 of the
     test-infra trifecta strategic direction at
     `spec/dev-notes/pick-a-test-infra-trifecta-2026-05-29.md` (analyst
     pass following the architect's process-tooling survey at
     `spec/dev-notes/process-tooling-survey-2026-05-29.md` § Pick A
     Top-5 Rank 2). Cheapest of the three trifecta pillars (~1 dev day,
     ~50-80 LoC helper). Closes the agent-contract gap surfaced by
     `ui-testability-deep-dive-2026-05-15.md § 4.1`: on visual-assertion
     FAIL, emit a self-contained `visual-fail-<ts>.html` with inline-
     base64 baseline + actual + diff PNG triple + assertion location +
     assertion body + optional VLM verdict slot. FAIL-only emission
     (PASS path byte-identical to today). Owns the
     `.claude/agents/tester.md` amendment for the trifecta bundle per
     direction § Risk R1 mitigation; viewport-matrix Wave 1 sibling
     inherits the stanza without amendment. Q1 (output path; analyst
     Recommended DURABLE = `target/visual-diff/<test>-<ts>.html` +
     opt-in `EMIT_VISUAL_FAIL_TO_SPEC=1`) + Q2 (base64 encoding crate;
     Recommended DURABLE = base64="0.22" dev-dep) + Q3 (tester.md stanza
     placement; Recommended DURABLE = append to existing section). All
     three Qs bias DURABLE per AGENT.md 2026-05-28. ADR-0048 carries
     forward verbatim (one Changelog row at M-T1, no new ADR). Anchor
     contract zero delta — 71/71 byte-identical pre/post. PARALLEL-SAFE
     with sibling `ui-test-harness-viewport-matrix` per AGENT.md §
     Parallelism rules conflict matrix (independent file-scope; the
     stanza inheritance contract handles the only shared-file risk).
     Trace row `REQ-VISUAL-FAIL-HTML-REPORTER-001` opened `proposed`.
     M-T1 fast-skip expected. HANDOFF → architect (in parallel block
     with viewport-matrix sibling). -->
- **visual-fail-html-reporter v0.1.0** — Pick A Wave 1 trifecta pillar
  (cheapest, ~1 dev day, ~50-80 LoC helper at
  `crates/ui/tests/fixtures/visual_fail_html.rs`). On visual-snapshot
  FAIL, emit a self-contained `visual-fail-<ts>.html` with inline
  baseline + actual + perceptual-diff PNG triple + assertion location +
  assertion body. Closes the
  [`ui-testability-deep-dive-2026-05-15.md § 4.1`](dev-notes/archive/2026-Q2/ui-testability-deep-dive-2026-05-15.md#41-testermd--emit-a-structured-fail-artifact-not-just-prose)
  agent-contract gap. Owns the `.claude/agents/tester.md` amendment for
  the Wave 1 trifecta bundle (viewport-matrix sibling inherits without
  amendment per direction § Risk R1). Q1/Q2/Q3 all bias DURABLE per
  AGENT.md 2026-05-28: target/+opt-in-spec output path, base64="0.22"
  dev-dep, append to existing tester.md stanza. ADR-0048 carries
  forward (no new ADR). Zero anchor delta; 71/71 byte-identical. Trace
  `REQ-VISUAL-FAIL-HTML-REPORTER-001` proposed. Brief:
  [`spec/visual-fail-html-reporter/feature.md`](visual-fail-html-reporter/feature.md).
  Direction: [`spec/dev-notes/pick-a-test-infra-trifecta-2026-05-29.md`](dev-notes/archive/2026-Q2/pick-a-test-infra-trifecta-2026-05-29.md).
  HANDOFF → architect (M-T1 fast-skip; parallel-safe with viewport-matrix
  sibling).

<!-- updated 2026-05-29 (analyst, ui-test-harness-viewport-matrix M0 close) —
     **PROMOTED Queue → Active 2026-05-29** under Pick A Wave 1 of the
     test-infra trifecta strategic direction at
     `spec/dev-notes/pick-a-test-infra-trifecta-2026-05-29.md`. Mid-cost
     pillar (~3-4 dev days). Extends the
     `ui-test-harness-bootstrap v0.1.0` Charts-only three-viewport snapshot
     harness (1280×720 floor / 1920×1080 typical / 3360×1890 operator) to
     ALL existing widget tests under `crates/ui/tests/` (panels, modals,
     status bar, agent feed, debug screen). Architect M-T1 audits the
     existing widget test files (~10-15 files / ~30-40 #[test] fn;
     final count ~90-120 after 3× expansion) + dry-runs baseline PNG
     size (~50-100 MB net repo growth projected per H3) + ratifies opt-
     out list per K1 (widgets that can't render at operator slot; ≤ 3
     widgets). Bootstrap V15 Charts baselines stay byte-identical
     (chart-canvas-overhaul tooltip-hover acceptance preserved). Q1
     (coverage scope) + Q2 (helper shape: function-with-closure vs macro)
     + Q3 (`.gitattributes` rule: `binary diff=exif` vs plain `binary`)
     all bias DURABLE per AGENT.md 2026-05-28 (Q1=(a) full coverage in
     v0.1.0, not phased Charts-first). Inherits the visual-fail-HTML
     stanza from Wave 1 sibling (no independent tester.md amendment).
     ADR-0048 + bootstrap § Design carry forward verbatim (one Changelog
     row at M-T1, no new ADR). Zero anchor delta; 71/71 byte-identical.
     PARALLEL-SAFE with sibling `visual-fail-html-reporter` per
     AGENT.md § Parallelism rules conflict matrix. Trace row
     `REQ-UI-TEST-HARNESS-VIEWPORT-MATRIX-001` opened `proposed`. M-T1
     inventory + dry-run + ratification expected; ~1 week wall-clock
     total to ship. HANDOFF → architect (in parallel block with
     visual-fail-HTML sibling). -->
- **ui-test-harness-viewport-matrix v0.1.0** — Pick A Wave 1 trifecta
  pillar (mid-cost, ~3-4 dev days). Extends the bootstrap Charts-only
  three-viewport harness (1280×720 / 1920×1080 / 3360×1890) to ALL
  existing widget tests under `crates/ui/tests/` (panels, modals,
  status bar, agent feed, debug screen). Architect M-T1 audits ~10-15
  test files + dry-runs ~50-100 MB baseline PNG growth + ratifies
  ≤ 3-widget opt-out list per K1. Bootstrap V15 Charts baselines
  byte-identical pre/post. Q1 (coverage scope) + Q2 (helper shape) +
  Q3 (`.gitattributes` rule) all bias DURABLE per AGENT.md 2026-05-28
  (Q1=(a) full widget coverage in v0.1.0, not phased Charts-first).
  Inherits visual-fail-HTML stanza from Wave 1 sibling (no
  independent tester.md amendment). ADR-0048 + bootstrap § Design
  carry forward (no new ADR). Zero anchor delta; 71/71 byte-identical.
  Trace `REQ-UI-TEST-HARNESS-VIEWPORT-MATRIX-001` proposed. Brief:
  [`spec/ui-test-harness-viewport-matrix/feature.md`](ui-test-harness-viewport-matrix/feature.md).
  Direction: [`spec/dev-notes/pick-a-test-infra-trifecta-2026-05-29.md`](dev-notes/archive/2026-Q2/pick-a-test-infra-trifecta-2026-05-29.md).
  HANDOFF → architect (M-T1 inventory + dry-run + ratification;
  parallel-safe with visual-fail-HTML sibling).

<!-- updated 2026-05-29 (analyst, v2-1-tracing-layer-redactor M0 close) —
     **PROMOTED Queue → Active 2026-05-29** under Pick B Wave 1 of the
     cross-cutting safety duo strategic direction at
     `spec/dev-notes/pick-b-cross-cutting-safety-duo-2026-05-29.md`
     (analyst pass following the architect's process-tooling survey at
     `spec/dev-notes/process-tooling-survey-2026-05-29.md` § Pick B
     Top-5 Rank 3). More-expensive pillar of the duo (~1.5 dev days).
     SPLIT OFF from `v2-llm-strategy-v21-followups` Queue entry (#3);
     LLM-budget tile + clippy items stay Queue per process-tooling
     survey § What's NOT a compounder honorable mentions (defer with
     v2 LLM lane activation). Closes the v2-llm-strategy v2.0.0 pass-3
     deferred half (per `crates/llm/src/redact.rs:18-26` deferral
     note): installs a `tracing_subscriber::Layer` field-visitor at
     the audit/llm boundary that redacts API keys (`sk-` / `sk-ant-`
     / `sk-proj-` / OpenAI-shape) + Bearer tokens + JWTs + AWS-style
     secrets + password-like field NAMES + high-entropy strings
     ≥ 32 chars BEFORE they hit the audit ledger or stdout. Q-RED-1
     (regex set shape; analyst Recommended DURABLE = closed regex
     set + per-site opt-out only) + Q-RED-2 (provider header bypass;
     Recommended DURABLE = wire-layer exemption via reqwest middleware
     NOT redactor allowlist) + Q-RED-3 (WARN-mode flag shape;
     Recommended DURABLE = env var `REDACT_LAYER_MODE=warn|gate`).
     All three Qs bias DURABLE per AGENT.md 2026-05-28. WARN mode
     default at v0.1.0 per shared bundle Q-DUO-WARN (2-week
     observation; v0.2.0 patch flips to gate). Pass-3 redact ADR
     carries forward (one Changelog row at M-T1, no new ADR). Anchor
     contract zero delta — 75/75 byte-identical pre/post (Layer
     affects tracing emit only). PARALLEL-SAFE with sibling
     `ui-contrast-asserter` per AGENT.md § Parallelism rules conflict
     matrix (independent file-scope: `crates/llm` + `crates/audit`
     vs `crates/ui/tests/contrast.rs`). Trace row
     `REQ-V2-1-TRACING-LAYER-REDACTOR-001` opened `proposed`. M-T1
     fast-skip likely. HANDOFF → architect (parallel-spawn with
     ui-contrast-asserter sibling). -->
- **v2-1-tracing-layer-redactor v0.1.0** — Pick B Wave 1 cross-cutting
  safety duo pillar (more-expensive, ~1.5 dev days). Closes the
  v2-llm-strategy v2.0.0 pass-3 deferred half (`crates/llm/src/redact.rs`
  pure-fn was pass 3; the `tracing_subscriber::Layer` field-visitor
  half is this brief). Installs a Layer at the audit/llm boundary that
  redacts API keys + Bearer + JWTs + AWS-style secrets + password-like
  field names + high-entropy strings BEFORE they hit the audit ledger
  or stdout. Every future LLM call and structured log emit inherits
  redaction with zero per-call wiring. **SPLIT off from
  `v2-llm-strategy-v21-followups` Queue (#3)**; LLM-budget tile +
  clippy items stay Queue per the survey § What's NOT a compounder.
  Q-RED-1 (closed regex set + per-site opt-out) + Q-RED-2 (provider
  header bypass at WIRE layer not redactor) + Q-RED-3 (env-var WARN
  mode flag) all bias DURABLE per AGENT.md 2026-05-28. WARN mode at
  v0.1.0 per bundle Q-DUO-WARN (2-week observation). Pass-3 redact
  ADR carries forward (no new ADR). Zero anchor delta; 75/75
  byte-identical. Trace `REQ-V2-1-TRACING-LAYER-REDACTOR-001`
  proposed. Brief:
  [`spec/v2-1-tracing-layer-redactor/feature.md`](v2-1-tracing-layer-redactor/feature.md).
  Direction: [`spec/dev-notes/pick-b-cross-cutting-safety-duo-2026-05-29.md`](dev-notes/archive/2026-Q2/pick-b-cross-cutting-safety-duo-2026-05-29.md).
  HANDOFF → architect (M-T1 fast-skip; parallel-safe with
  ui-contrast-asserter sibling).

<!-- updated 2026-05-29 (analyst, ui-contrast-asserter M0 close) —
     **PROMOTED Queue → Active 2026-05-29** under Pick B Wave 1 of the
     cross-cutting safety duo strategic direction at
     `spec/dev-notes/pick-b-cross-cutting-safety-duo-2026-05-29.md`.
     Cheap pillar of the duo (~0.5 dev days). Data-driven
     `crates/ui/tests/contrast.rs` test that enumerates `(fg, bg)`
     token pairs derived from `crates/ui/src/theme.rs` and asserts
     WCAG 2.1 contrast ratios per `spec/ui-design-principles.md
     ## Accessibility minimums` (4.5:1 AA body, 7:1 AAA equity).
     New tokens auto-inherit assertion without per-token wiring.
     R1.4 MIN_PAIRS floor assertion (≥ 30 per architect M-T1
     ratification) defends against silent enumeration break from
     future token storage refactors (K2). Q-CONT-1 (WARN-mode default;
     analyst Recommended DURABLE = inherits bundle Q-DUO-WARN 2-week
     WARN observation) + Q-CONT-2 (WCAG formula impl; Recommended
     DURABLE = hand-rolled ~20 LoC, zero dep escalation for closed
     WCAG 2.1 math) + Q-CONT-3 (opt-out marker placement; Recommended
     DURABLE = in-file `OPT_OUTS` table inside `contrast.rs`, theme.rs
     stays clean of test-only annotations). All three Qs bias
     DURABLE per AGENT.md 2026-05-28. ADR-0048 boundary-test
     precedent carries forward (one Changelog row at M-T1, no new ADR).
     Anchor contract zero delta — 75/75 byte-identical pre/post (zero
     production code touched per R-NR.1; pure test infra addition).
     PARALLEL-SAFE with sibling `v2-1-tracing-layer-redactor` per
     AGENT.md § Parallelism rules conflict matrix. Trace row
     `REQ-UI-CONTRAST-ASSERTER-001` opened `proposed`. M-T1 includes
     one-pass theme.rs audit + opt-out list seed + MIN_PAIRS floor
     ratification; fast-skip otherwise. HANDOFF → architect
     (parallel-spawn with v2-1-tracing-layer-redactor sibling). -->
- **ui-contrast-asserter v0.1.0** — Pick B Wave 1 cross-cutting safety
  duo pillar (cheap, ~0.5 dev days). Data-driven test at
  `crates/ui/tests/contrast.rs` enumerates `(fg, bg)` token pairs from
  `crates/ui/src/theme.rs` and asserts WCAG 2.1 contrast per
  [`ui-design-principles.md ## Accessibility minimums`](ui-design-principles.md#accessibility-minimums)
  (4.5:1 AA body, 7:1 AAA equity). Closes the palette-refactor
  regression class without rendering a pixel. New tokens auto-inherit
  assertion. R1.4 MIN_PAIRS floor defends against silent enumeration
  break (K2). Q-CONT-1 (WARN default inheriting bundle Q-DUO-WARN) +
  Q-CONT-2 (hand-rolled formula, zero dep) + Q-CONT-3 (in-file
  OPT_OUTS table, theme.rs stays clean) all bias DURABLE per
  AGENT.md 2026-05-28. WARN mode at v0.1.0 (2-week observation;
  v0.2.0 patch flips to gate). ADR-0048 boundary-test precedent
  carries forward (no new ADR). Zero anchor delta; 75/75 byte-identical.
  Trace `REQ-UI-CONTRAST-ASSERTER-001` proposed. Brief:
  [`spec/ui-contrast-asserter/feature.md`](ui-contrast-asserter/feature.md).
  Direction: [`spec/dev-notes/pick-b-cross-cutting-safety-duo-2026-05-29.md`](dev-notes/archive/2026-Q2/pick-b-cross-cutting-safety-duo-2026-05-29.md).
  HANDOFF → architect (M-T1 fast-skip + one-pass theme.rs audit +
  opt-out list seed; parallel-safe with v2-1-tracing-layer-redactor
  sibling).

<!-- updated 2026-05-29 (analyst, pick-c-orchestrator-hygiene-compounder-trio
     M0 close) — **PROMOTED Queue → Active 2026-05-29** under Pick C
     Wave 1 of the architect's process-tooling survey at
     `spec/dev-notes/process-tooling-survey-2026-05-29.md` § Pick C
     (Top-5 Rank 5). Three features promoted in parallel:
     `queue-staleness-reconciliation` (~1 dev day),
     `adr-registry-atomic-lint` (~0.5 dev day),
     `operator-ledger-schema-lint` (~0.5 dev day). Bundle total
     ~2 dev days — cheapest of architect's Month-1 picks (vs Pick A
     trifecta at ~5-7d + Pick B duo at ~2d). Operationalises retro
     fix-improve #1 (Queue staleness) + #6 (ADR registry drift) +
     #5 (operator pending ledger). All three are Python-stdlib-only
     scripts under `scripts/` — zero Cargo touch, zero new external
     deps, zero anchor delta. Strategic direction at
     `spec/dev-notes/pick-c-orchestrator-hygiene-2026-05-29.md`
     frames the trio (durable-over-quick per AGENT.md 2026-05-28).
     One bundle-level operator-decide Q-HYG-EMIT (Recommended
     DURABLE = markdown table + per-violation context lines) locked
     shared diff dialect across all three scripts. Per-feature K4
     amendment ownership-table:
     `queue-staleness-reconciliation` OWNs AGENT.md § Queue
     pre-flight invocation example; `adr-registry-atomic-lint` OWNs
     architect.md § ADR registry invocation example;
     `operator-ledger-schema-lint` OWNs ledger frontmatter R3.3
     amendment + AGENT.md cross-reference R3.4. Per-feature Qs all
     bias DURABLE: Q-QSR-1 status-mismatch only at v0.1.0 (defer
     stale-by-age to v0.2.0), Q-QSR-2 markdown table (inherits
     Q-HYG-EMIT), Q-ADR-WHEN pre-commit hook, Q-ADR-AMEND always
     bump on any ADR modification, Q-LED-WHEN session pre-flight,
     Q-LED-NOTE require dev-note citation on FAILED rows. No new
     ADRs — all three scripts + thin contract codifications under
     existing AGENT.md / architect.md sections. Three new trace
     rows opened proposed:
     `REQ-QUEUE-STALENESS-RECONCILIATION-001` +
     `REQ-ADR-REGISTRY-ATOMIC-LINT-001` +
     `REQ-OPERATOR-LEDGER-SCHEMA-LINT-001`. PARALLEL-SAFE with all
     in-flight agents (Bug #64 dev edits trace.toml + ledger row
     only — disjoint hunks; v5 + v2.1 presenters write decks;
     v5 cleanup dev touches crates/strategy/). HANDOFF →
     architect (M-T1 fast-skips expected across all three). -->
- **queue-staleness-reconciliation v0.1.0** — Pick C Wave 1
  orchestrator hygiene compounder trio LARGEST pillar (~1 dev day).
  Python-stdlib script `scripts/queue_staleness_check.py` the
  orchestrator invokes at session start. Reads `spec/backlog.md`
  Queue + Active sections, cross-references each Queue feature
  folder's frontmatter `status:`, flags status-mismatch drift (Queue
  stub claims candidate / proposed / in-flight but folder is
  `shipped | shipped (retired) | deprecated`). Excludes already-
  annotated post-ship Queue text per AGENT.md § Queue pre-flight
  § step 2. Catches the recurring 30-45 min reactive cleanup cost
  surfaced 3× in 3 weeks (audits 2026-05-07 / 05-27 / 05-29).
  Q-QSR-1 stale-by-age scope (a) **DURABLE** status-mismatch only
  at v0.1.0 (defer stale-by-age to v0.2.0 after operator signal).
  Q-QSR-2 emit format (a) **DURABLE** markdown table per bundle
  Q-HYG-EMIT. OWNS AGENT.md § Queue pre-flight reconciliation sweep
  invocation example amendment per bundle K4 ownership-table. No
  new ADR; no Cargo touch. Zero anchor delta. Trace
  `REQ-QUEUE-STALENESS-RECONCILIATION-001` proposed. Brief:
  [`spec/queue-staleness-reconciliation/feature.md`](queue-staleness-reconciliation/feature.md).
  Direction: [`spec/dev-notes/pick-c-orchestrator-hygiene-2026-05-29.md`](dev-notes/archive/2026-Q2/pick-c-orchestrator-hygiene-2026-05-29.md).
  HANDOFF → architect (M-T1 fast-skip + parse-shape ratification +
  self-test cases; parallel-safe with adr-registry-atomic-lint +
  operator-ledger-schema-lint siblings).

- **adr-registry-atomic-lint v0.1.0** — Pick C Wave 1 orchestrator
  hygiene compounder trio CHEAPEST pillar (~0.5 dev day). Python-
  stdlib script `scripts/adr_registry_check.py` enforcing the
  architect.md § ADR registry atomic-write contract (codified
  2026-05-29). Pre-commit hook on any commit touching
  `spec/architecture/adr/`. Asserts (a) every ADR file has a row in
  `architecture/adr/README.md ## Registry`; (b) README frontmatter
  `updated:` bumped same-commit on any ADR modification per Q-ADR-AMEND
  (a) DURABLE strictest interpretation; (c) ADR status enum in
  `{accepted, proposed, superseded, deprecated}`. Excludes
  TEMPLATE.md + README.md from per-ADR checks. Catches the recurring
  registry drift class (ADRs 0044+, 0045-0049 unregistered). Q-ADR-WHEN
  (a) **DURABLE** pre-commit hook (catches drift before git history;
  CI-only rejected because architect.md contract reads as 'atomic at
  commit authoring time'). Q-ADR-AMEND (a) **DURABLE** always bump
  on any ADR modification (zero ambiguity at lint time). OWNS
  architect.md § ADR registry atomic-write invocation example
  amendment per bundle K4 ownership-table. No new ADR; no Cargo
  touch. Zero anchor delta. Trace `REQ-ADR-REGISTRY-ATOMIC-LINT-001`
  proposed. Brief:
  [`spec/adr-registry-atomic-lint/feature.md`](adr-registry-atomic-lint/feature.md).
  Direction: [`spec/dev-notes/pick-c-orchestrator-hygiene-2026-05-29.md`](dev-notes/archive/2026-Q2/pick-c-orchestrator-hygiene-2026-05-29.md).
  HANDOFF → architect (M-T1 fast-skip + git-diff semantics
  ratification + Q-ADR-STATUS-ENUM clarification; parallel-safe
  with siblings).

- **operator-ledger-schema-lint v0.1.0** — Pick C Wave 1
  orchestrator hygiene compounder trio cheaper pillar (~0.5 dev
  day). Python-stdlib script `scripts/operator_ledger_check.py`
  upgrading `spec/dev-notes/operator-side-pending-ledger.md`
  (created 2026-05-29 per retro fix-improve #5) from convention to
  schema-enforced living document. Asserts (a) per-table schema
  (Pending / Done / Cancelled column lists); (b) status enum
  `{pending, FAILED, done, cancelled}` via markdown-tolerant
  normalization; (c) Done rows have completion date; (d) **stale-
  FAILED escalation** — FAILED rows > STALE_FAILED_DAYS (= 7)
  surface escalation reminder at session start; (e) FAILED rows
  require follow-up dev-note citation in Notes per Q-LED-NOTE (a)
  DURABLE strictest interpretation. Consolidates the chronic
  carry-over class (Bug #64 visual-verify, Yahoo bulk fetch,
  toast-queue smoke tests). Q-LED-WHEN (a) **DURABLE** orchestrator
  session pre-flight only (primary value is stale-FAILED escalation
  which fires at session start not commit). Q-LED-NOTE (a)
  **DURABLE** require dev-note citation on FAILED rows
  (structurally enforces investigation-on-failure pattern Bug #64
  D.1.1 sets precedent for). OWNS ledger frontmatter R3.3 amendment
  + AGENT.md R3.4 cross-reference per bundle K4 ownership-table.
  Append-only contract preserved (READ-ONLY on row bodies; only
  frontmatter touch). No new ADR; no Cargo touch. Zero anchor
  delta. Trace `REQ-OPERATOR-LEDGER-SCHEMA-LINT-001` proposed.
  Brief:
  [`spec/operator-ledger-schema-lint/feature.md`](operator-ledger-schema-lint/feature.md).
  Direction: [`spec/dev-notes/pick-c-orchestrator-hygiene-2026-05-29.md`](dev-notes/archive/2026-Q2/pick-c-orchestrator-hygiene-2026-05-29.md).
  HANDOFF → architect (M-T1 fast-skip + SCHEMA constant + Bug #64
  D.1.1 regression-case parse confirmation; parallel-safe with
  siblings).


<!-- updated 2026-05-28 (analyst, v3-regime-classifier M-A5 light-touch
     refresh — promoted Queue → Active per operator Phase 2 re-pick after
     v2.5 TCN re-investigation analyst-halt). Analyst agent
     a78dc46ac61e304ee API-aborted at tool 34 with substantial work done
     (feature.md narrowed from R1-R8/H1-H6/Q1-Q7 to canonical M0 shape
     R1-R5/K1-K6/H1-H4/Q1-Q5 + 4-cell verdict tree; 430 lines inserted,
     589 deleted); orchestrator inline-finished the Queue → Active move
     + trace state flip proposed→arch-ready.
     2026-05-28 architect M-T1 closed (this annotation update): ADR-0049
     authored + feature.md § Design populated + tasks.md Waves A-F locked
     + trace state arch-done. HANDOFF → developer Wave A. -->
<!-- RETIRED 2026-05-29 — Wave E empirical T-REG-NO-ALPHA + V-REG-5;
     operator R-O 2026-05-29 picked RETIRE. See Recent (shipped)
     2026-05-29 cohort for full retirement record. -->
<!-- - **v3-regime-classifier v0.1.0** — RETIRED 2026-05-29; see Recent. -->


<!-- updated 2026-05-28 (analyst, v5-latency-slippage-sim-v0.4.0-candle-feature-gated-re-emit
     M0 close). **PROMOTED Idea → Active 2026-05-28**. Closes the operator-
     approved v0.3.0 SOFT-PASS carve-out: 8 candle/realdata-feature-gated
     scenarios (TCN-weights ×2, TCN-realdata ×2, TCN-weights-realdata ×2,
     PatchTST-realdata ×1, VolTarget-GARCH-realdata ×1) whose v0.3.0 canonical
     SHAs remain noop-identical to their noop-baseline twins at spec/anchors.toml
     lines 121-155+242+272 because the default CI binary was built without
     `--features candle realdata`. v0.4.0 rebuilds on the canonical Apple
     Silicon box (per v2.5 TCN Metal-CPU-drift precedent) and re-emits all 8
     scenarios under canonical LatencySlippageSimConfig { 30, 80, 8 } (ADR-0045
     D1 unchanged). 8 SHAs update in-place under namespace
     `v5-realdata-medium-2026-05` (Q3=(a) same-pin precedent from v0.3.0);
     total anchor count stays 70/70. Pure rebuild + re-emit — no plumbing or
     engine changes (v0.3.0 Wave A discharged the per-path contract per
     ADR-0047 D2). 4 R / 4 K / 3 H / 2 Q + non-regression contract +
     pre-drawn 2-cell verdict tree + ~1-2 days wall-clock cost framing.
     Q1-Q2 standing-Autoapprove-eligible at analyst defaults (Q1=(a) Apple
     Silicon; Q2=(a) yes). M-T1 likely fast-skips (ADR-0047 carries forward);
     M-OD likely empty. K1 falsifier: canonical Apple Silicon box not
     available → route back with operator-decide on dropping the 4 realdata
     scenarios from the anchor set. Trace row `REQ-V5-LATENCY-SLIPPAGE-V0-4-0-001`
     opened at `proposed` state. HANDOFF → architect (M-T1 fast-skip
     ratifies + emits dev handoff). -->
<!-- moved to Recent (shipped) — v0.1.0 operator-approved 2026-05-28 -->
<!-- - **v5-latency-slippage-sim-v0.4.0-candle-feature-gated-re-emit v0.1.0** — see Recent section below. -->

<!-- updated 2026-05-29 (analyst, v5-latency-slippage-sim-v0.5.0-square-root-market-impact
     M0 close). **PROMOTED Idea → Active 2026-05-29** per operator-locked Phase 1 #3.
     Closes the v0.1.0 ADR-0043 § D3 deferred promise ("linear bps slippage at v0.1.0;
     defer square-root market impact to v0.2.0+"). Upgrades the linear-bps slippage
     model to the academic-canonical square-root market-impact form
     `cost = α · √(Q/V)` (Almgren & Chriss 2001; Kissell 2014 ch. 3). Per-asset V
     proxy sourced from existing Binance parquet (90-day trailing daily volume,
     revision-pinned via `data/binance/REVISION.toml` SHA `3a8b96…bfc7`). Re-emits
     all 19 currently-friction-real anchored scenarios under parallel namespace
     `v5-sqrt-impact-2026-05` (mirrors ADR-0045 D2 noop-vs-canonical twin pattern);
     preserves the linear-bps namespace `v5-realdata-medium-2026-05` as comparison
     oracle. Anchor cascade: 71 → 90 (additive — both models co-exist; the 71
     existing rows stay byte-identical per ADR-0038 § D6.a). 5 R / R-NR / 4 K /
     3 H / 3 Q + pre-drawn 2-cell verdict tree + cost framing both routes.
     **Apply new AGENT.md 2026-05-29 durable-over-quick contract**: Q1+Q2+Q3 all
     recommend the DURABLE option (Q1=(a) α=1.0 Kissell midpoint; Q2=(a) Binance
     parquet 90-day trailing; Q3=(a) Linear fallback for synthetic). Cheap
     fallbacks frame as STRICTLY WORSE wall-clock (~3 weeks + 3 follow-on
     briefs vs ~1 week one-shot). K1 falsifier: synthetic scenarios fall back to
     Linear { bps: 8 } (9 of 19 SHAs trivially byte-identical across namespaces).
     K2 falsifier: f64 conversion boundary for `√` over Decimal (architect M-T1
     locks the contract). K3 falsifier: thin-liquidity-hours saturation; cap
     `MAX_SLIPPAGE_BPS = 1_000` (10%). K4 falsifier: compound determinism
     candle × realdata × friction × sqrt — 2-run byte-identity required.
     H1: sqrt drag ≥ 2× linear drag on TCN-realdata. H2: sqrt drag ≈ linear
     on low-turnover (Pairs zscore-mr, VolTarget-GARCH). H3: 2-run determinism
     holds. **No UI surface change** (R-NR-UI). Trace row
     `REQ-V5-LATENCY-SLIPPAGE-V0-5-0-001` opened at `proposed` state.
     HANDOFF → operator-decide (Q1-Q3 all-DURABLE Autoapprove-eligible) →
     architect M-T1 for numerical-precision contract + per-asset volume
     retrieval shape + ADR amendment vs new ADR. -->
- **v5-latency-slippage-sim-v0.5.0-square-root-market-impact v0.1.0** — Closes the
  <!-- # noqa: queue-staleness — reconciled 2026-05-30: shipped 2026-05-29 + operator-approved; see Recent. Proper Active→Recent section move deferred to operator backlog-triage. -->
  v0.1.0 ADR-0043 § D3 deferred promise (linear-bps → square-root market-impact).
  Cost = `α · √(Q/V)` per Almgren & Chriss 2001; Kissell 2014. Per-asset V from
  existing Binance parquet 90-day trailing daily volume (revision-pinned, no new
  data source). Re-emits 19 friction-real scenarios under new namespace
  `v5-sqrt-impact-2026-05` parallel to existing `v5-realdata-medium-2026-05`
  (linear-bps stays as comparison oracle). Anchor cascade 71 → 90 (additive).
  3 Q all-DURABLE Autoapprove-eligible per AGENT.md 2026-05-29 contract.
  Brief: [`spec/v5-latency-slippage-sim-v0.5.0-square-root-market-impact/feature.md`](v5-latency-slippage-sim-v0.5.0-square-root-market-impact/feature.md).
  Trace: `REQ-V5-LATENCY-SLIPPAGE-V0-5-0-001` (state **`arch-done`** per architect
  M-T1 2026-05-29). HANDOFF → developer (Waves A–F). M-OD 2026-05-29 resolved
  Q1=(a) α=1.0 + Q2=(a) 90-day Binance parquet + **Q3=(b) MIXED universe-avg V
  on synthetic — operator override** (9 synthetic SHAs in `v5-sqrt-impact-2026-05`
  will diverge from `v5-realdata-medium-2026-05` linear-bps twins by-design;
  v0.6.0 sub-namespace cleanup commitment recorded — either split into
  `realdata`/`synthetic` sub-namespaces or consolidate around 10 real-sqrt + 9
  linear-synthetic). M-T1 ADR decision: **amend ADR-0043 § Changelog** (NOT new
  ADR-0050) — mirrors 2026-05-27 Murmur3 D2 amendment precedent. Cost ~1 week
  wall-clock (DURABLE route).

<!-- updated 2026-05-27 (analyst, lab-yahoo-realdata-v0.1.2-eth-usd-anchor-and-cache-badge
     M0 close) — **PROMOTED Idea → Active 2026-05-27** by operator multi-select
     option C against the v0.1.1 presenter deck open list: (1) lock ETH-USD as
     anchor row 70 of `spec/anchors.toml` under namespace `lab-yahoo-realdata-v0.1.2`
     (mirrors v0.1.1 BTC pattern at row 69, body SHA 8045623b…); (2) ship the
     deferred T-D2 cache-state SUMMARY badge — an AGGREGATE multi-ticker indicator
     that COMPLEMENTS (does not replace) the per-pair Fresh/Stale/Empty pill
     already shipped at v0.1.0 Wave D-followup. Q1 load-bearing: extend
     `run_yahoo_sma.rs` with `--ticker` flag (analyst-recommended) vs add parallel
     `run_yahoo_sma_eth.rs` binary; 15-LoC delta vs +250 LoC, scales DRY to 8
     future tickers, anchor preservation trivially proven by default-arg
     invariance (H3: BTC SHA `8045623b…` byte-identical when `--ticker` omitted).
     Q2 + Q3 ui-designer placement + content choices (Autoapprove-eligible at
     analyst defaults: source-toggle row sibling + middle-ground copy "Cache: N
     tickers · last YYYY-MM-DD"). **5 R / 4 K / 4 H / 3 Q** + non-regression
     contract (zero new design tokens, exactly 1 new string) + cost framing
     (~4-6 hours wall-clock; dev + ui-designer parallelizable, zero file
     overlap) + pre-drawn 2-cell verdict tree. K1 fallback: if Binance ETHUSDT
     reference data is missing/stale at M-DEV, route back to analyst with
     operator-decide on synthetic-comparison fallback. Trace row
     `REQ-LAB-YAHOO-REALDATA-V0-1-2-001` opened at `proposed` state. Frontmatter
     stays `owner: analyst`, `status: draft` — operator-decide is next.
     HANDOFF → architect (M-T1 ratifies Q1/Q2/Q3 + decomposes M-DEV + M-DEV-UI). -->
<!-- moved to Recent (shipped) — v0.1.0 operator-approved 2026-05-28 -->
<!-- - **lab-yahoo-realdata-v0.1.2-eth-usd-anchor-and-cache-badge v0.1.0** — see Recent section below for v0.1.0 ship summary. -->

<!-- updated 2026-05-28 (analyst, lab-yahoo-realdata-v0.1.3-rev-frontmatter-and-binance-eth-h1
     M0 close) — **PROMOTED Idea → Active 2026-05-28**. Closes the 2 architect-
     flagged design notes from v0.1.2 M-FINAL (SOFT-PASS verdict, test-final
     2026-05-28 report): (1) `rev=<sha>` substring in `run_yahoo_sma.rs:259`
     "Data source:" body line couples body-SHA stability to REVISION.toml
     aggregate — every operator-initiated `fetch_yahoo_klines` invocation drifts
     the BTC anchor SHA (pattern guaranteed to recur for BNB/SOL/XRP/… fetches
     at v0.1.4+); (2) H1 was discharged at v0.1.2 via K1 fallback (Yahoo ETH
     vs Yahoo BTC same-window 0.84%) because no `eth-2024-h1-sma-cross` Binance
     scenario was registered — Binance ETHUSDT 2024 parquets exist (12 files
     confirmed by v0.1.2 tester) but `crates/backtest/src/main.rs` has no arm.
     Scope: (R1) move `rev=` from body → front-matter `revision_sha:` via a
     canonical Yahoo report-emit helper (Q1=(a) [Recommended — DURABLE per
     AGENT.md 2026-05-28]; +1 day at v0.1.3 vs Q1=(b) inline-fix but -1.5 to -3
     days follow-on across v0.2.0+ MACD/RSI/BBands emitters); (R2) register
     `eth-2024-h1-sma-cross` Binance hourly scenario mirroring existing
     `btc-2024-h1-sma-cross` arm at L242; (R3) anchor cascade 70 → 71 (BTC row
     69 SHA in-place update under existing namespace `lab-yahoo-realdata-v0.1.1`
     per Q2=(a) [Recommended — DURABLE]; row 70 ETH daily byte-identical; new
     row 71 `eth-2024-h1-sma-cross` under namespace `lab-yahoo-realdata-v0.1.3`);
     (R4) H1 ETH direct re-discharge (Yahoo daily vs Binance hourly, threshold
     30%, expected 5-15%); (R-NR) zero new design tokens / strings.rs adds —
     this is backend hygiene + scenario registration only. **5 R / 4 K / 3 H /
     2 Q** + non-regression contract + pre-drawn 2-cell verdict tree + cost
     framing (~1.5-2 days Q1=(a) Recommended). M-T1 likely fast-skips
     (ADR-0040 § Changelog amendment only, per D-V0.1.2-6 ADR-extend-not-new
     precedent; no new ADR). M-DEV-UI lane DOES NOT EXIST at v0.1.3 (backend-
     only ship). K1 falsifier: if a forgotten Yahoo emitter exists that also
     embeds `rev=`, route back to architect for migration-scope expansion.
     Trace row `REQ-LAB-YAHOO-REALDATA-V0-1-3-001` opened `proposed`.
     Frontmatter stays `owner: analyst`, `status: draft` — M-OD is next.
     HANDOFF → architect (M-T1 ratifies Q1/Q2 + decomposes M-DEV waves A-D). -->
<!-- moved to Recent (shipped) — v0.1.0 operator-approved 2026-05-29 -->
<!-- - **lab-yahoo-realdata-v0.1.3-rev-frontmatter-and-binance-eth-h1 v0.1.0** — see Recent section below. -->

<!-- updated 2026-05-28 (architect, lab-recipe-test-harness M-T1 close — analyst
     pass folded in per orchestrator brief since the WHAT was already
     documented in spec/bug-log.md#64 attempt-1 post-mortem + dev-note
     bug-64-progress-bar-investigation-2026-05-27.md). **PROMOTED Idea →
     Active 2026-05-28** as a P1 architect-led tooling investment to close
     the testing gap exposed by the Bug #64 D.1.1+D.2.1 revert at commit
     `e94615e` (revert hash; ship was `5f9f920`). Three live regressions
     escaped 415 PASS + 70/70 anchors + K5 5/5 because no test exercises
     the channel/subscription flow of `runner::spawn_lab_run` (boundary
     tests stop at `run_scenario`; pure-state `LabState` invariants stop
     at message arms). This brief ships two test surfaces totaling ~200
     LoC: (1) `spawn_lab_run_yahoo_harness.rs` with `MockYahooBarSource`
     covering sentinel emission + `tokio::select!` channel survival
     (categories A + B); (2) `lab_stop_button_gating.rs` covering the
     view-gating predicate `model.lab_run_inflight` lifecycle (category
     C). Harness pattern (d) Combination per ADR-0048 D1; production
     binary path API-additive only (extract `pub trait YahooBarSource`).
     Anchor-additive zero per D6 — channel-only events, no file output,
     anchors stay 70/70. **GATES the Bug #64 re-attempt** — no polish
     work may ship until this lands and proves it catches the three
     regression classes via the M-FINAL T-T4 falsification probe.
     M-OD is empty (architect locked all decisions at sensible defaults).
     Trace row `REQ-LAB-RECIPE-TEST-HARNESS-001` opened at `arch-done`
     state. HANDOFF → developer (T-D1 → T-D6 sequential; tester closes
     loop with falsification-probe evidence). -->
<!-- moved to Recent (shipped) — v0.1.0 operator-approved 2026-05-28 -->
<!-- - **lab-recipe-test-harness v0.1.0** — see Recent section below for v0.1.0 ship summary. -->

<!-- updated 2026-05-29 (analyst, lab-recipe-test-harness-v0.2.0-cross-surface-extension
     M0 close) — **PROMOTED Idea → Active 2026-05-29** under the new
     durable-over-quick contract (AGENT.md 2026-05-29: `(Recommended)` rewards
     DURABLE, not cheap). v0.1.0 (shipped 2026-05-28) proved the two-surface
     harness pattern catches the channel-survival + state-gating regression
     class Bug #64 surfaced — but only on `spawn_lab_run` (S1) and
     `lab_run_inflight` (S2). R1 inventory enumerates 9 Recipe / aggregator
     surfaces across `crates/ui/` + `crates/agent/`; FOUR are completely
     uncovered for at least one of {S1 boundary, S2 gating}:
     `TrainingLogRecipe` (exact Bug #64 shape — `Arc<Mutex<Option<_>>>::take()`
     + per-run salt over `std::mpsc::Receiver<TrainingLogLine>`),
     `TrainingPoller` (audit-DB-poll subscription with run-id identity gate),
     `ToastDismissRecipe` (always-on `tokio::time::interval` — zero tests),
     `ActivityAuditAggregator` (`tokio::select!` rx + 100 ms interval — same
     two-arm shape as Bug #64). Plus partial extensions: ServerTime S2 +
     TrailMirror S2 + Activity S1. Mirror v0.1.0 file-layout (~80 LoC per
     Recipe = ~600 total). Zero new ADRs (ADR-0048 D1-D6 carries forward);
     zero design tokens; zero `strings.rs` adds; anchor-additive 71/71 stable.
     K1-K4 falsifiers + H1-H4 hypotheses + Q1-Q2 operator-decide framed
     durable-recommended (Q1=(a) all 4 + extras ~1 week dev + 1 day tester
     vs Q1=(b) TrainingLog only ~2-3 days + ~3-5 days v0.3.0 deferred;
     Q2=(a) per-Recipe T-T4 falsification proof — mandatory per v0.1.0 lesson
     "prove it or it's theater" — vs Q2=(b) no proof, rejected). 4-cell
     pre-drawn verdict tree. M-T1 ratifies Q1+Q2 + R3 single-trait-vs-per-Recipe
     mock pattern + decomposes M-DEV per-Recipe waves A-F. Trace row
     `REQ-LAB-RECIPE-TEST-HARNESS-V0-2-0-001` opened `proposed`. Frontmatter
     stays `owner: analyst`, `status: draft` — operator-decide is next.
     HANDOFF → architect (M-T1). -->
- **lab-recipe-test-harness v0.2.0 — cross-surface extension v0.1.0** —
  Durable-coverage follow-up to v0.1.0 (shipped 2026-05-28). v0.1.0 proved
  the two-surface harness pattern (boundary-test + state-gating) catches
  the channel-survival + state-gating regression class Bug #64 surfaced
  on `spawn_lab_run`. R1 inventory found 4 other Recipe / aggregator
  surfaces in `crates/ui/` + `crates/agent/` with the EXACT same shape
  and zero boundary/gating coverage: `TrainingLogRecipe` (Bug #64 `take()`
  + salt shape; HIGHEST URGENCY), `TrainingPoller`, `ToastDismissRecipe`,
  `ActivityAuditAggregator` (Bug #64 `tokio::select!` shape). v0.2.0
  extends the proven pattern PREEMPTIVELY before the next regression
  lands. Per the new durable contract: ~1 week dev + 1 day tester now
  beats ~3-5 days deferred v0.3.0 cleanup plus 1-2 visual-verify revert
  windows. Zero new ADRs (ADR-0048 D1-D6 carries forward). Zero anchor
  delta (71/71 byte-identical). Brief:
  [`spec/lab-recipe-test-harness-v0.2.0-cross-surface-extension/feature.md`](lab-recipe-test-harness-v0.2.0-cross-surface-extension/feature.md).
  Trace: `REQ-LAB-RECIPE-TEST-HARNESS-V0-2-0-001` (state `arch-done` as
  of 2026-05-29). **ARCH M-T1 done 2026-05-29**: D-V0.2.0-1 per-Recipe
  mocks (NOT shared trait); D-V0.2.0-2 mock inventory pinned
  (`MockTrainingLogChannel` / `MockAuditTickBus` / `MockTrailMirrorHandle`
  / `MockActivityBus` / `MockAuditLedger`); D-V0.2.0-3 11-row T-T4
  falsification probe line table; D-V0.2.0-4 ADR-0048 carries forward
  (one Changelog row, no new ADR); D-V0.2.0-5 Wave A→D dependency-ordered
  (A‖B parallel — TrainingLog + ActivityAuditAggregator; C extracts
  `SubscriptionBatchDescriptor` seam — ServerTime S2 + ToastDismiss S1+S2;
  D depends on C — TrailMirror S2 + Activity S1 + TrainingPoller S1+S2);
  ~940 LoC tests + ~130 LoC src deltas across 3 API-additive extractions;
  ~6 dev days + 1 tester day. HANDOFF → developer (Wave A start —
  TrainingLogRecipe is exact Bug #64 shape, highest urgency).

<!-- updated 2026-05-27 (analyst, cockpit-toast-queue M0 close — inline-salvaged
     after analyst agentId a43a615341bd60112 529'd at 26 tool uses;
     orchestrator appended this backlog row + trace row REQ-COCKPIT-TOAST-QUEUE-001
     at EOF since both were missing from the 529'd run). -->
<!-- moved to Recent (shipped) — v0.1.0 operator-approved 2026-05-27 -->
<!-- - **cockpit-toast-queue v0.1.0** — see Recent section below for v0.1.0 ship summary. -->

<!-- updated 2026-05-27 (analyst, v5-latency-slippage-sim-v0.2.0-anchor-migration
     M0 close — inline-salvaged after analyst agentId ac4d192d801af160a 529'd at
     14 tool uses; orchestrator wrote tasks.md + appended this backlog row +
     trace row REQ-V5-ANCHOR-MIGRATION-V0-2-0-001 at EOF). -->
<!-- moved to Recent (shipped) — v0.1.0 operator-approved 2026-05-27 -->
<!-- - **v5-latency-slippage-sim-v0.2.0-anchor-migration v0.1.0** — see Recent section below for v0.1.0 ship summary. -->

<!-- updated 2026-05-27 (analyst, v5-latency-slippage-sim-v0.3.0-full-path-wiring
     M0 promotion Queue → Active). Closes the operator-approved v0.2.0
     Ship Route (a) partial-migration follow-on commitment: (1) wires
     canonical LatencySlippageSimConfig into the 6 strategy construction
     sites v0.2.0 missed (SmaComposed, TcnOverlay, PatchTstOverlay, Pairs,
     VolTargetOverlay, GarchVolOverlay) so their canonical SHAs stop being
     byte-identical to noop-baseline; (2) resolves the LOAD-BEARING Group A
     data-source question — accept real-Binance baseline as new oracle epoch
     OR revert to synthetic baseline (Q1 = HARD operator-decide, no safe
     analyst default); (3) flips the v0.2.0-whitelisted t1937_nine_strategy_
     anchors_unchanged test to GREEN via namespace-aware resolver mirroring
     verify_anchors.sh. 6 R / 6 K / 4 H / 5 Q + non-regression contract +
     pre-drawn 4-cell verdict tree + cost framing. Cost ~3-5 days wall-clock.
     Trace row REQ-V5-FULL-PATH-WIRING-001 opened at `proposed` state. -->
<!-- moved to Recent (shipped) — v0.3.0 operator-approved 2026-05-27 -->
<!-- - **v5-latency-slippage-sim-v0.3.0-full-path-wiring v0.1.0** — see Recent section below for v0.3.0 ship summary. -->

<!-- updated 2026-05-26 (analyst + architect, v5-latency-slippage-sim M0 + M-T1 close) —
     **PROMOTED Idea → Active 2026-05-26** by operator directive: "New
     feature track to close the gap between backtesting and live
     execution." Closes the well-known backtest-vs-live alpha
     overestimation gap by simulating two real-world frictions in
     `crates/exec` + `crates/cost`:
     - Network latency (uniform jitter range, default 0..=0 ms)
     - Linear-bps order-book slippage (default 0 bps)

     **Default = noop**, so the 34 SHA-256 anchors in
     `spec/anchors.toml` stay byte-identical at v0.1.0 ship. Anchor
     migration to non-zero values is deferred to a separate v0.2.0
     brief per Q5 analyst-recommended default.

     **CLAUDE.md non-negotiable honored**: ships with a baseline-
     equity-divergence e2e test (R5) at M-DEV Wave E per the v3-
     volatility-forecaster-noop-fix 2026-05-22 precedent.

     **5 R / 7 K / 4 H / 5 Q** + non-regression contract + cost framing
     + pre-drawn 4-cell verdict tree. Cost ~1.5-2 weeks wall-clock
     (analyst 0.5d + architect 0.5d + dev Waves A-E 5-8d + tester 1d
     + presenter 0.5d).

     **5 sub-decisions locked by ADR-0043** (NOT 0040 as the operator
     brief suggested — 0040 is taken by yahoo-realdata-path; 0041 by
     trader-crate-split; 0042 by cockpit-activity-broadcast; 0043 is
     the next free number):
     - D1 always-on code path with default-zero noop (rejects Cargo
       feature flag)
     - D2 seeded `ChaCha20Rng` sub-stream keyed on
       `(scenario_seed, order_id)` for replay determinism
     - D3 linear bps slippage at v0.1.0; defer square-root market
       impact to v0.2.0
     - D4 NEW `AuditEvent::SimulatedExecMetrics` variant with skip-
       when-zero guard
     - D5 backtest-only scope; live mode untouched

     **Wave plan**: Wave A configuration toggle (FIRST task — CRITICAL
     anchor gate at T-D-N2) blocks Waves B-E. Wave B latency in
     `crates/exec`; Wave C slippage in `crates/cost` (B ‖ C parallel);
     Wave D audit-event variant; Wave E e2e divergence test +
     criterion bench (perf < 1%) + non-regression contract.

     **K5 sequencing constraint**: vol-killswitch-overlay-noop-fix
     Bug #65 Q4=(p3) developer is in flight as of 2026-05-26. Both
     briefs touch `crates/strategy/` + cost-modifier semantics.
     Sequence: Bug #65 lands first; this brief's developer rebases
     after.

     Trace row `REQ-V5-LATENCY-SLIPPAGE-001` opened at `proposed`
     state. HANDOFF → operator-decide (Q1-Q5 standing-Autoapprove-
     eligible at analyst defaults) → developer Wave A. -->

<!-- moved to Recent (shipped) — v0.1.0 operator-approved 2026-05-27 -->
<!-- - **v5-latency-slippage-sim v0.1.0** — see Recent section below for v0.1.0 ship summary. -->

<!-- updated 2026-05-26 (analyst, cockpit-activity-audit-ledger-producer M0 close) —
     **PROMOTED Idea → Active 2026-05-26** as the v0.1.1 follow-on slot
     opened by `cockpit-activity-status-bar v0.1.0` (shipped 2026-05-26).
     v0.1.0 R5.2 + K3 explicitly deferred the
     `ActivityKind::AuditLedgerWrite` producer because audit-ledger
     writes fan-out at thousands/sec during a fast backtest — the
     per-event 100 ms `ActivityHandle::tick` throttle is the wrong place
     to enforce aggregation. This brief picks the aggregation policy.

     **Architecture insight surfaced by the M0 pass**: the existing
     `crates/audit/src/tick.rs` `AuditTick<AuditEvent>` broadcast
     already tees every post-commit writer in `journal.rs` (7-8 call
     sites: `post_fill`, `post_strategy_signal`, `kill_switch_tripped`,
     `strategy_event`, `forecast_emitted`, `uptime_*`,
     `llm_forecast_emitted`). The aggregator subscribes to that bus —
     **ZERO changes to `crates/audit/`**. Aggregator lives in NEW
     `crates/agent/src/activity_audit.rs` sibling of v0.1.0's
     `activity.rs`. ~150 LOC; AtomicU32 counter + 100 ms
     `tokio::time::interval` + long-lived `ActivityHandle` with
     idle-end semantics.

     **Three operator-decide Qs**, all Autoapprove-eligible at the
     analyst-recommended defaults:
     - **Q1 — aggregation policy**: (a) per-batch / **(b)
       per-time-window 100 ms — ANALYST DEFAULT** (aligns with the
       existing 100 ms `ActivityHandle::tick` throttle from v0.1.0
       R1.4 — same cadence) / (c) per-entity (one handle per
       `AuditEvent` variant; 9 variants today blow the status-bar
       max-3-visible budget).
     - **Q2 — label content**: **(a) redacted "Audit: N writes" —
       ANALYST DEFAULT** (PII-safe; the rate-of-writes is already
       observable via existing metrics counters; the operator drills
       into the actual ledger for detail) / (b) verbose
       "Audit: KillSwitchTripped" (PII leak via screenshot vector;
       hard-veto) / (c) kind-mix summary (forward-listed to v0.2.0).
     - **Q3 — failure handling**: **(a) continue aggregator + spawn
       sibling Failed-event ActivityHandle — ANALYST DEFAULT** (the
       successful writes stay green; failures get the red 3 s hold
       per parent R2.5) / (b) flip the aggregated handle to Failed on
       any inner write error (misleading — taints the green ones).

     **R5 performance gate (K3-discharge)**: criterion bench
     `aggregator_overhead_per_tick` (< 100 ns/tick budget) +
     anchor-replay parity bench on the
     `top10-2024-fy-momentum-bs1` anchor (< 1 % wall-clock
     divergence at p99 WITH vs WITHOUT aggregator). MUST pass
     before tester M-FINAL.

     **Anchor risk zero by construction** — UI + agent additive only;
     no backtest / strategy / audit changes. 34/34 anchors
     byte-identical.

     **Cost**: ~2-3 days end-to-end wall-clock. Wave A (aggregator
     ~0.5d) → Wave B (UI label + spawn-site ~0.5d) → Wave C (perf
     gates ~1d) → Wave D (storm + flood-mitigation ~0.5d).

     **Trace row**: `REQ-COCKPIT-ACTIVITY-AUDIT-LEDGER-001` at
     `proposed` state.
     **Feature folder**:
     [`spec/cockpit-activity-audit-ledger-producer/`](cockpit-activity-audit-ledger-producer/feature.md).
     **ADR sketch**: ADR-NNNN (audit-ledger activity aggregator —
     number assigned at architect M-T1 against ADR registry, likely
     0042+). HANDOFF → architect for M-T1 decomposition. -->

<!-- moved to Recent (shipped) — v0.1.0 operator-approved 2026-05-27 -->
<!-- - **cockpit-activity-audit-ledger-producer v0.1.0** — see Recent section below for v0.1.0 ship summary. -->


<!-- updated 2026-05-24 (analyst, lab-yahoo-realdata) —
     **PROMOTED Idea → Active 2026-05-24** under the operator's
     2026-05-24 decision "Replace Binance for Lab — multi-asset
     pivot." This brief sits downstream of two predecessors: (i)
     `backtest-real-binance-data v0.1.0` (shipped 2026-05-18) which
     locked the parquet revision-pin protocol per ADR-0032; (ii)
     `lab-end-to-end-v2 v0.1.0` Wave D-2 (shipped 2026-05-24) which
     extracted single-symbol dispatch arms (`v0.sma`, `v0.5.macd`,
     `v0.5.rsi`, `v0.5.bbands`) into `engine::run_scenario`, making
     the Lab's "pair × strategy" UX real for single-symbol strategies.
     This brief swaps the underlying data source for that UX from
     synthetic GBM to Yahoo-Finance-cached historical OHLCV.

     **What this brief does NOT do**: the 34 locked anchors stay
     byte-identical (Binance CLI path stays in tree for anchor
     reproducibility; Lab simply stops dispatching to it). New
     Yahoo-based anchors emit at a future v0.1.1 M-FINAL after an
     operator-approved baseline.

     **Cost framing**: ~1-2 weeks wall-clock. Wave C splits into 4
     independent sub-waves:
     - C-1 (`YahooBarSource` + cache) — ~3 days, no UI surface;
     - C-2 (`fetch_yahoo_klines` CLI) — ~1 day, depends on C-1;
     - C-3 (Lab dispatch wiring) — ~3 days, parallel with C-1 + C-4;
     - C-4 (`Venue::Yahoo` cascade) — ~1 day, gates C-3 UI work.
     Wave D (ui-designer) runs parallel with C-3 once `LabSource`
     enum lands.

     **Operator-decide Q's** (load-bearing — architect M-T1 gates on
     Q1, Q2, Q4, Q6):
     - Q1 — engine-dispatch shape: (a) yahoo-specific arms vs
       (b) source-agnostic engine + Lab-side bar swap. Analyst
       recommends **(b)** — minimum anchor risk.
     - Q2 — asset universe scope: (a) crypto-mirror only vs
       (b)-(d) equities/FX/commodities expansion. Analyst
       recommends **(a)** at v0.1.0; (b)-(d) are one-week
       follow-ups each.
     - Q3 — Yahoo crate pick: (a) `yahoo_finance_api 4.1.x` vs
       (b) `yfinance-rs 0.7.x` vs (c) custom HTTP. Analyst
       recommends **(a)**.
     - Q4 — cadence policy: (a) hourly only vs (b) daily only vs
       (c) adaptive. Analyst recommends **(c)** with badge UI.
     - Q5-Q10 — strategy params, ticker convention, cache backend,
       in-cockpit fetch button, coverage threshold, git-tracking.
       Analyst defaults documented in feature.md.

     **Cross-feature impact**: `lab-end-to-end-v2` D-2c "Binance
     parquet → Lab wiring" is SUPERSEDED by this brief per the
     operator's decision; the v2 spec gains an explicit
     SUPERSEDED note (analyst T-A9 in this M0 pass).

     **Trace row**: `REQ-LAB-YAHOO-REALDATA-001` at `proposed` state.
     **Feature folder**:
     [`spec/lab-yahoo-realdata/`](lab-yahoo-realdata/feature.md).
     **ADR sketch**: ADR-0040 (Yahoo realdata path + revision pin) —
     architect authors at M-T1 from the analyst outline in
     feature.md.
-->

<!-- moved to Recent (shipped) — v0.1.0 operator-approved 2026-05-24 -->
<!-- - **lab-yahoo-realdata** — see Recent section below for v0.1.0 ship summary. -->

<!-- moved to Recent (shipped) — v0.1.1 operator-approved 2026-05-27 -->
<!-- - **lab-yahoo-realdata v0.1.1** — see Recent section below for v0.1.1 ship summary. -->

<!-- updated 2026-05-26 (analyst, vol-killswitch-overlay-noop-fix) —
     **PROMOTED Idea → Active 2026-05-26** under Bug #65 (P0 safety
     wiring-bug recovery). Sibling of the shipped
     `v3-volatility-forecaster-noop-fix v0.1.0` 2026-05-22 — same
     no-op pattern; `crates/strategy/src/vol_killswitch_overlay.rs`
     increments its `kill_switch_count` counter correctly when the
     trigger condition fires, but the `Signal::kind = Hold` mutation
     never reaches the executor's load-bearing path for the
     cross-sectional momentum inner strategy. Equity matches the
     un-overlaid baseline byte-for-byte. **Severity P0 safety**: a
     killswitch that doesn't kill is the worst kind of no-op — in
     production, vol spikes above the threshold show "kill-switch
     tripped" in metrics, but the executor takes the position anyway.

     **Discovery**: Wave 1's overlay-e2e hygiene gate
     (`crates/strategy/tests/vol_killswitch_overlay_end_to_end.rs`)
     surfaced the bug 2026-05-26. Two tests fail correctly detecting
     the no-op: `trigger_fires_and_equity_diverges` (0 bp divergence
     vs ≥1 bp required) + `post_trigger_signals_are_hold` (zero
     Hold signals on the trigger bar). Both are `#[ignore]`-gated
     with `tracked-in: bug-log #65` annotations; the developer
     un-ignores at M-DEV after the fix lands. Negative control
     (`passthrough_when_threshold_unreachably_high`) passes — so
     the trigger path is the broken one.

     **Smoking gun**: lines 229-244 of `vol_killswitch_overlay.rs`
     mutate `sig.kind = SignalKind::Hold` BUT gate the mutation on
     `if sig.symbol == bar.symbol`. The inner strategy is
     `MomentumStrategy` (cross-sectional momentum), which emits
     signals for the BASKET at rebalance time, not for the trigger
     bar's symbol. Analyst's H1 (85% confidence): the per-signal
     symbol filter is the bug. Architect runs a 5-minute
     `tracing::warn!` probe at M-T1 to falsify H1 before locking
     the fix shape.

     **Scope at v0.1.0**: wire-up fix at the strategy → executor
     handoff in `vol_killswitch_overlay.rs` (~10-20 LoC, single
     file, single method body). Q1=(i) `Signal::kind = Hold`
     mutation widened to cover the basket — kill-switch semantic is
     binary, not scalar; smallest blast radius (cf the precedent's
     Q1=(ii) trait method, which was needed for vol-target's scalar
     semantic but not for kill-switch's binary semantic). Q2=(a)
     ZERO new anchors (grep on `spec/anchors.toml` for vol_killswitch
     returns zero matches; anchor risk ZERO by construction).
     Q3=(a) defer the `Strategy::dampen_signals` trait surface
     decision to v0.1.1+ once a second consumer surfaces.

     **What this brief does NOT do**: touch GARCH math, change the
     trigger condition arithmetic, change the cooldown semantic,
     touch `MomentumStrategy::on_bar`, change `trading_core::Signal`
     shape, add a new trait method to `Strategy`, add new anchored
     scenarios, touch any audit / reflection / executor crate. 34
     anchors stay byte-identical; pure strategy-internal patch.

     **Cost framing**: ~1-3 days end-to-end wall-clock. Analyst pass
     ~0.5 day (this); operator-decide ~30 min (standing Autoapprove);
     architect M-T1 ~0.5 day (H1 probe + fix shape lock); developer
     M-DEV ~1 day (single Wave A with forensic-gate FAIL/PASS
     bracket per the precedent T-D-N3a/3b protocol); tester M-FINAL
     ~0.5 day; presenter ~0.5 day. No LLM costs; pure source patch.

     **Operator-decide Q's** (3 surfaced; all standing-Autoapprove-
     eligible per the v3-volatility-forecaster-noop-fix precedent):
     - Q1 — fix shape: (i) mutate `Signal::kind = Hold` (smallest
       blast radius; kill-switch is binary) vs (ii) add a new
       `Signal::quantity_scale` field (rejected — over-engineered for
       binary semantic) vs (iii) add `Strategy::dampen_signals`
       defaulted trait method (rejected — YAGNI). Analyst recommends
       **(i)**.
     - Q2 — anchor handling: (a) zero new anchors (analyst-
       recommended) vs (b) add 1-2 anchored killswitch scenarios at
       v0.1.0 (deferred to v0.1.1+) vs (c) tighten e2e to drive
       top10 universe (rejected — over-scoped). Analyst recommends **(a)**.
     - Q3 — `Strategy::dampen_signals` trait surface: (a) defer to
       architect M-T1 (YAGNI) vs (b) add at v0.1.0 (rejected) vs
       (c) richer `signal_kind_override` surface (rejected). Analyst
       recommends **(a)**.

     **Pre-drawn verdict routing tree** (presenter inherits):
     - R-O1 — fix works; all 3 e2e tests PASS; no anchor delta → **SHIP**.
     - R-O2 — fix works but unexpected anchor delta → operator-decide HOLD.
     - R-O3 — fix doesn't work (H1 was the wrong root cause) →
       architect re-spawn; cost widens to 3-5 days.

     **Trace row**: `REQ-VOL-KILLSWITCH-NOOP-FIX-001` at `proposed`
     state (appended at END of `spec/trace.toml`; does NOT modify
     any existing row; parallel architect owns
     REQ-REFLECTION-TRADER-001 at line 1084).
     **Feature folder**: [`spec/vol-killswitch-overlay-noop-fix/`](vol-killswitch-overlay-noop-fix/feature.md).
     **Bug log**: [`spec/bug-log.md` § #65](bug-log.md).
     **Predecessor**: `v3-volatility-forecaster-noop-fix v0.1.0`
     (shipped 2026-05-22; sibling fix for vol_targeting_overlay
     under the same no-op pattern).
-->

<!-- moved to Recent (shipped) — v0.1.0 fix landed 2026-05-26 (bug-log #65); orchestrator retroactively closed spec drift 2026-05-27 during audit-2026-05-27 P0 triage. -->
<!-- - **vol-killswitch-overlay-noop-fix v0.1.0** — see Recent section below for v0.1.0 ship summary. -->

<!-- updated 2026-05-25 (architect, cockpit-activity-status-bar) —
     **PROMOTED Idea → Active 2026-05-25** under operator request
     2026-05-25: "Status bar should show all the current steps the
     cockpit is doing — downloading data, backtesting, everything
     else which could be helpful for the UI user to understand
     what's going on in background." Brief authored by architect
     in M0 sweep (rare — analyst pass skipped because the
     operator request is already an unambiguous extension of three
     shipped surfaces: `cockpit-training-control v0.2.0` R3.5
     status strip, `lab-end-to-end-v2 v0.1.0` Wave D-4 progress
     channel, `lumen-phase-1-foundation` R13 24 px bottom status
     bar contract).

     **Scope at v0.1.0**: aggregate activity tape inside the
     existing 24 px bottom status bar, surfacing three operator-
     cited slow producers — Yahoo preload (30-60 s cold cache),
     Lab Run (long backtest), Training subprocess (multi-minute
     train_tcn). Event source: new `EventBus::activity_tx`
     broadcast channel on the agent crate (capacity 256, mirrors
     existing 9-channel pattern). UI consumer: new
     `ActivityRecipe` subscription + new `widgets/activity_tape`
     region rendered between server-time and CPU placeholder.
     RAII `ActivityHandle` Drop semantics catch panic-unwinds
     ("Failed: dropped" rows in red 3 s hold). Producer-side
     100 ms throttle prevents flooding. Q1=(a) EventBus / Q2=(a)
     bottom bar / Q3=(a) stack max-3 + "+N more" / Q4=(a) < 1 ms
     render budget + criterion bench / Q5=(a) red 3 s hold /
     Q6=(a) read-only / Q7=(a) producer+consumer throttle /
     Q8=(a) Yahoo+Lab+Training all default-recommended.

     **What this brief does NOT do**: touch backtest / strategy /
     exec / risk / forecast / reports / audit / data / replay-
     cache / core / cost / models bodies. UI + agent only. The
     34 locked anchors stay byte-identical (R-NR.1, zero scenario-
     body changes by construction). No new audit migration; no
     persistence; no subprocess / IPC. No new Lumen tokens (reuses
     ACCENT, DANGER, FG_3). LLM-call producer + audit-ledger-
     writes producer are forward-listed and explicit OUT-of-scope
     at v0.1.0 (defer to v0.1.1 once `v3-llm-forecaster` ships +
     audit-writer aggregator design lands respectively).

     **Cost framing**: ~1 week wall-clock. Wave A (`crates/agent`
     bus + RAII handle) ~1 day; Wave B (`crates/ui` tape state +
     recipe + widget) ~2 days, parallel with Wave C (R4 producer
     wiring at three call sites) ~1 day; Wave D (criterion bench
     + integration storm test) ~1 day; tester M-FINAL ~0.5 day;
     presenter ~0.5 day. Rollback cost ~ 60 LOC across 4-5 files.
     Anchor risk ZERO by construction.

     **Operator-decide Q's** (8 surfaced; Q1+Q2+Q4 load-bearing,
     all 8 standing-Autoapprove-eligible at analyst-recommended
     defaults). See feature.md § Open questions.

     **Trace row**: `REQ-COCKPIT-ACTIVITY-001` at `proposed`
     state.
     **Feature folder**:
     [`spec/cockpit-activity-status-bar/`](cockpit-activity-status-bar/feature.md).
     **Predecessor chain**:
     `lumen-phase-1-foundation` (R13 24 px status-bar contract) →
     `cockpit-training-control v0.2.0` (R3.5 train status strip,
     audit poll precedent) →
     `lab-end-to-end-v2 v0.1.0` (Wave D-4 progress channel) →
     `cockpit-activity-status-bar v0.1.0` (this brief, aggregates
     all three into a global tape).
-->

<!-- Moved Active → Recent 2026-05-26 — operator approval "I approve, next".
     v0.1.0 shipped; commit chain 4248c00 → 0ff402f → e4f39ed → c1597ec.
     Trace row REQ-COCKPIT-ACTIVITY-001 state = passed.
     Open v0.1.1 follow-ons: LLM call producer (rides v3-llm-forecaster);
     TrainingPressed e2e wiring; cockpit-smoke operator-manual capture;
     pre-existing backtest::engine.rs:539 clippy::map_unwrap_or sweep;
     spec/trace.toml anchors-string char-iteration artifact fix. -->
- **cockpit-activity-status-bar v0.1.0** — SHIPPED 2026-05-26 (operator
  approval "I approve, next"; deck at
  [`spec/cockpit-activity-status-bar/presentations/cockpit-activity-status-bar-2026-05-26.md`](archive/presentations-2026-Q2.tar.gz)).
  Activity tape live in bottom status bar; 3 producers (Yahoo / Lab Run
  / Training). 34/34 anchors byte-identical. 31 new tests + 5 criterion
  benches + 4 insta baselines. See Recent section below.

<!-- updated 2026-05-26 (analyst, subscription-pipe-server-time-template) —
     **PROMOTED Idea → Active 2026-05-26** as a Wave-1 follow-on closing
     the operator's 2026-05-26 ServerTimeRecipe carve-out. The
     subscription-pipe test class template landed for `LabProgressRecipe`
     ([`crates/ui/tests/lab_progress_recipe_stream.rs`](../crates/ui/tests/lab_progress_recipe_stream.rs))
     and `TrailMirrorRecipe`
     ([`crates/ui/tests/trail_mirror_recipe_stream.rs`](../crates/ui/tests/trail_mirror_recipe_stream.rs))
     in Wave 1; `ServerTimeRecipe` was deferred because
     `cockpit-activity-status-bar v0.1.0` was concurrently touching
     `crates/ui/src/bin/cockpit_live.rs`. With cockpit-activity v0.1.0
     SHIPPED, this brief picks up the third and final canonical UI Recipe
     impl. Closes one of three identified Recipe surfaces under the
     [`testing-framework-audit-2026-05-25 § R1`](dev-notes/archive/2026-Q2/testing-framework-audit-2026-05-25.md)
     recommendation; brings the workspace to 3/3 covered.

     **Scope at v0.1.0**: (R1) refactor `ServerTimeRecipe::stream` to
     delegate to a `pub fn stream_impl(rt_handle) -> BoxStream<Message>`
     helper mirroring the precedent `ui::lab::progress::stream_impl`
     shape (architect M-T1 chooses (a) keep inline in `cockpit_live.rs`
     vs (b) extract to `crates/ui/src/live/server_time.rs` — default
     (a)); (R2) NEW test file
     `crates/ui/tests/server_time_recipe_stream.rs` with 4-5 tests
     (happy path, monotonicity, stream remains open, full Recipe
     end-to-end integration, optional lag handling); (R3) update
     `subscription-missing-e2e` spec-lint rule IF Wave 1 shipped it
     (analyst grep 2026-05-26 found ZERO matches — defer at R3.a
     default); (R4) non-regression contract — 34/34 anchors byte-
     identical + `cockpit_live.rs::subscription()` batch behavior
     unchanged + `Recipe::hash` impl byte-identical; (R5) workspace
     test count delta = +4 to +5.

     **What this brief does NOT do**: ship the
     `subscription-missing-e2e` spec-lint rule from scratch (deferred);
     change `ServerTimeRecipe`'s cadence or salt its identity hash
     (K2 forward-risk only); touch backtest / strategy / exec / risk
     / forecast / reports / audit / data bodies. UI bin + new test
     file only. 34/34 anchors stay byte-identical (NR-1, zero
     scenario-body changes by construction).

     **Cost framing**: ~0.5 day end-to-end wall-clock. Architect M-T1
     ~30 min (optional — light); developer M-DEV ~2 h (Wave A single
     wave: refactor + 4-5 tests + run gates); tester M-FINAL ~30 min;
     presenter ~30 min (optional — small enough to skip). ZERO LLM
     costs.

     **Operator-decide Q's**: NONE. Q1 = 0 by construction. The fix
     shape is determined by precedent (mirror Wave 1's verbatim);
     R3 is conditional and defaults to defer; architect R1 (a) vs (b)
     is an internal-architecture decision no operator routing needed.
     Standing Autoapprove applies trivially.

     **Trace row**: `REQ-SUBSCRIPTION-PIPE-SERVER-TIME-001` at
     `proposed` state (appended at END of `spec/trace.toml`; does NOT
     modify any existing row).
     **Feature folder**: [`spec/subscription-pipe-server-time-template/`](subscription-pipe-server-time-template/feature.md).
     **Predecessor (Wave 1 templates)**:
     [`crates/ui/tests/lab_progress_recipe_stream.rs`](../crates/ui/tests/lab_progress_recipe_stream.rs)
     +
     [`crates/ui/tests/trail_mirror_recipe_stream.rs`](../crates/ui/tests/trail_mirror_recipe_stream.rs)
     (both shipped 2026-05-25/26).
     **Audit framing**:
     [`spec/dev-notes/testing-framework-audit-2026-05-25.md § R1`](dev-notes/archive/2026-Q2/testing-framework-audit-2026-05-25.md).
-->

- **subscription-pipe-server-time-template v0.1.0 (Wave-1 follow-on — ServerTimeRecipe coverage)** —
  Brief authored 2026-05-26 closing the operator's 2026-05-26
  ServerTimeRecipe carve-out from the Wave 1 subscription-pipe test
  class template. With `cockpit-activity-status-bar v0.1.0` SHIPPED,
  `crates/ui/src/bin/cockpit_live.rs` is free for the third and final
  canonical UI Recipe refactor + integration test pair. Scope: refactor
  `ServerTimeRecipe::stream` body into a `pub fn stream_impl(rt_handle)
  -> BoxStream<Message>` helper (mirror Wave 1's
  `ui::lab::progress::stream_impl` shape, ~10-15 LoC change in one
  file), author a NEW `crates/ui/tests/server_time_recipe_stream.rs`
  with 4-5 tests (happy path / monotonicity / stream-remains-open /
  full Recipe end-to-end / optional lag handling). Q1 = 0
  operator-decide (refactor + test addition only). Anchor risk ZERO
  by construction. Workspace test count delta +4 to +5. **Spec**:
  [`spec/subscription-pipe-server-time-template/feature.md`](subscription-pipe-server-time-template/feature.md).
  **Trace**: `REQ-SUBSCRIPTION-PIPE-SERVER-TIME-001`. **Predecessors**:
  Wave 1 templates at
  [`crates/ui/tests/lab_progress_recipe_stream.rs`](../crates/ui/tests/lab_progress_recipe_stream.rs)
  +
  [`crates/ui/tests/trail_mirror_recipe_stream.rs`](../crates/ui/tests/trail_mirror_recipe_stream.rs).
  Estimated ~0.5 day wall-clock end-to-end.

<!-- updated 2026-05-25 (analyst, reflection-memory-trader-wiring) —
     **PROMOTED Idea → Active 2026-05-25** as a P0 hygiene-gate
     recovery brief. The workspace test
     `crates/reflection/tests/no_strategy_caller.rs::t1809_no_strategy_crate_consumes_reflection_retrieval`
     is currently RED on main — `v3-llm-forecaster` Waves B/C/G
     (commits 8c40ab0, 97b7c39, 8dcd72c) landed reflection-retrieval
     code directly inside `crates/strategy/src/llm_forecaster/`,
     violating R8.1 / R10.8 from `spec/v3-llm-forecaster/feature.md`.
     The gate-test names this brief as the recovery path.

     **Scope**: create new `crates/trader/` workspace crate
     (Q1=(a) analyst-recommended); move the entire 8-file
     `crates/strategy/src/llm_forecaster/` subtree + 13 integration
     test suites (Q2=(a) clean-cut); strategy crate's Cargo.toml
     drops the `reflection` path-dep (structural enforcement of the
     R8.1 invariant); no new trait surface at v0.1.0 (Q3=(a) —
     `MemoryProvider` trait deferred to v0.1.1 once second consumer
     lands). Gate-test recovery contract is R5: gate-test returns to
     PASS at M-FINAL + a sibling positive-assertion test
     `t1810_trader_crate_owns_reflection_retrieval` lands in the
     trader crate.

     **What this brief does NOT do**: touch the `Strategy` trait,
     touch the reflection writer pipeline, touch any backtest scenario
     body bytes, touch any audit migration. The 34 locked anchors
     stay byte-identical (R6.1 / H2 — additive-zero by construction);
     the 98 LLM-forecaster integration tests stay PASS post import-
     path rewrite (R6.2 / H3); Phase F UI byte-identity preserved
     (R6.3). Pure package-level refactor.

     **Cost framing**: 3-5 days wall-clock — architect M-T1 ~0.5 day,
     developer M-DEV ~2 days (split into Wave A workspace plumbing →
     Wave B file moves → Wave C import rewrites → Wave D gate-test
     tightening → Wave E errata + docs), tester M-FINAL ~0.5 day,
     presenter ~0.5 day. **No LLM costs** (pure refactor; no model
     calls).

     **Operator-decide Q's** (load-bearing — architect M-T1 gates):
     - Q1 — new `crates/trader/` vs extend existing. Analyst
       recommends **(a)** new crate per product.md § 3 Trader agent.
     - Q2 — move scope: entire subtree vs reflection-touching files
       only. Analyst recommends **(a)** clean-cut.
     - Q3 — inverse-API trait shape at v0.1.0. Analyst recommends
       **(a)** no new trait; v0.1.1 brief for `MemoryProvider`.
     - Q4 — gate-test tightening (add `NullReflectionStore` to
       forbidden list). Analyst recommends **(a)** tighten.
     - Q5 — trader crate owns audit emission. Analyst recommends **(a)**.
     - Q6 — sequencing parallel with lab-yahoo-realdata v0.1.1.
       Analyst recommends **(a)+(b)** parallel — no file overlap.
     - Q7 — v3-llm-forecaster errata location. Analyst recommends
       **(a)** append `## Errata` to v3 feature.md (preserves history).

     **Cross-feature impact**: v3-llm-forecaster `spec/v3-llm-forecaster/
     feature.md` gets a `## Errata` block per Q7=(a) (CLAUDE.md
     non-negotiable — anchored reports under `reports/` are byte-
     immutable; the feature.md itself is NOT a report and accepts
     additive edits). v3 decomp.md path references — architect M-T1
     decides update strategy at K6.

     **Trace row**: `REQ-REFLECTION-TRADER-001` at `proposed` state.
     **Feature folder**: [`spec/reflection-memory-trader-wiring/`](reflection-memory-trader-wiring/feature.md).
     **Gate test**: [`crates/reflection/tests/no_strategy_caller.rs`](../crates/reflection/tests/no_strategy_caller.rs).
-->

<!-- Moved Active → Recent 2026-05-26 — operator approval "Approved — ship".
     v0.1.0 shipped; commit chain 6d3d716 → 028761c → f05fc9b → f6a2a42.
     Trace row REQ-REFLECTION-TRADER-001 state = passed.
     **R8.1 RED gate that's been red on main the entire session — now GREEN.**
     Architect's corrected counts (9 source + 10 tests + 1 bin, not 8/13/0)
     applied at T-AR-2; trader crate now carries 153 tests (incl. doc-tests). -->
- **reflection-memory-trader-wiring v0.1.0** — SHIPPED 2026-05-26
  (operator approval "Approved — ship"). The R8.1 layering gate is
  now structurally enforced via the new `crates/trader/` workspace
  crate; strategy crate dropped its `reflection` path-dep. 9 source
  files + 1 bin + 10 integration tests migrated; t1809 + t1810 both
  GREEN; 34/34 anchors byte-identical. Deck at
  [`spec/reflection-memory-trader-wiring/presentations/reflection-memory-trader-wiring-2026-05-26.md`](archive/presentations-2026-Q2.tar.gz).
  See Recent section.

<!-- updated 2026-05-24 (analyst, lab-end-to-end-v2) —
     **PROMOTED Idea → Active 2026-05-24** after operator's 2026-05-24
     verification walk of the Lab screen exposed multiple gaps between
     the locked Phase A/B R-rows and the runtime reality. Two root
     causes are wiring bugs that should never have shipped under the
     locked Phase A/B rows:

     1. F1 — `crates/ui/src/state.rs:1922-1931` `LabRunCompleted` arm
        carries a NOTE comment promising a binary-side update wrapper
        rotates `last_run_report ← new RunReportMirror`; the wrapper
        DOES NOT EXIST in `crates/ui/src/bin/cockpit_live.rs`. Phase B's
        R5 ("chart reads equity from `last_run_report` first") is
        therefore dead code — `last_run_report` is never set from a real
        run.
     2. F2 — `crates/ui/src/state.rs:1892-1897` `LabSelectPair` arm
        updates `lab_state.pair` but NOT `model.selected_symbol`. The
        chart's price-line read at `screens/lab.rs:149-152` keys on
        `selected_symbol`. Pair chip is decorative for the chart.

     Plus 4 scope-leakage debts:
     3. F3 — `engine::run_scenario` dispatch is cross-sectional-only;
        4 single-symbol scenarios (SMA / MACD / RSI / BBands) still live
        in `crates/backtest/src/main.rs` CLI-only, not extracted in
        Phase B.
     4. F4 — `crates/ui/src/bin/cockpit_live.rs:1027` drops the
        `RunCancelHandle` immediately (`let (_, cancel_recv) = …`).
        Stop button never worked. Engine doesn't thread `cancel_rx`
        into scenario bar loops.
     5. F5 — No progress channel exists end-to-end. Operator's
        2026-05-24 walk asks for a progress-bar widget instead of the
        opaque ThrottledSpinner.
     6. F6 — `chart_markers` is fed by `audit::query::recent_fills_filtered`,
        not by `RunReport.fills`. Fresh-Run markers gap.

     **Cost framing**: ~1-2 weeks wall-clock. Wave D-1 (binary wrapper
     + LabSelectPair fix + fixtures pair pre-load + Run-completion test)
     is small + zero anchor risk — ~2 days. Wave D-2 (single-symbol
     dispatch extraction) is anchor-gated, ~3 days. Wave D-3 (Stop
     button) is medium, ~2 days. Wave D-4 (progress channel + widget)
     is medium, ~2-3 days; parallelizable with D-3 across different
     crates. Total estimated wall-clock 7-10 days.

     **Operator-decide Q's** (HIGH-stakes ones):
     - Q1 — strategy dispatch shape (single-symbol arms vs `pair_filter`
       vs scope selector vs defer). Analyst recommends single-symbol arms.
     - Q2 — Stop button in v2 scope or sibling feature. Analyst
       recommends in scope.

     **Trace row**: `REQ-LAB-E2E-V2-001` at proposed state.
     **Feature folder**: [`spec/lab-end-to-end-v2/`](lab-end-to-end-v2/feature.md).
     **Predecessor chain**: `ui-rethink-phase-a-lab v0.2.0` (shipped
     2026-05-18) → `ui-rethink-phase-b-lab-run v0.2.0` (shipped
     2026-05-19) → `lab-end-to-end-v2 v0.1.0` (this).
-->

- **lab-end-to-end-v2** — close Phase A/B runtime gaps in the Lab
  screen + add progress-bar widget. Wiring bugs (binary-side wrapper
  missing; `LabSelectPair` doesn't update `selected_symbol`) + scope
  debts (cross-sectional-only engine dispatch; Stop button never
  wired; no progress channel). 34/34 anchors must stay byte-identical.
  **Spec**: [`spec/lab-end-to-end-v2/feature.md`](lab-end-to-end-v2/feature.md).
  **Trace**: `REQ-LAB-E2E-V2-001`. **SHIPPED v0.1.0 2026-05-25** —
  see Recent section.

- **lab-polish-round-2** — position-curve overlay (R1) + SMA fast/slow
  param editor (R2) + KPI strip densification (R3). Author-approved
  follow-on from the 2026-05-25 verification walk after lab-end-to-end-v2
  shipped — Lab is functional but minimal; operator wants per-pair
  position visibility, in-cockpit param tuning, and denser at-a-glance
  KPIs. Anchor-additive (KPI extension goes UI-only, doesn't touch
  Markdown body). **Spec**:
  [`spec/lab-polish-round-2/feature.md`](lab-polish-round-2/feature.md).
  Estimated 3-5 days wall-clock. Operator-decide Q1-Q3 pending.

<!-- updated 2026-05-22 (analyst-bridge, v3-llm-forecaster) —
     **PROMOTED Queue → Active 2026-05-22** by operator under the
     `v3-volatility-forecaster-noop-fix` v0.1.0 sprint-review deck
     approval. The retirement chain that promoted C5:
     v3-volatility-forecaster-noop-fix v0.1.0 shipped 2026-05-22
     (VERDICT → PASS; 34/34 anchors); fix wave re-ran the
     overlay with real wiring; result MODEL-BROKEN / NO-ALPHA /
     **NEGATIVE-NET-DELTA** (–0.021719 net_delta vs un-targeted
     real-baseline). Joint advisory routed (a) RETIRE C1 with REAL
     evidence (vs the artifactual evidence from the parent's no-op
     overlay); operator picked **C5 over C2** for moat-alignment
     (product.md § Differentiator line 79-83: persistent reflection
     memory + auditable double-entry ledger) + `crates/llm` infra
     reuse (RecordingProvider/ReplayProvider/BudgetedProvider/
     CachedSystemPromptBuilder/ToolSchema all shipped at
     v2-llm-strategy v2.0.0 — no new infra needed). C2
     (`v3-regime-classifier`) stays in Queue per
     [Queue § Strategy](#queue) below — "DEFERRED-2026-05-22
     retained pending C5 ship".

     **C5 existing analyst pass** was spec-only design exploration
     (R1-R10 requirements + K1-K10 risk register + H1-H5 falsifiable
     hypotheses + Q1-Q8 operator-decide rows with analyst-recommended
     defaults + 8-item non-regression contract + deferred-milestone
     activation contract). The analyst-bridge pass 2026-05-22
     populated `spec/v3-llm-forecaster/tasks.md` M-OD/M-T1/M-DEV/
     M-FINAL/M-PRESENTER scaffolding so the architect can open M-T1
     on a clean handoff. Bridge-pass findings:

     - **crates/llm reuse confirmed**: all 7 surfaces cited in R1-R7
       are present at the cited paths. LlmProvider trait at
       `crates/llm/src/trait_def.rs`; BudgetedProvider auto-degrade
       gate at `budgeted.rs`; RecordingProvider + ReplayProvider
       sqlite-backed `(request_hash, response)` cache at
       `recording.rs` + `replay.rs` (**load-bearing for Q5=(b)
       determinism + H4 byte-identity anchor pre-condition**);
       CachedSystemPromptBuilder 2-cache-breakpoint at
       `prompt_cache.rs` (~75% input-token discount on repeats);
       ToolSchema + JSON-schema validation at `tools.rs`;
       LlmProviderFactory at `factory.rs`. No new infra needed in C5.
     - **crates/reflection reuse confirmed**: top_k retrieval +
       `REPORT_TIME_TOP_K = 5` (Q3=(a) default) + RetrievalQuery +
       3-state regime tagger (`regime.rs` — also load-bearing for
       C2; C5 may consume the same tag in RetrievalQuery if the
       analyst-bridge `(symbol, regime_tag, recent_outcome)`
       composition holds). Writer pipeline untouched per R10.8
       read-only consumer contract.
     - **Q1-Q8 standing-Autoapprove eligibility**: Q1/Q2/Q3/Q5/Q7/
       Q8 + Q-V2X-SEQ + Q-ASSISTANT-WAKE eligible (6 of 8 Qs + 2
       sequencing-Qs all default to analyst-recommended low-risk
       paths). **Q4 (Phase F Assistant slot body promotion shape
       — biggest product-differentiation surface in v0.1.0) + Q6
       (NEW ADR-0038-or-renumber "LLM-forecaster verdict criteria
       L1-L4" — codifies a new durable artifact across future LLM-
       strategy ships) require explicit operator decision**.
     - **ADR namespace open**: Q6=(b) new ADR-0038 conflicts with
       the existing `0038-vol-forecast-verdict-shape.md` from the
       retired C1 v3-volatility-forecaster lane. Architect M-T1
       confirms the renumber (likely 0039 or higher); analyst-
       bridge surfaces this as T-OD6 open question.
     - **Anchor count progression updated**: 30 existing (analyst-
       pass-time count) → 34 existing (post-noop-fix v0.1.0 ship
       which updated 4 SHAs in-place under existing namespaces) →
       36 at C5 v0.1.0 ship (+2 under new v3.0.0-llm-forecaster
       pin: `top10-2023-fy-llm-forecaster-realdata` +
       `top10-2024-fy-llm-forecaster-realdata`). Anchor risk
       MEDIUM per H4 LLM determinism + replay-cache pre-condition;
       T-AR-5 K4 mitigation locks SHAs only after 3-back-to-back
       identical cache-build runs.

     **Cost framing**: ~6-8 weeks wall-clock per survey ranking
     (analyst 1w done + architect 1-2w + dev 3-5w + tester 1w +
     presenter 1-2d). Wave F (Phase F Assistant slot body promotion)
     gated on Q4=(c) operator pick at T-OD4 — defer to v0.1.0
     follow-on `v0.1.1-assistant-slot-wake` if operator picks
     Q4=(a)-only. HIGH variance per K8 novel-territory risk.

     **Trace row**: `REQ-V3-LLM-FORECASTER-001` flipped
     `draft → proposed`; `arch` / `crates` / `tests` / `anchors`
     stay empty until architect / developer / tester fill at
     their respective milestones.

     **Pre-drawn 4-cell routing tree (presenter inherits at M-P2)**:
     R-O1 L0 PASS + H1 ≥ +0.10 Sharpe-delta → SHIP; promote to
     paper-trading per product.md § Strategy lifecycle; spawn
     `v3-llm-forecaster-overlay-on-momentum` Q4=(b) deferred v0.2.0.
     R-O2 L1/L2/L4 trigger + H1 marginal → HOLD; re-tune cadence
     (T-AR-4) or prompt structure (T-AR-2); re-run.
     R-O3 F-equivalent (H1 = 0 AND no L-verdict) → retire C5;
     preserve spec as what-not-to-chase reference (mirrors v2.5
     DL retirement); re-route to C2 v3-regime-classifier.
     R-O4 L3 cost-overrun → bump R5.4 N to 168 weekly cadence or
     downgrade to quick-think tier; re-run backtest only.

     HANDOFF → operator-decide (Q4 + Q6 explicit; Q1/Q2/Q3/Q5/Q7/
     Q8 + Q-V2X-SEQ + Q-ASSISTANT-WAKE standing-Autoapprove eligible
     per operator's 2026-05-22 standing directive) → architect M-T1
     → developer Waves A-G → tester → presenter → operator next-
     decision keyed to R-O1..R-O4 verdict cell. -->

<!-- updated 2026-05-22 (analyst, v3-volatility-forecaster-noop-fix) —
     **P0 WIRING-BUG DISCOVERY**: the GARCH vol-targeting overlay at
     `crates/strategy/src/vol_targeting_overlay.rs:305-319` is a **no-op**.
     `compute_scale` returns the correct scale factor, but the `else`
     branch increments a stats counter and returns `base_signals`
     unmodified. The inline comment admits the scale is "diagnostic
     only — the backtest engine reads quantities from fills, not from
     signal metadata." The scale flows nowhere.

     **Diagnostic chain (orchestrator caveman probe 2026-05-22 ~11:44Z)**:
     (1) Caveman patch multiplying `sigma_hat` by 2.95 (the parent's
     mean_calibration_ratio = 2.952191) produces byte-identical equity
     to the parent anchor 66cd69ad… ($113,479.98 / 13.48% / 73.73% DD /
     6203 trades) — definitionally a no-op.
     (2) The rebaseline pass's un-targeted `top10-2023-fy-momentum-
     realdata` baseline produces the byte-identical SAME equity tuple
     — overlay output == baseline output, which is the no-op signature.
     (3) `stats.signals_scaled = 6203` ticks for every signal, but the
     signals are returned unmodified per code review.

     **Provisional invalidation**: both `v3-volatility-forecaster
     v0.1.0` and `v3-volatility-forecaster-rebaseline v0.1.0` are
     wiring-failure ships, not real alpha tests. The MODEL-BROKEN /
     NO-ALPHA joint advisory + the (a) RETIRE-C1 routing pick are
     **invalidated** until the wire-up fix lands and the re-run
     produces a real verdict. The V3 calibration finding
     (mean_calibration_ratio = 2.952191) survives the fix verbatim
     (GARCH-only diagnostic, measured before the overlay applies the
     scale).

     Brief at `spec/v3-volatility-forecaster-noop-fix/feature.md`
     (status: proposed, owner: analyst, version: 0.1.0, priority: P0,
     parent: v3-volatility-forecaster v0.1.1 parent_disposition
     = provisionally-invalidated-pending-rewire, sibling:
     v3-volatility-forecaster-rebaseline v0.1.0 same disposition).
     R1–R6 tight 1-3 day requirements:
     R1 wire-up fix at the strategy → executor handoff (10-line scope
     in vol_targeting_overlay.rs); R2 end-to-end equity-divergence
     regression test (overlay equity ≠ un-targeted baseline equity by
     ≥ 1 bp when scale ≠ 1.0) — the MISSING gate that would have
     caught the no-op; R3 affected anchors re-emit cleanly (3-4 rows:
     top10-2023-fy-vol-target-overlay-realdata + sharpe-comparison-
     vol-target-bs1-realdata + sharpe-comparison-vol-target-bs1-
     realbaseline change for sure; vol-verdict-bs1-realdata audit-
     pending at T-AR-2); R4 amendment blocks in parent + rebaseline
     feature.md § Verification; R5 ADR-0038 § D6 wiring-bug-fix
     exception clause documenting legitimate re-emission protocol;
     R6 unit + integration regression tests guarding scale != 1.0
     propagation at the strategy and engine boundaries.

     **TCN overlay co-investigation (T-A2) — RULED OUT**: TCN
     overlay's dampen-to-Hold semantic mutates `Signal.kind`, which
     IS a load-bearing field the executor reads. No parallel bug;
     Q3=(b) vol-target-only fix is the default.

     **Q1–Q3 operator-decide WITH ANALYST-RECOMMENDED DEFAULTS**:
     Q1=(ii) defaulted `Strategy::quantity_scale(&self, symbol) → f64`
     trait method (minimum blast radius vs Q1=(i) `Signal.quantity_
     scale` field change); Q2=(a) re-emit affected anchors in-place
     under existing namespaces + ADR-0038 § D6 amendment subsection
     documenting the wiring-bug-fix re-emission protocol; Q3=(b)
     vol-target-only fix.

     **Standing Autoapprove** from operator's 2026-05-22 prior session
     applies to Q1–Q3 defaults; orchestrator may auto-tick T-OD1..3
     before spawning architect for M-T1. Trace row
     `REQ-V3-VOL-FORECASTER-NOOP-FIX-001` opened `proposed` (parent
     = REQ-V3-VOL-FORECASTER-001).

     **Anchor projection**: 34 PASS (current) → 34 PASS at M-FINAL,
     with 3-4 fresh SHAs (the affected vol-target rows) and 30-31
     unchanged (negative invariant; non-vol-target scenarios stay
     byte-identical).

     **Cost framing**: ~1-3 days end-to-end. Wire-up fix is 10 lines
     of code plus the architect's Q1 choice plumbing (~50 LoC if
     Q1=(ii) defaulted trait method; ~150 LoC if Q1=(i) Signal field
     change). Anchor re-emission ~2 backtest runs (~80s wall-clock).
     ADR-0038 § D6 amendment ~30 lines. Regression tests ~100 LoC.

     **Pre-drawn 4-cell routing tree (presenter inherits)**:
     R-O1 T-VOL-NO-ALPHA + PASS → (a) RETIRE C1 with REAL evidence
     (vs the current artifactual evidence); promote C2 or C5;
     R-O2 T-VOL-MARGINAL + PASS → (a) RETIRE OR (d) v0.1.2 GARCH
     refit;
     R-O3 T-VOL-ALPHA-UNLOCKED + PASS → reopen `v3-volatility-
     forecaster` as a LIVE candidate (prior MODEL-BROKEN / NO-ALPHA
     verdict fully retracted); spawn `v3-garch-calibration-tune`
     for V3 repair before banking the alpha live;
     R-O4 FAIL → developer fix iteration; if overflow, operator-
     decide extend-budget vs roll back.

     Discovery dev-note at `spec/dev-notes/v3-vol-overlay-noop-
     discovery-2026-05-22.md` captures the 8-hour timeline from
     rebaseline ship → byte-identity surfacing → caveman probe →
     smoking gun + the five gate layers that missed it + the
     meaningful end-to-end test shape (R2).

     HANDOFF → operator-decide (Q1..Q3 standing-Autoapproved) →
     architect M-T1 → developer Wave A (fix + tests) + Wave B
     (anchor re-emission + ADR amendment) → tester → presenter →
     operator next-decision keyed to R-O1..4 verdict cell. -->

<!-- updated 2026-05-22 (analyst, v3-volatility-forecaster-rebaseline) —
     **Routing (b) RE-BASELINE FIRST** spawned from
     `v3-volatility-forecaster` v0.1.0 presenter deck approval on
     2026-05-22. The parent shipped with joint advisory verdict
     **V3 × T-VOL-NO-ALPHA → MODEL-BROKEN / NO-ALPHA**; the
     sharpe-comparison carrying T-VOL-NO-ALPHA uses a SYNTHETIC GBM
     un-targeted v1 momentum baseline against a REAL Binance overlay,
     and the operator picked (b) over (a) RETIRE-C1 / (c) DEBUG-V3 /
     (d) v0.1.1 GARCH refit to disambiguate the data caveat before
     committing multi-week budget.

     Brief at `spec/v3-volatility-forecaster-rebaseline/feature.md`
     (status: proposed, owner: analyst, version: 0.1.0, parent:
     v3-volatility-forecaster v0.1.0 shipped-with-MODEL-BROKEN-NO-ALPHA-
     advisory). R1–R5 tight 1-day-scoped requirements:
     R1 baseline MUST be real-data; R2 baseline provenance pinned per
     ADR-0032 (revision_sha 3a8b96c43f…); R3 T-classifier re-evaluated
     against new net_delta per ADR-0038 § D1.c; R4 anchor-additive
     through report emission, +1 anchor at M-FINAL under NEW
     `[v3.0.0-volatility-rebaseline]` namespace block (existing 3
     `[v3.0.0-volatility]` anchors stay byte-immutable per ADR-0038
     § D6); R5 2-run byte-identity determinism carries forward from
     parent R11.9 / R11.10. K-rebase-1..4 risks (anchored-report-reuse
     CLOSED; T-VOL-NO-ALPHA still possible; T-classifier flip possible;
     determinism fail recoverable). H-rebase-1..2 hypotheses (real-vs-
     real net_delta WILL move, magnitude/direction unknown; V3 finding
     survives independently regardless of T-classifier outcome).

     **Investigation findings embedded** (analyst T-A2): the parent's
     `--scenario vol-target-bs1` dispatch hard-codes the synthetic
     baseline at
     `crates/forecast/src/bin/sharpe_comparison.rs:1293`; no real-data
     un-targeted v1 momentum scenario exists in
     `crates/backtest/src/main.rs::Scenario::from_name` today (only
     `top10-2023-1h-momentum` and `top10-2024-h1-momentum`, both
     `data_source: Synthetic`); no anchored realdata momentum report
     exists in `spec/backtest-real-binance-data/reports/` or anywhere
     else under `spec/`. Net: cheapest path (report-reuse) CLOSED;
     developer must add a new `top10-2023-fy-momentum-realdata`
     scenario (~25 LoC additive, mirrors existing `-realdata` pattern;
     ~40s backtest wall-clock; no design churn).

     **Q1–Q3 operator-decide ALL WITH ANALYST-RECOMMENDED DEFAULTS:**
     Q1=(b) introduce `top10-2023-fy-momentum-realdata` scenario
     (Q1=(a) anchored-report-reuse structurally REJECTED by T-A2
     finding #3); Q2=(a) anchor
     `sharpe-comparison-vol-target-bs1-realbaseline` under NEW
     `[v3.0.0-volatility-rebaseline]` namespace, N_new = +1;
     Q3=(a) deliverable lands at
     `spec/v3-volatility-forecaster-rebaseline/reports/`.

     **Standing Autoapprove** from operator's 2026-05-22 prior session
     applies to Q1–Q3 defaults; orchestrator may auto-tick T-OD1..3
     before spawning architect for M-T1. Trace row
     `REQ-V3-VOL-FORECASTER-REBASELINE-001` opened `proposed` (parent
     = REQ-V3-VOL-FORECASTER-001).

     **Anchor projection:** 33 PASS (post-parent-ship) → **34 PASS**
     at M-FINAL under Q2=(a) default, or 35 PASS if operator opts in
     on Q2=(b) (anchor both the new sharpe-comparison AND the new
     baseline backtest). Net anchor-additive — zero churn on the
     existing 33.

     **Cost framing:** ~1 day end-to-end per (b) routing rationale —
     scenario add ~10 min + sharpe-comparison.rs patch ~10 min +
     backtest re-run ~40s + sharpe-comparison re-run ~10s + 2-run
     byte-identity ~25s + tester gates + presenter pass. <2 hours of
     developer time; the rest is architect M-T1 + tester M-FINAL +
     presenter assembly.

     **Pre-drawn 4-cell routing tree (presenter inherits):**
     R-O1 T-VOL-NO-ALPHA + PASS → (a) RETIRE C1, promote C2 or C5;
     R-O2 T-VOL-MARGINAL + PASS → (d) v0.1.1 GARCH refit;
     R-O3 T-VOL-ALPHA-UNLOCKED + PASS → (c) DEBUG V3 (spawn
     `v3-garch-calibration-tune`); re-opens v0.1.0 retirement
     question;
     R-O4 (any) + FAIL determinism → back to developer for fix; if
     iteration overflows, operator-decide extend-budget vs (a).

     HANDOFF → operator-decide (Q1..Q3, standing-Autoapproved) →
     architect M-T1 → developer Wave A+B (parallel-safe: scenario add
     + sharpe-comparison.rs patch can land in one PR) → tester →
     presenter → operator next-decision keyed to R-O1..4 verdict
     cell. -->

<!-- updated 2026-05-22 (analyst, v3-volatility-forecaster) —
     **C1 / 3 hybrid-sequence analyst passes** triggered by operator's
     2026-05-22 routing post v25a-patchtst-overlay v0.1.0 ship
     (joint F4-F4-F4 across TCN BS-1/BS-2 @ 1h + PatchTST BS-1 @ 24h
     retired the v2.5 DL forecast overlay umbrella). Q-PICK = C1
     (volatility) + C2 (regime) + C5 (LLM-as-forecaster) = 3 picks;
     Q-BUDGET ~6-8 weeks total cap; Q-SEQ = HYBRID (build C1 first;
     C2 + C5 analyst-only spec parallel-authored same day, no code
     commitment until C1 verdict OR operator promote); Q-PROCESS =
     3 analysts in parallel. This entry is **C1 — the only code-
     committing pick at this gate**.

     Brief at `spec/v3-volatility-forecaster/feature.md` (status:
     draft, owner: analyst, version: 0.1.0, predecessor:
     v25a-patchtst-overlay v0.1.0 RETIRED-evidence-source, parent:
     NONE — new strategy lane signaled by v3.0.0 anchor pin)
     carries R1-R12 requirements (vol target derivation via
     Parkinson estimator + GARCH(1,1) baseline + conditional DL
     refinement + vol forecaster trait/impl + V-verdict algorithm
     + NEW ADR-0038 + vol-targeting overlay strategy on v1
     momentum + kill-switch + standalone + backtest scenario +
     Sharpe-comparison + watch recipe + non-regression contract
     + verification gates + risk-engine integration deferral).
     H1-H4 hypothesis register: H1 DL beats GARCH ≥5% QLIKE;
     **H2 vol-targeting Sharpe-delta ≥ +0.10 vs un-targeted v1
     baseline (THE alpha-unlock test)**; H3 3-4 week cheap-first
     ship under Q2=(a); H4 hourly crypto vol IS predictable.
     K-vol-1..6 risk register (turnover eats lift; strategy-side
     vs risk-engine ADR amendment; scope creep guard; H4
     falsification; V-verdict disagreement; cheap-first under-
     delivers).

     **Q1-Q6 operator-decide ALL WITH ANALYST-RECOMMENDED
     DEFAULTS:** Q1=(b) Parkinson estimator (5-7× more sample-
     efficient than realized-vol-from-close per Parkinson 1980;
     reuses existing high/low columns; zero new data sourcing);
     **Q2=(a) GARCH(1,1)-only-MVP** (cheap-first per
     retrospective lesson #1; defers DL refinement to v0.1.1 if
     v0.1.0 finishes T-VOL-MARGINAL); Q3=(d) all-3-consumer-
     builders (vol-targeting overlay on v1 momentum as PRIMARY
     anchor target + kill-switch + standalone, all opt-in via
     builders); **Q4=(b) NEW ADR-0038 V-verdict** (V1-V5
     priority tree + V_ALPHA; **ADR-0033 stays IMMUTABLE per
     retrospective lesson #2**); Q5=(a) anchor under
     `v3.0.0-volatility` (signals strategy-lane shift; mirrors
     `v2.5a.0-patchtst` naming; N_new = 3 anchors); Q6=(a) BS-1
     train + BS-2 val span (apples-to-apples vs v2.5 scenario
     surface).

     "Autoapprove" activates the bundle; defaults are internally
     consistent. Trace row `REQ-V3-VOL-FORECASTER-001` opened
     `draft`. Promoted Queue/Strategy → Active 2026-05-22 (first
     ship in v3 post-DL strategy reformulation lane).

     **Anchor baseline:** 28 PASS + 2 known-FAIL (carry-forward
     from v25a-patchtst-overlay v0.1.0 ship; 30 anchors total
     including the 2 v2.5a.0-patchtst additions). POST under
     Q5=(a) expects **28 + 3 = 31 PASS + 2 known-FAIL** (or 32
     PASS + 2 FAIL if Q-anchors sub picks kill-switch anchor).

     **Cost framing:** Q2=(a) GARCH-only ~2-3 weeks best case
     (~3-4 weeks with one retry); Q2 ≠ (a) DL refinement ~4-6
     weeks. Operator's 6-8 week cap holds either way; Q2=(a)
     leaves ~3-4 weeks for promoting one of C2/C5 to code after
     C1 verdict, **Q2 ≠ (a) leaves only ~2-3 weeks** — operator
     should weight Q2 default heavily.

     **Prior probability of clearing H2 +0.10 Sharpe-delta:**
     MEDIUM-HIGH per Moreira-Muir-2017 vol-targeting precedent
     on equity factor portfolios (reported Sharpe lifts 0.15-0.40
     on momentum); crypto-at-hourly-cadence transaction-cost
     drag is the load-bearing empirical unknown; analyst's prior
     is alpha SURVIVES the turnover net cost given the [0.5×,
     2×] scale clamp default.

     HANDOFF → operator-decide (Q1-Q6) → architect for M-T1 /
     ADR-0038 V-verdict shape. Sibling analyst passes (no code):
     `spec/v3-regime-classifier/feature.md` (C2) +
     `spec/v3-llm-forecaster/feature.md` (C5) under Queue §
     Strategy with `status: draft` + activation gated on C1
     verdict. -->

<!-- updated 2026-05-21 (analyst, v25a-patchtst-overlay) —
     analyst pass landed for phase 2 of the 4-phase DL roadmap
     activated by operator's Q1=(b) RETIRE v2.5 TCN decision at
     v25-tcn-horizon-bump-or-retire M-OD 2026-05-21. Brief at
     `spec/v25a-patchtst-overlay/feature.md` (status: draft,
     owner: analyst, version: 0.1.0, predecessor:
     v25-tcn-horizon-bump-or-retire v0.1.0, parent:
     v25-dl-forecast-overlay v0.0.0 roadmap) carries R1-R10
     requirements (PatchTST model in candle + training scaffold
     with ADR-0035 § D1 post-training σ_train contract from the
     start + ForecastProvider impl + 1 BS-1 anchored checkpoint
     + sibling strategy + backtest scenario + alpha-investigation
     cycle reusing ADR-0033 § D3 immutable F-verdict). H1-H4
     hypothesis register (paradigm signal vs TCN; session-level
     attention; 4-6 week feasibility; 24h horizon SNR). K1-K6
     risk register (compute over-run; candle-attention bugs;
     F4-on-PatchTST routes to v2.6; anchor regression; default
     hyperparameters; scope creep into v2.5 TCN crate). Q1-Q8
     operator-decide ALL WITH ANALYST-RECOMMENDED DEFAULTS:
     Q1=(a) PatchTST, Q2=(a) full MVP, Q3=(a) patch_len=16
     stride=8, Q4=(b) 24h horizon, Q5=(c) carry-forward
     5-feature input, Q6=(a) BS-1 2023 span, Q7=(a) anchor
     under v2.5a.0-patchtst, Q8=(a) sibling strategy.
     "Autoapprove" activates the bundle; defaults are internally
     consistent. Trace row REQ-V25A-PATCHTST-001 promoted
     roadmap → draft. Promoted Queue/Strategy → Active 2026-05-21
     (ACTIVATION TRIGGERED tag cleared). Anchor baseline
     26 PASS + 2 known-FAIL (carry-forward from
     v25-tcn-horizon-bump-or-retire); POST under Q7=(a) + Q8=(a)
     expects 28 PASS + 2 known-FAIL. Cost ~3-5 weeks best case;
     ~5-7 weeks with one Wave-B retry. Apple Silicon Metal
     bound; PatchTST/42 small config (~1.5-2M params) targets
     ~5-7 days per training run. HANDOFF → operator-decide
     (Q1-Q8) → architect for M-T1 / ADR-0036
     PatchTST-training-contract. -->

<!-- updated 2026-05-21 (analyst, v25-tcn-horizon-bump-or-retire) —
     analyst pass landed for the multi-week fallback after the
     v25-tcn-threshold-tuning v0.1.0 (shipped 2026-05-21) joint
     T-MARGINAL + T-MARGINAL verdict (BS-1 +0.018 / BS-2 +0.045
     Sharpe-delta at τ=0.1/ε=0.001; below the +0.10
     T-ALPHA-UNLOCKED threshold). Brief at
     `spec/v25-tcn-horizon-bump-or-retire/feature.md` is
     SCOPE-DECISION-GRADE — Q1 (primary scope: a horizon-bump /
     b retire-promote-PatchTST / c both in parallel / d defer-on-
     live) is HARD BLOCKER with NO safe analyst default. R1-R8
     scope-dependent; H1-H3 hypothesis register; K1-K7 risk
     register; Q1-Q7 operator-decide. Trace row
     `REQ-V25-TCN-HORIZON-BUMP-OR-RETIRE-001` opened `draft`.
     Cost framing: (a) ~7-10 days / (b) ~4-6 weeks /
     (c) ~6-9 weeks / (d) ~30-90 days. Promoted Queue/Strategy →
     Active 2026-05-21 (ACTIVATION TRIGGERED tag cleared). -->

<!-- updated 2026-05-21 (analyst, v25-tcn-threshold-tuning) — analyst
     pass landed for the cheap-first follow-on to v25-tcn-recalibrate
     v0.1.0 (shipped 2026-05-21, operator routing (c) chosen). Brief
     at `spec/v25-tcn-threshold-tuning/feature.md` carries R1-R9, H1-H3,
     K1-K6, Q1-Q6 with analyst-recommended defaults. Trace row
     `REQ-V25-TCN-THRESHOLD-TUNING-001` opened `draft`. Fallback stub
     `v25-tcn-horizon-bump-or-retire` added under Queue § Strategy below,
     activation gated on this feature's joint T-verdict (T-NO-ALPHA →
     fund horizon-bump; T-ALPHA-UNLOCKED → ship tuned cell + close out).
     Substantive motivation: predecessor recalibrate ship eliminated
     the σ_train 608× / 580× inflation but joint F-verdict legitimately
     stays F4 under immutable ADR-0033 § D3 (`frac_inside_epsilon`
     0.031 / 0.057 < 0.5 threshold). HOWEVER gate-survival jumps from
     0% to 40.1% (BS-1 τ=0.6) / 34.5% (BS-2 τ=0.6) / 88.8% (BS-1 τ=0.1) /
     86.4% (BS-2 τ=0.1) — necessary-but-not-sufficient for alpha.
     The τ × ε sweep is the cheap empirical answer. -->

<!-- Removed 2026-05-22 (audit-2026-05-22 P2.5 cleanup):
     - `v25-tcn-alpha-investigation` Active row — shipped 2026-05-19;
       moved to Recent (shipped) section.
     - `v25-tcn-overlay` Active row — flipped status: shipped 2026-05-22
       per audit P1.2 (F4 disposition). The entry pointed at children
       that have since shipped (alpha-investigation, recalibrate,
       threshold-tuning, horizon-bump-or-retire, v25a-patchtst-overlay)
       and to a now-retired roadmap. Historical context preserved in
       the v2.5 DL journey retrospective:
       `spec/dev-notes/v25-dl-journey-retrospective-2026-05-22.md`.
     The Queue § Strategy stubs below (v25-tcn-horizon-bump-or-retire,
     v25-tcn-alpha-investigation pivot-reference) are preserved as
     archeology of the operator-decide choice flow that produced the
     7-ship journey + joint F4-F4-F4 retirement decision. -->

<!-- updated 2026-05-26 (analyst, cockpit-activity-llm-producer) —
     **PROMOTED Idea → Active 2026-05-26** as the v0.1.1 follow-on
     of the just-shipped `cockpit-activity-status-bar v0.1.0`
     (2026-05-26). v0.1.0 § Q8 forward-listed `ActivityKind::LlmCall`
     as the next producer to wire once `v3-llm-forecaster` provider
     lifecycle stabilised + the trader-crate housing landed. Both
     conditions are now met: `v3-llm-forecaster` Waves B/C/G shipped
     2026-05-23..05-24; `reflection-memory-trader-wiring v0.1.0`
     shipped 2026-05-26 (moved `crates/strategy/src/llm_forecaster/`
     → `crates/trader/src/llm_forecaster/`). The LLM call site is
     now `crates/trader/src/llm_forecaster/anthropic_impl.rs:412-416`
     — single hot path, `async fn forecast` of `LlmForecasterImpl`.

     **Scope at v0.1.1**: wire an `ActivityHandle` around
     `provider.complete(request).await` in `anthropic_impl.rs:412-416`.
     Label: `"LLM call: <model_id>"` (Q1=(a) — no prompt content,
     no symbol context, structural PII redaction by construction).
     Conditional wiring via new optional field
     `activity_sender: Option<ActivitySender>` injected at construction
     time + builder setter `.with_activity_sender(s)`. When None
     (all existing test paths + bin paths + anchored backtest paths),
     `forecast()` behaves byte-identical to today — zero perf impact,
     zero event emission, zero anchor risk. `!Send` constraint
     workaround per Q2=(a): handle created on one line, awaited,
     explicit `drop(activity)` BEFORE any subsequent `.await` (there
     is none today — H1 falsification probe at architect M-T1). All
     `LlmForecasterError` variants map to `handle.fail(error.to_string())`
     per Q3=(a) — surfaces in tape as red 3-second hold (inherits
     parent R2.5).

     **What this brief does NOT do**: touch GARCH math, change
     timeout semantics (45 s Q5b architect-locked), touch
     `BudgetedProvider` cap accounting, touch `spawn_audit_row`,
     change `LlmForecasterImpl::new` arity (additive builder setter
     instead), touch `crates/strategy/` (would re-introduce the R8.1
     layering violation just resolved by
     reflection-memory-trader-wiring), touch `crates/agent/`
     (consumes the parent's `ActivitySender` + `ActivityKind::LlmCall`
     enum value forward-listed at v0.1.0; does not extend). Anchor
     risk ZERO by construction (R5.1) — 34 anchors stay byte-
     identical because anchored backtest paths construct
     `LlmForecasterImpl` without an `ActivitySender`.

     **Cost framing**: ~1-2 days end-to-end wall-clock. Analyst pass
     ~0.5 day (this); operator-decide ~30 min (all 3 Qs
     standing-Autoapprove); architect M-T1 ~0.5 day (H1 probe + K6
     injection-site lock); developer M-DEV ~0.5-1 day (single Wave A,
     single file edit + ~ 200 LOC test file with 6 integration tests);
     tester M-FINAL ~0.5 day; presenter ~0.5 day. **No LLM costs**
     (wiremock + stub providers). Rollback ~ 220 LOC.

     **Operator-decide Q's** (3 surfaced; all standing-Autoapprove-
     eligible at analyst-recommended defaults — focused producer
     wire-up; no mechanism choices to make, parent v0.1.0 locked
     them):
     - Q1 — label content: (a) `"LLM call: <model_id>"` (analyst
       recommended — structural PII redaction; no prompt, no symbol,
       no lesson content) vs (b) include symbol context (rejected —
       slippery slope) vs (c) generic without model ID (rejected —
       loses cost/latency debugging actionability).
     - Q2 — handle ownership / Send-constraint workaround: (a) store
       inside `LlmForecasterImpl::forecast` for the duration of the
       `complete()` call; explicit drop BEFORE any subsequent `.await`;
       `!Send` is fine because the call is awaited in-place (analyst
       recommended — smallest blast radius, zero parent design
       change) vs (b) Arc-Mutex (rejected — lock contention) vs
       (c) make handle `Send` (rejected — parent design lock) vs
       (d) tokio spawn (rejected — thread hop).
     - Q3 — failure-state handling: (a) `handle.fail(error.to_string())`
       on `LlmError`; red 3 s hold in tape (analyst recommended) vs
       (b) cancel on user-cancellable errors only (rejected — no
       cancel surface exists today) vs (c) drop without fail
       (rejected — misleading "succeeded" on network error).

     **Cross-feature impact**:
     - `cockpit-activity-status-bar v0.1.0` § Q8 forward-list closes.
     - `v3-llm-forecaster v0.1.0` provider lifecycle untouched —
       we tap the existing call site, no protocol change.
     - `reflection-memory-trader-wiring v0.1.0` provided the housing
       crate; no further interaction.
     - **Forward-listed at v0.1.2+**: per-token streaming Tick events
       (requires Anthropic streaming API switch); reflection
       retrieval as a separate activity (K4 — operator-on-request);
       cost / token-usage in label (requires parent `ActivityHandle`
       post-response rewrite contract).

     **Trace row**: `REQ-COCKPIT-ACTIVITY-LLM-PRODUCER-001` at
     `proposed` state (appended at END of `spec/trace.toml`; does
     NOT modify any existing row; parallel analysts also appending
     at EOF — row-level carving preserved).
     **Feature folder**:
     [`spec/cockpit-activity-llm-producer/`](cockpit-activity-llm-producer/feature.md).
     **Predecessor chain**:
     `cockpit-training-control v0.2.0` →
     `lab-end-to-end-v2 v0.1.0` →
     `cockpit-activity-status-bar v0.1.0` (parent — Q8 forward-list) +
     `v3-llm-forecaster v0.1.0` (provider) +
     `reflection-memory-trader-wiring v0.1.0` (housing) →
     `cockpit-activity-llm-producer v0.1.1` (this brief).
-->

<!-- moved to Recent (shipped) — v0.1.0 operator-approved 2026-05-27 -->
<!-- - **cockpit-activity-llm-producer v0.1.0** — see Recent section below for v0.1.0 ship summary. -->

<!-- updated 2026-05-26 (analyst, cockpit-training-pressed-wiring M0) —
     v0.1.1 follow-on brief authored against
     [`cockpit-activity-status-bar v0.1.0`](cockpit-activity-status-bar/feature.md)
     Wave C open question. The Wave C T-D-N9 producer wiring landed the
     `training_activity_handle` field + tick/end/cancel arms in
     `crates/ui/src/bin/cockpit_live.rs::AppState::update` (lines
     1103-1131) but the upstream `Message::TrainingPressed` arm is NOT
     wired to call `lab::trainer::spawn_training_run`. The pure-state
     arm at `crates/ui/src/state.rs:2064-2070` is a documented no-op
     ("Actual subprocess spawn lives in the binary"); the binary's
     `update` wrapper has no corresponding intercept. The gap is
     called out verbatim at `cockpit_live.rs:1020-1025`. **From a real
     cockpit launch, pressing the Train button does nothing.**

     **Scope at v0.1.0**: bind the binary-side intercept (~30-60
     LOC). Mirrors the `LabRunRequested` precedent in the same file.
     Resolves default `TrainingConfig` (R3 — analyst-default
     `crates/forecast/train_tcn.toml`), constructs mpsc + cancel
     handles, calls `lab::trainer::spawn_training_run(..., Some(self.bus.activity()))`,
     stores `(TrainingHandle, ActivityHandle)` on success, toasts on
     error. The downstream Wave C T-D-N9 lifecycle arms already
     consume `training_activity_handle` for tick/cancel/end — no
     changes there. Activity tape lights up automatically once the
     producer fires.

     **What this brief does NOT do**: schema changes (R-NR.2 —
     `training_events` audit table unchanged); bus channel changes
     (R-NR.3); new Lumen tokens (R-NR.4); state.rs signature changes
     (R-NR.5 — `Message`, `Cockpit`, `LabState`, `update` unchanged);
     hyperparameter editing (parent `cockpit-training-control` Q3
     deferred); config picker dropdown (Q1=(c) deferred); audit-DB
     toggle UI (deferred); multi-run queue (parent out-of-scope).

     **Operator-decide Qs** (both analyst-default + standing-
     Autoapprove-eligible; cost of wrong default ~5 LOC):
     - Q1 — Default training config source: (a)
       `crates/forecast/train_tcn.toml` (the canonical v2.5 config,
       already on disk) vs (b) `crates/strategy/configs/training/btc_macd_trend.toml`
       (referenced in upstream task brief but **does NOT exist on
       disk** — verified 2026-05-26) vs (c) defer to follow-on
       picker UI. Analyst recommends **(a)**.
     - Q2 — Cancellation on double-press: (a) button disabled
       (inherits parent R3.4 — `cockpit-training-control` R3.4)
       vs (b) re-press SIGKILL-cancels vs (c) re-press queues
       (out-of-scope per parent). Analyst recommends **(a)**.

     **Anchor risk ZERO by construction** — 34 anchors stay
     byte-identical (no `crates/backtest` / `crates/strategy` /
     `crates/exec` / `crates/forecast` source touches; the
     `train_tcn` binary is invoked as a SUBPROCESS, its bytes
     unchanged). 818+ workspace tests stay green; cockpit-smoke 0
     panics; spec-lint contributes zero new violation categories.
     Hard gates per R-NR.6 / R-NR.7 / R-NR.8 / R-NR.9.

     **Cost framing**: ~0.5-1 day end-to-end wall-clock. Analyst
     pass ~0.5 day (this); operator-decide ~5 min (both Qs
     standing-Autoapprove); architect M-T1 ~1-2h (or skip directly
     to developer — the wiring shape is unambiguous against the
     `LabRunRequested` precedent); developer M-DEV ~3-4h (~30-60
     LOC binary glue + 4 integration tests + 1 unit test);
     tester M-FINAL ~30 min. Rollback ~ 60 LOC.

     **Trace row**: `REQ-COCKPIT-TRAINING-PRESSED-001` at
     `proposed` state (appended at END of `spec/trace.toml`).
     **Feature folder**:
     [`spec/cockpit-training-pressed-wiring/`](cockpit-training-pressed-wiring/feature.md).
     **Predecessor**: `cockpit-activity-status-bar v0.1.0` (shipped
     2026-05-26 — landed the field this brief populates).
     **Parent**: `cockpit-training-control v0.2.0` (shipped
     2026-05-19 — defined `spawn_training_run`, `TrainingConfig`,
     `TrainingHandle`, cancellation contract).
-->

<!-- moved to Recent (shipped) — v0.1.0 operator-approved 2026-05-27 -->
<!-- - **cockpit-training-pressed-wiring v0.1.0** — see Recent section below for v0.1.0 ship summary. -->

## Queue

<!-- updated 2026-05-27 (orchestrator, v5-latency-slippage-sim-v0.2.0-anchor-migration
     Wave A-D close) — operator approved Ship Route (a) on the v0.2.0 ship:
     "Ship v0.2.0 as-is + backlog v0.3.0 for full wiring (Recommended)".
     The Wave A-D dev surfaced two gaps the operator explicitly accepted as
     ship-blocker-bypassed: (1) scope gap — only Momentum (2/34 scenarios) got
     real friction migration; 6 strategy paths (SMA/Composed, TCN, PatchTST,
     Pairs, VolTarget, GARCHVol) don't have LatencySlippageSimConfig wired into
     their construction sites, so their canonical SHAs = noop SHAs byte-identical;
     (2) data-source drift — Group A (5 SMA/Composed) canonical SHAs DIFFER from
     noop but ONLY due to synthetic→real-Binance auto-switch (env effect, not
     v5 sim). v0.3.0 closes both. -->

### Strategy

> **⚠️ STRATEGY RESEARCH CONCLUDED 2026-06-08 — SHIP PASSIVE. Every entry in this
> subsection is CONCLUDED / RETIRED / SUPERSEDED, NOT pending.** Across all three reachable
> channels (price/OHLCV, derivatives-positioning, on-chain) no active strategy beat passive
> buy-and-hold net of cost. Both remaining forks were tested and FAILED: the derivatives
> **market-neutral perp-basis spread** ran 2026-06-08 → FAMILY-UNIFORM-FRAGILE on all 12
> surfaces (domain CLOSED with finality), and the **on-chain** fork → PIT-infeasible / fragile.
> The robustness follow-ons (C3 shipped FRAGILE; C5 superseded by the 2026-06-15
> overfit-guard + bear-survey, both FRAGILE) and the v2.5 DL chain are retired. **No
> active-strategy bets remain.** Reconciliation:
> [`spec/dev-notes/backlog-staleness-audit-2026-06-15.md`](dev-notes/backlog-staleness-audit-2026-06-15.md).

<!-- updated 2026-06-06 (analyst, basis-reversal vehicle-vs-signal fork) —
     QUEUED `perp-basis-mn-spread` v0.2.0 (the RECOMMENDED next strategic bet),
     awaiting operator greenlight of the A-vs-B fork. -->

- **perp-basis-mn-spread v0.2.0 — market-neutral long/short basis spread**
  **🚫 CONCLUDED 2026-06-08 — DO NOT RE-RUN. Built + tested → FAMILY-UNIFORM-FRAGILE on all
  12 surfaces; derivatives-positioning domain CLOSED with finality (this is the test that
  produced "ship passive"). trace `REQ-PERP-BASIS-MN-SPREAD-001` = `tester-done`; 12
  robustness reports + `crates/backtest/tests/mn_spread_divergence_e2e.rs` on disk. Text
  below is the pre-build analyst rationale, kept as archaeology.**
  (pre-build framing, NOT current: "the RECOMMENDED next bet; ~5-8 dev-days; trace
  `REQ-PERP-BASIS-MN-SPREAD-001` state `proposed`"; brief at `spec/perp-basis-mn-spread/feature.md`). **Follow-on
  to `perp-basis-signal-robustness` v0.1.0** (closed PASS / FAMILY-UNIFORM-FRAGILE
  at all fees incl. 0bps gross). The adjudication
  (`spec/dev-notes/basis-reversal-vehicle-vs-signal-fork-2026-06-06.md`) found the
  long-only verdict is a **VEHICLE verdict, not a signal verdict**: the long-only
  arm carries full market beta + captures only the long-low-basis leg, benchmarked
  against BH +1.74 (35.7× gap); the fee-sweep **falsified fee-bleed** (p50 moves
  ~0.002 Sharpe across the {0,2,5,10}bps ladder — the killer is BETA, not fees). The
  market-neutral spread strips beta (null → ≈0, removing the 35× hurdle) + captures
  BOTH legs (the spike's full −0.10 IC). Runs THREE arms on the same paths
  (basis-spread / funding-spread / basis⊥funding) to resolve the **funding-confound**
  (+0.47/+0.66 overlap — is basis the funding mirror, or distinct alpha?). The
  short-leg funding-cost model **already exists** (`montecarlo.rs:325-363`, only line
  350's `continue` skip gates it); the bulk + dominant risk is the short-side engine
  in `run_path` (the FIRST run_path touch since v0.1.0 → the 107-anchor neutrality
  re-proof when `k_short=0`). ONE frozen-§0 change: BH control → dollar-neutral ≈0
  null (a CORRECTION, not a goalpost move). Pre-registered H0/H1 + k1/k2/k3 kill-
  criteria + 5 framed Qs (Q-MN-1..5) for the architect M-T1. **Recommended (A) over
  (B) on-chain** (durable-over-quick): tests a PROVEN-LIVE signal on HOURLY data
  already banked vs an UNMEASURED hypothesis on DAILY data (~730 pts/yr) not yet
  banked, at the same ~5-8d cost; resolves a standing question; de-risks on-chain
  either way. **If-budget-tightens:** Option A-lite spike ({0,5}bps × {2023,2024} = 4
  surfaces, gate the full build on clearing the ≈0 null). On-chain is the
  deferred-not-abandoned next domain IFF the spread is also FRAGILE. **Queued, not
  promoted** — awaiting operator greenlight of the A-vs-B fork. HANDOFF → architect
  M-T1 on greenlight.

  <!-- ON-CHAIN (the deferred fork option B): the #2-ranked orthogonal domain
       (`spec/dev-notes/new-data-domain-scoping-2026-06-05.md` § 3 domain B) —
       exchange net-flows / stablecoin supply / active addresses; FREE-ish, DAILY,
       full-history (DeFiLlama no-key + Glassnode/CryptoQuant free-tier daily+delayed);
       ~5-8 dev-days (new fetcher + PIT hygiene + daily adapter). NOT queued as the
       next bet — the analyst recommends (A) market-neutral FIRST (tests the
       proven-live signal in its correct vehicle; on-chain stays the pre-registered
       next domain IFF (A) is FRAGILE). Re-surface on an (A) FRAGILE verdict. -->

<!-- Monte-Carlo robustness lane — follow-on Queue (opened 2026-05-30, analyst M0).
     The first slice (C1 `monte-carlo-bootstrap-path-generator` + C2
     `strategy-robustness-harness`) is in Active. These three are the
     deliberately-NOT-promoted-yet follow-ons, sequenced per the operator's
     Q3 (learning loop LAST) and the direction note § 6 / architect § 6.2: -->

- **C3 — Monte-Carlo param-sweep runner** (robustness lane follow-on, ~3-5
  dev-days). Generalize the `threshold_sweep` (τ×ε) sweep into a generic
  `scenarios::sweep` over an arbitrary strategy param grid + per-family
  strategy builders (the registry already maps TOML→strategy). Cross with C1's
  bootstrap paths → a parameter-sensitivity surface + plateau-vs-peak verdict
  ("stable plateau = robust; sharp peak = fragile / curve-fit"). Reuses C2's
  determinism rules. **Queued, not promoted** — lands after C1+C2 prove the
  anchor-coexistence story. (Architect Phase MC-2.)
  <!-- BACKLOG-CANDIDATE 2026-05-30 (analyst): C3 SCOPED as a decision-grade
       brief at spec/momentum-parameter-robustness-sweep/feature.md (status:
       draft, v0.1.0) + tasks.md. Multiple-testing decision SETTLED = Option (a)
       (full θ-surface + family verdict; defer "best θ is robust" to C5 deflation;
       anti-cherry-pick by construction). Hypothesis-aimed grid = lookback × k_long
       × drift/hold-band, ~12-16 Tier-1 cells, ~10 min at N=500 (NOT 1 hr). ~85%
       reuse of the C2 harness. C3 narrowed from "generic scenarios::sweep over
       any family" to "momentum family only, v0.1.0" — generic sweep deferred.
       Reversible pre-greenlight: operator still chooses C3 vs pivot vs C5. Proposed
       trace row REQ-MOMENTUM-PARAMETER-ROBUSTNESS-SWEEP-001 staged in the brief
       (NOT yet in trace.toml — add on greenlight). Feeds the architect IFF C3 is
       picked. -->

- **C4 — Reflection-feedback decision seam (deterministic learning loop)**
  (robustness lane follow-on, ~5-8 dev-days). The highest-leverage but
  highest-architecture-risk pillar. A sanctioned pre-run selector (in
  `trader`/`agent`, NOT `strategy` — the t1809 gate keeps the strategy crate
  consumer-free) reads `retrieve_top_k` lessons and *configures* the strategy /
  prunes the param grid; a robustness run's distribution writes a summarizing
  `LessonCard`. Closes the half-open loop (write-mostly telemetry today; one LLM
  consumer; deterministic strategies architecturally walled off). **Sequenced
  LAST per operator Q3** so the loop consumes a real distribution and loop-
  determinism (deterministic retrieval ordering) does not entangle the first
  anchorable deliverable. Requires no LLM. (Architect Phase MC-4; direction note
  § 4.) Possible ADR if lessons feed an anchored report (retrieval determinism).

- **C5 — CPCV / Deflated-Sharpe overfit guard** (robustness lane follow-on,
  ~4-6 dev-days). Combinatorial Purged Cross-Validation (López de Prado) over the
  real path: many purged + embargoed train/test splits → a distribution of
  performance + Probability of Backtest Overfitting (PBO, target < 15%) +
  Deflated Sharpe. Orthogonal to C1 (CPCV perturbs the *partition*; the bootstrap
  perturbs *paths*) — the overfit guard the TCN τ×ε ship needed and lacked.
  Pure-analysis, no live-trade path. Consumes C1's generator. **Queued, not
  promoted.** (Architect Phase MC-3 sibling; direction note § 2.3.)

<!-- PROMOTED Queue → Active 2026-05-27 (analyst M0). Brief authored at
     spec/v5-latency-slippage-sim-v0.3.0-full-path-wiring/feature.md;
     tasks at spec/v5-latency-slippage-sim-v0.3.0-full-path-wiring/tasks.md;
     trace row REQ-V5-FULL-PATH-WIRING-001 at EOF of spec/trace.toml in
     proposed state. See Active section above for the live tracking row. -->
<!-- - **v5-latency-slippage-sim v0.3.0 (full-path wiring + data-source-drift decision + t1937 test refresh).**
  (Queue stub preserved as comment for archeology; live row now in Active.) -->

<!-- RETIRED 2026-05-21 — operator picked Q1=(b) retire. v25-tcn-overlay
     shipped_disposition records the F4 verdict. Stale Queue entry kept
     for archaeology + linked from Recent (shipped) cohort. -->
<!-- - **v2.5 TCN horizon-bump or retire (`v25-tcn-horizon-bump-or-retire`).**
  RETIRED 2026-05-21 (operator Q1=(b) retire). See Recent (shipped)
  for the v0.1.0 ship summary. v25-tcn-overlay frontmatter carries
  `shipped_disposition: F4 verdict — production deployment NOT
  recommended.` -->

- **v2.5 TCN horizon-bump or retire** — **RETIRED 2026-05-21**;
  see Recent (shipped) below. Operator Q1=(b) retire; v25-tcn-overlay
  has shipped_disposition F4 verdict (no production deployment).
  Replaced in DL roadmap by v2.5a PatchTST (shipped) + v2.5b vanilla
  Transformer (Queue, not started).

- **v2.5 alpha-verdict investigation** — **SHIPPED 2026-05-19**
  (`v25-tcn-alpha-investigation v0.3.0`); chained into
  `v25-tcn-recalibrate v0.1.0` (2026-05-21 σ-train metadata fix),
  `v25-tcn-threshold-tuning v0.1.0` (2026-05-21 9×5 τ,ε sweep, best
  +0.018/+0.045 Sharpe-delta T-MARGINAL), and finally
  `v25-tcn-horizon-bump-or-retire v0.1.0` (2026-05-21 operator Q1=(b)
  RETIRE). v2.5 TCN line is closed. Stale Queue entry purged
  2026-05-28; future strategy work routes through v2.5a PatchTST or
  v2.5b vanilla Transformer.

- **v2.5a — PatchTST forecast overlay (`v25a-patchtst-overlay`).** <!-- # noqa: queue-staleness — reconciled 2026-05-30: folder shipped (part of retired v2.5 DL chain); stale Queue→Active pointer stub; see Recent / Active. Stub-retirement deferred to operator backlog-triage. -->
  _moved Queue → Active 2026-05-21 (analyst pass)_ — see
  [Active section](#active) for the live tracking row and
  [`feature.md`](v25a-patchtst-overlay/feature.md) for the
  full v0.1.0 brief (R1-R10, H1-H4, K1-K6, Q1-Q8 with
  analyst-recommended defaults). MVP scope per Q2=(a):
  code + 1 trained BS-1 PatchTST checkpoint + F-verdict report
  + Sharpe-comparison + sibling strategy + 2 anchors under
  `v2.5a.0-patchtst`. Cost ~3-5 weeks best case; ~5-7 weeks with
  one Wave-B retry.
- **v2.5b — Vanilla decoder-only Transformer overlay
  (`v25b-transformer-overlay`).** **_RETIRED 2026-05-22_** — operator
  routing (a) at
  [`v25a-patchtst-overlay`](v25a-patchtst-overlay/feature.md) ship.
  Phase 3 of the [4-phase DL roadmap](v25-dl-forecast-overlay/feature.md);
  was activation-gated on phases 1+2 shipping. Phase 1 (TCN) shipped
  with F4-T-MARGINAL; phase 2 (PatchTST) shipped with F4-T-MARGINAL
  scoring LOWER than TCN. Joint F4-F4-F4 across 2 model families
  exhausted the prior for "next architecture family unlocks alpha";
  v2.5b vanilla decoder Transformer would not justify the ~3-5 week
  compute commitment. Stub feature folder
  [`spec/v25b-transformer-overlay/feature.md`](v25b-transformer-overlay/feature.md)
  preserved for archeology. Re-activation gated on a substantive
  reformulation of the forecast task itself (e.g. volatility / regime /
  168h trend) and operator-decide on whether DL paradigm tests deserve
  another budget allocation.
- **v2.6 — Forecast bake-off + retirement (`v26-forecast-bakeoff`).**
  **_RETIRED 2026-05-22_** — operator routing (a) at
  [`v25a-patchtst-overlay`](v25a-patchtst-overlay/feature.md) ship.
  Phase 4 of the [4-phase DL roadmap](v25-dl-forecast-overlay/feature.md);
  was the canonical retirement gate for the 4-phase DL umbrella. The
  bake-off premise (head-to-head TCN vs PatchTST vs vanilla Transformer)
  is moot now that 2 of 3 paradigms have F4'd. Stub feature folder
  [`spec/v26-forecast-bakeoff/feature.md`](v26-forecast-bakeoff/feature.md)
  preserved for archeology. The retirement decision itself is the
  bake-off — operator chose "all 3 v2.5 paradigms retire as F4
  no-alpha" rather than waiting for v2.5b to confirm.
- **Pre-pivot breadcrumb:** the dropped Kronos approach is preserved at
  [`spec/dev-notes/archive/2026-Q2/kronos-evaluation-2026-05-10.md`](dev-notes/archive/2026-Q2/kronos-evaluation-2026-05-10.md)
  [SUPERSEDED] as a what-not-to-do reference. **v2.5 DL umbrella also
  retired 2026-05-22 (F4-F4-F4 verdict + operator routing (a))** —
  [`spec/v25-dl-forecast-overlay/feature.md`](v25-dl-forecast-overlay/feature.md)
  preserved as another what-not-to-do reference (DL overlay at 1h/24h
  log-return horizon on hourly crypto bars does not extract +0.10
  Sharpe-delta on the v1 cross-sectional momentum baseline).

<!-- moved Queue → Active 2026-05-28 (operator Phase 2 re-pick after
     v2.5 TCN analyst-halt). See § Active above for live tracking. -->
<!-- - **v3 — Regime classifier (`v3-regime-classifier`).** — moved Active 2026-05-28. -->

- **v3 — LLM-as-forecaster (`v3-llm-forecaster`).** <!-- # noqa: queue-staleness — reconciled 2026-05-30: folder shipped-partial (Wave D deferred pending API key); stale Queue→Active pointer stub; see Recent / Active. Stub-retirement deferred to operator backlog-triage. -->
  _moved Queue → Active 2026-05-22 (analyst-bridge)_ — see
  [Active section](#active) for the live tracking comment block
  and [`feature.md`](v3-llm-forecaster/feature.md) for the full
  v0.1.0 brief (R1-R10, H1-H5, Q1-Q8 with analyst-recommended
  defaults, K1-K10 risk register, 8-item non-regression contract).
  Candidate 5 of three picks ({C1 volatility + C2 regime + C5
  LLM-as-forecaster}) from the
  [strategy-reformulation survey](dev-notes/archive/2026-Q2/strategy-reformulation-survey-2026-05-22.md#candidate-5--reflection-memory-as-forecaster-v2-llm-signal);
  promoted at C1's retirement (v3-volatility-forecaster programme
  shipped with NEGATIVE-NET-DELTA real evidence post-noop-fix).
  Picked over C2 v3-regime-classifier for moat-alignment
  ([product.md § Differentiator line 79-83](product.md#differentiator)
  — persistent reflection memory + auditable double-entry ledger
  is the long-term moat; C5 is the **only** survey row where the
  signal source IS the moat) + `crates/llm` infra reuse (no new
  infra needed; all v2-llm-strategy v2.0.0 surfaces intact).
  Cost ~6-8 weeks per analyst-bridge; HIGH variance per K8 novel-
  territory risk. The biggest product-differentiation surface is
  R9 Phase F right-rail Assistant slot body promotion (Q4=(c)
  operator-decide at T-OD4) — the operator *sees* the LLM
  reasoning + retrieved lesson cards + audit correlation live;
  if this lights up, the moat becomes operator-visible.

<!-- 2026-05-29 (analyst, v3-xgboost-cheap-classifier M0) — Queue entry
     per post-v3 strategy direction Route A pre-position. Brief authored
     at spec/v3-xgboost-cheap-classifier/feature.md (R1-R5 + R-NR + K1-K6 +
     H1-H3 + Q1-Q3 + 4-cell verdict tree + cost framing both routes);
     tasks at spec/v3-xgboost-cheap-classifier/tasks.md; trace row
     REQ-V3-XGBOOST-001 at EOF of spec/trace.toml in proposed state.
     Queue NOT Active — promote to Active only on operator explicit
     pick of Route A from post-v3-strategy-direction-2026-05-29.md.
     Pre-flight reconciliation confirmed no existing spec/v3-xgboost-*
     folder with shipped status. -->

- **v3-xgboost-cheap-classifier v0.1.0 (Candidate 6; non-DL model-class
  axis).** _Queue pre-position per post-v3 strategy direction Route A;
  promote to Active only on operator explicit pick_ — see
  [`spec/v3-xgboost-cheap-classifier/feature.md`](v3-xgboost-cheap-classifier/feature.md)
  for the v0.1.0 brief and
  [`spec/dev-notes/post-v3-strategy-direction-2026-05-29.md`](dev-notes/archive/2026-Q2/post-v3-strategy-direction-2026-05-29.md)
  for the route framing. Candidate 6 from the
  [2026-05-22 strategy-reformulation survey](dev-notes/archive/2026-Q2/strategy-reformulation-survey-2026-05-22.md#candidate-6--non-dl-approaches-hmm-kernel-methods-statistical-filters)
  — tests the opposite hypothesis to C1/C2/C5 retirement set:
  **low-capacity gradient-boosted trees may suit low-SNR hourly OHLCV
  better than DL/Markov/LLM by underfitting-by-design**. Asymmetric
  falsification: XGBoost ≥ baseline refutes "edge isn't in fancy model
  choice"; XGBoost ≤ baseline strengthens "edge isn't extractable from
  hourly OHLCV regardless of model class" → Route C pivot stronger.
  Either outcome information-bearing. Cost ~4-6 weeks DURABLE
  (Q1=(a) classifier + Q2=(a) overlay + Q3=(a) `xgboost` crate);
  ~2-3 weeks cheap fallback (Q1=(b) regressor + Q3=(c) pure-Rust) but
  trait-seam break → +1-2 week v0.2.0 cleanup → strictly worse on
  durability per AGENT.md 2026-05-29. R2 reuses frozen v0.1.0
  `RegimeClassifier` trait seam from `crates/forecast/src/markov_switching.rs`
  (v3 Wave A) — XGBoost impl satisfies same trait without amendment.
  R3 overlay-style multiplier on v1 momentum (different operator-lock
  per model class from C2's dispatcher). 4-cell verdict tree
  pre-drawn: V-XGB-PASS / V-XGB-CLASSIFIER-ONLY / V-XGB-DAMPENED /
  V-XGB-INCONCLUSIVE.

### UI / cockpit (Lumen design-system adoption — Phase 6 reserved)

<!-- updated 2026-05-27 (analyst, cockpit-toast-queue-v0.2.0-cleanup M0). Closes
     the v0.1.0 ship's architecture-deviation follow-on (developer kept the
     `pub toast_message: Option<SmolStr>` FIELD alongside the new
     `toast_queue: VecDeque<ToastEntry>` because `cockpit_training_pressed_wiring.rs`
     writes the field directly; annotated `// MIGRATION: remove at v0.2.0`).
     R1 remove the field; R2 migrate the 2 known field-write sites (both in
     one test file) to `Message::ShowToastWithSeverity` / `Message::ShowToast`
     dispatch; R3 audit + optionally remove the `toast_message()` method shim
     (developer-discretion sub-route, not operator-decide). Pure refactor —
     zero operator-visible behaviour change. No Qs (standing-Autoapprove-
     eligible). Cost ~2-4 hours wall-clock. Pre-verified at analyst pass:
     `grep -rn "toast_message" crates/` confirms exactly the 2 field-write +
     5 method-read sites in the K5 test file; production `cockpit_live.rs`
     already routes via the message API. Trace row
     REQ-COCKPIT-TOAST-QUEUE-CLEANUP-001 opened at `proposed` state. -->
<!-- moved to Recent (shipped) — v0.1.0 operator-approved 2026-05-28 -->
<!-- - **cockpit-toast-queue v0.2.0 cleanup** — see Recent section below for v0.1.0 ship summary. -->

- **Lumen Phase 6 — Assistant slot.** _reserved_ — depends on the
  v2 LLM strategy queued item above. Right-rail collapsible panel
  for the v2 LLM assistant per
  [`spec/design/project/ui_kits/desktop/Assistant.jsx`](archive/design-prototypes-2026-Q2.tar.gz).
  Phase 2 reserved the right-rail column-track in the shell grid
  at `Length::Fixed(0.0)`; the actual Phase 6 brief lands when v2
  LLM is approved. Until then, no analyst spawn. Stub at
  [`features/lumen-phase-6-assistant-slot.md`](lumen-design-adoption/phase-6-assistant-slot/feature.md).
  _(Phases 1–5 of the lumen-design-adoption initiative are shipped
  and live in the Recent section; this Queue entry is the only
  remaining initiative work, gated on v2 LLM.)_

- **Cockpit Windows / Linux support (`cockpit-cross-platform`) — SOURCE SHIPPED;
  CI verification DEFERRED to NEAR PROJECT COMPLETION.** _surfaced 2026-05-12
  (operator D3); SCOPED + BUILT 2026-06-15 (analyst→architect→developer; ADR-0057;
  trace `REQ-COCKPIT-CROSS-PLATFORM-001` state `dev-done`)._ The portability source
  layer shipped this session and is **macOS-verified**: errno fix
  (`std::io::Error::last_os_error`), `windows="=0.57.0"` target stanza,
  reqwest→rustls-tls flip, the 4 snapshot test files `#![cfg(target_os="macos")]`-gated
  (visual baselines stay macOS-canonical per ADR-0057), 56 baselines byte-identical,
  anchors 119/119. The 3-OS CI matrix YAML (ubuntu/macos/windows) is written and
  **parked INERT** at `.github/workflows/ci.yml.deferred` (GitHub does not run a
  `.deferred` file). **⏸ MILESTONE (operator, 2026-06-15): activating the CI matrix
  — `git mv ci.yml.deferred ci.yml`, which is the actual Linux/Windows verification —
  is DEFERRED to NEAR PROJECT COMPLETION**, not near-term (push starts GitHub Actions
  on the repo + runs on every push/PR). Until that milestone: do NOT activate CI;
  Linux/Windows builds stay unverified-by-CI. Activation trigger = operator, at the
  near-done milestone → first CI run → tester signs off T-T1..3. _vendor/iced_tiny_skia
  was confirmed OS-agnostic this session — no fork change (operator-lock upheld)._

### Process / tooling

- **`lab-recipe-test-harness v0.3.0+` extension** (Recipe candidate
  list pre-positioned). _candidate, Wave 2 of Pick A test-infra trifecta;
  gated on v0.2.0 Wave A→D ship_ — Wave 1 of the trifecta
  ([`spec/dev-notes/pick-a-test-infra-trifecta-2026-05-29.md`](dev-notes/archive/2026-Q2/pick-a-test-infra-trifecta-2026-05-29.md))
  promotes visual-fail-HTML + viewport-matrix in parallel. The Wave 2
  harness v0.3.0+ analyst spawn happens **AFTER** the in-flight v0.2.0
  ships (per dev-note § Sequencing — scope discovery depends on v0.2.0
  Wave A→D outcomes; ADR-0048 § Changelog needs the v0.2.0 row before
  v0.3.0 amends again). Candidate Recipes sketched in the dev-note
  § "What harness v0.3.0+ candidate list looks like":
  (1) **`LessonCardRecipe`** — K4 byte-identity coverage for
  `crates/reflection/src/lesson_cards/`; ~2 dev days; closes the
  RegimeTag-deletion-class regression class for lesson-card
  artifacts.
  (2) **`BacktestProgressRecipe`** — backtest UI tracking; ~2-3 dev
  days; closes the Bug #64 `tokio::select!` shape on the backtest
  progress channel (currently uncovered).
  (3) **`TrailMirrorRecipe` Surface 1 extension** — extends v0.2.0
  Wave D's S2-only coverage with the select-arm-survival assertion
  pattern from v0.2.0 Wave B; ~0.5 dev days. Combined ~5-6 dev days
  + 1.25 tester days ≈ ~1.5 weeks wall-clock. Same shape as v0.2.0
  (per-Recipe-specific mocks, per-Recipe T-T4 falsification probe in
  module docstring, zero new ADRs, zero anchor delta). **Analyst
  spawn trigger**: v0.2.0 tester emits `VERDICT → PASS` AND ADR-0048
  § Changelog has the v0.2.0 row. **NO** Queue → Active promotion
  before that gate.

- **`v2x-trading-state-bus`** (v2 LLM evolution) —
  _candidate, sourced from
  [`spec/dev-notes/external-code-patterns-2026-05-17.md`](dev-notes/archive/2026-Q2/external-code-patterns-2026-05-17.md)_.
  Replace ad-hoc parameter threading in the v2 LLM agent pipeline with
  an owned `TradingState { fundamentals, sentiment, news, technical,
  debate: Vec<Argument>, count: u32, … }` struct that each agent
  destructures, mutates its slice, and passes on. Mirrors
  [TradingAgents'](https://github.com/TauricResearch/TradingAgents)
  LangGraph state-dict pattern but in Rust. Bull/Bear adversarial
  researcher pattern (see below) plugs in cleanly. Not a v2.0 ship
  enhancement (v2 already shipped 2026-05-13); a v2.1 or v2.2
  feature. Analyst spawn when operator chooses.

- **`v26-bakeoff-llm-arbiter`** (v2.6 enhancement) —
  _**RETIRED-by-context 2026-05-29** (process-tooling-survey § Stale-flag
  findings) — parent `v26-forecast-bakeoff` v2.6 retired 2026-05-22 per
  v2.6-no-alpha verdict. With no parent to enhance, the LLM-arbiter
  candidate has nothing to plug into. Original sourcing from
  [`spec/dev-notes/external-code-patterns-2026-05-17.md`](dev-notes/archive/2026-Q2/external-code-patterns-2026-05-17.md);
  pattern (bull/bear arbitration over multiple DL forecasters) remains
  available for any future bake-off-flavored feature, but is no longer
  Queue-positioned._

- **v2.1 — Cockpit LLM-budget tile + pedantic clippy cleanup
  (`v2-llm-strategy-v21-followups`) — REDACTOR PORTION SPLIT OFF
  2026-05-29.**
  _candidate, surfaced 2026-05-13 by v2-llm-strategy v2.0.0 ship;
  partially promoted 2026-05-29 — see Active section above for the
  split-off `v2-1-tracing-layer-redactor v0.1.0` brief under Pick B
  Wave 1 of the cross-cutting safety duo strategic direction at
  [`spec/dev-notes/pick-b-cross-cutting-safety-duo-2026-05-29.md`](dev-notes/archive/2026-Q2/pick-b-cross-cutting-safety-duo-2026-05-29.md)_
  — two REMAINING deferred items (the redactor half was split off
  and promoted under Pick B Wave 1):
  (a) **T1938 cockpit "LLM budget" tile** — was deferred in pass 6
      because its dependency `audit::query::llm_spend_this_month`
      isn't implemented. v2.1 ships the audit query helper +
      the right-rail tile (three-color thresholds: green < 60%,
      amber 60-80%, red ≥ 80%; auto-degrade at 80% per Q6).
      **Defer with v2 LLM lane activation** per
      [`process-tooling-survey-2026-05-29.md § What's NOT a compounder`](dev-notes/archive/2026-Q2/process-tooling-survey-2026-05-29.md)
      honorable mentions — gates Lumen Phase 6 Assistant, no
      independent compounder benefit until v2 LLM lane re-activates.
  ~~(b) **T1915 tracing-Layer redactor half**~~ — **PROMOTED
      2026-05-29 to Active section above as standalone
      `v2-1-tracing-layer-redactor v0.1.0` (~1.5 dev days)** under
      Pick B Wave 1 cross-cutting safety duo. The Layer wire-up is
      cross-cutting safety that compounds regardless of v2 LLM lane
      state — no reason to defer with the rest of #3. See Active
      row for full promotion annotation.
  (c) **T1910 pedantic clippy cleanup** — 2 `cast_possible_truncation`
      warnings on `crates/audit/src/query.rs:219, 221` from the
      `cache_hit_ratio_since` query. Non-blocking per v2.0.0
      brief §Critical constraints #2. v2.1 cleans these up via
      `Decimal::try_from` or explicit clamp. Roll into next audit's
      housekeeping per the survey § What's NOT a compounder framing.
  Analyst spawn for the REMAINING (a)+(c) items when operator
  promotes; not urgent. The split-off (b) is now Active under its
  own brief.

- **Canvas-state seeding for snapshot tests
  (`ui-test-harness-canvas-state-seeding`).** _candidate, surfaced
  2026-05-12 by H2 operator review on `ui-test-harness-bootstrap`
  v0.1_ — closes the **render** half of `chart-canvas-overhaul` V15
  (V8 in the bootstrap brief). The v1.9.0 T2033 refactor decoupled
  tooltip rendering from `Cockpit.chart_tooltip` — the canvas reads
  hover state from its internal `ChartProgram::State`, not from
  `self.tooltip` — so the Q9 fixture's
  `Cockpit.chart_tooltip = Some(...)` has zero effect on the
  rendered PNG.  Scope: extend
  [`crates/ui/src/test_support.rs`](../crates/ui/src/test_support.rs)
  with `seed_canvas_hover_state(idx)` injecting state directly into
  `ChartProgram::State` before `iced_test::screenshot` runs. Two
  viable paths: (a) `iced_test::Simulator::send_event` to dispatch
  a synthetic `CursorMoved` over the marker centroid; (b) a
  `#[doc(hidden)]` test-only `ChartProgram::with_seeded_hover_state(
  idx, centroid) -> Self` constructor.  Acceptance: a new
  `charts_screen_dark_operator_hover.png` baseline shows the
  tooltip card next to a hovered marker; existing three baselines
  stay byte-identical (V15 detection + render fully closed).
  Analyst spawn after v0.1 ships.

- **Week-2 follow-up — full-widget viewport matrix
  (`ui-test-harness-viewport-matrix`).** _**PROMOTED Queue → Active
  2026-05-29**_ under Pick A Wave 1 of the test-infra trifecta
  strategic direction at
  [`spec/dev-notes/pick-a-test-infra-trifecta-2026-05-29.md`](dev-notes/archive/2026-Q2/pick-a-test-infra-trifecta-2026-05-29.md).
  Extends the v0.1 Charts-only three-viewport snapshot harness across
  ALL widget tests (panels, modals, status bar, agent feed, debug
  screen) per
  [dev-note §6 week 2](dev-notes/archive/2026-Q2/ui-testing-direction-2026-05-12.md#6-phased-adoption--4-week-plan).
  Brief: [`spec/ui-test-harness-viewport-matrix/feature.md`](ui-test-harness-viewport-matrix/feature.md).
  See Active section above for the promotion annotation.

- **Week-3 follow-up — evaluator subagent + PreToolUse hooks
  (`ui-test-harness-evaluator`).** _candidate, gated on
  `ui-test-harness-bootstrap` v0.1 ship_ — splits the tester role
  into test-runner (writeable) + evaluator (read-only, fresh
  context, default-FAIL PreToolUse hook) per
  [dev-note §4.2](dev-notes/archive/2026-Q2/ui-testing-direction-2026-05-12.md#42-default-fail-evaluator-subagent)
  and
  [`AGENT.md ## Test-runner / evaluator split`](../AGENT.md#test-runner--evaluator-split).
  Wires the PreToolUse hooks for `screencapture`, `osascript`, and
  `./target/release/cockpit` denying sub-agents (allowing
  orchestrator). Analyst spawn after v0.1 ships.

- **Week-4 follow-up — GitHub Actions CI + presenter integration <!-- # noqa: queue-staleness — reconciled 2026-05-30: cross-refs shipped (`ui-headless-emulator`); this entry is the ui-test-harness-ci candidate, see Recent for the emulator. -->
  (`ui-test-harness-ci`).** _candidate, gated on
  `ui-test-harness-viewport-matrix` + `ui-test-harness-evaluator`
  ship_ — macOS runner workflow uploading baseline+actual+diff PNG
  triples on visual snapshot failures; presenter deck format gets a
  fixed "screenshot artifacts" section pointing at the CI artifact
  URL per [dev-note §6 week 4](dev-notes/archive/2026-Q2/ui-testing-direction-2026-05-12.md#6-phased-adoption--4-week-plan).
  **Per [`ui-testability-deep-dive-2026-05-15.md §5.3`](dev-notes/archive/2026-Q2/ui-testability-deep-dive-2026-05-15.md#53-keep--drop--replace-against-the-existing-weeks-2-4-plan)**
  the analyst recommends pairing this CI brief with the 1-day
  cross-platform falsifier (item O) to retire or confirm operator
  decision D3 (macOS-only CI). **CHEAPENED 2026-05-15:** down to
  ~4 dev-days (from 5) per
  [`iced-014-feature-analysis-2026-05-15.md §4`](dev-notes/archive/2026-Q2/iced-014-feature-analysis-2026-05-15.md#headless-mode).
  iced 0.14's `iced_test::emulator::Emulator` (PR #2698) ships
  embedded Fira Sans + a real headless runtime, so we don't need
  to author font-fallback / xvfb plumbing. **FURTHER DECOMPOSED
  2026-05-16:** the Emulator adapter portion shipped standalone as
  [`ui-headless-emulator` v0.1](ui-headless-emulator/feature.md);
  remaining scope is CI workflow + cross-platform falsifier only.

- **comet debugger revisit trigger (`ui-comet-eval`).** _candidate,
  REVISIT-GATED 2026-05-16 by operator decision_ — Q-COMET-EVAL
  LOCKED → defer indefinitely STILL APPLIES. Three revisit triggers
  (any one fires re-evaluation): **(a)** our iced pin moves to 0.15.x,
  OR **(b)** `ui-inspect-mcp` / `ui-session-journal-iced-tester`
  surface a gap comet would close, OR **(c)** 2026-11-15 calendar
  6-month revisit. (Trigger (a) supersedes the 2026-05-16 operator-
  added "iced 0.15.0 stable" gate by being strictly looser.) Until
  any of the three fires: no spawn trigger, no schedule. See
  [`iced-014-feature-analysis-2026-05-15.md §3`](dev-notes/archive/2026-Q2/iced-014-feature-analysis-2026-05-15.md#comet-debugger)
  for the original analysis. **Duplicate L2488 entry collapsed into
  this row 2026-05-29 per process-tooling-survey § Stale-flag
  findings.**

  > **Attempted + aborted 2026-05-16.** Operator authorized the
  > full bump (Q-014-PIN + Q-COMET-EVAL override). Two strikes
  > surfaced before abort: (1) js-sys dep conflict (resolved with
  > iced-master's Cargo.lock copy); (2) **23 iced 0.14→0.15 API
  > churn errors in iced_aw alone**, with the same patterns
  > (`Theme::extended_palette`, `Text` gained `ellipsis`+`hint_factor`
  > fields, `Palette.text` removed, `Widget::update` arity changes,
  > `Font::with_name` removed, `Overlay::update` arity change)
  > guaranteed to recur across our ~30 widget/screen files.
  > **iced_aw + iced_fonts BOTH still on iced 0.14**, confirming
  > the ecosystem has not migrated yet. Revised cost estimate
  > climbed 5-9d → 6-11d before mandatory-stop. Aborted at the
  > operator's "Stop now" choice. Vendor work reverted (was
  > uncommitted). The revisit trigger above remains the correct
  > path — wait for iced 0.15.0 stable + iced_aw / iced_fonts
  > ecosystem migration.

- **Operator UI legibility — WCAG contrast asserter
  (`ui-contrast-asserter`).** _**PROMOTED Queue → Active
  2026-05-29**_ under Pick B Wave 1 of the cross-cutting safety duo
  strategic direction at
  [`spec/dev-notes/pick-b-cross-cutting-safety-duo-2026-05-29.md`](dev-notes/archive/2026-Q2/pick-b-cross-cutting-safety-duo-2026-05-29.md)
  (cheap pillar of the duo, ~0.5 dev days). Originally surfaced
  2026-05-15 by
  [`spec/dev-notes/ui-testability-deep-dive-2026-05-15.md §3.8`](dev-notes/archive/2026-Q2/ui-testability-deep-dive-2026-05-15.md#38-stretch--pure-rust-wcag-contrast-asserter--ui-contrast-asserter).
  Brief: [`spec/ui-contrast-asserter/feature.md`](ui-contrast-asserter/feature.md).
  See Active section above for the promotion annotation. WARN mode
  at v0.1.0 (2-week observation per bundle Q-DUO-WARN) before
  v0.2.0 patch flips default to gate.

- **Pure-state property tests — update + proptest harness
  (`ui-update-proptest`).** _candidate, surfaced 2026-05-15 by
  [`ui-testability-deep-dive-2026-05-15.md §3.4`](dev-notes/archive/2026-Q2/ui-testability-deep-dive-2026-05-15.md#34-stretch--update--property-based-state-machine-harness--ui-update-proptest)_
  — drive `ui::state::update` with
  [`proptest-state-machine`](https://crates.io/crates/proptest-state-machine)
  over randomized `Message` sequences. Five invariants to start:
  kill monotonicity, no cross-screen state leakage, PanelState arm
  reachability, subscription-error recoverability, audit-write
  idempotency. ~5 dev-days. Closes ~40 `Message` variants currently
  not directly covered (analysis at
  [`ui-testability-deep-dive-2026-05-15.md §2.10`](dev-notes/archive/2026-Q2/ui-testability-deep-dive-2026-05-15.md#210-state-invariant-tests-vs-view-tests--quantifying-the-gap)).
  Analyst spawn when operator promotes; pairs naturally with
  `ui-mutants-pass` below.

- **Storybook-equivalent widget gallery bin
  (`ui-gallery-bin`).** _v0.1-partial shipped 2026-05-15_ —
  V1-V4 green (build, smoke, widget cell exhaustiveness, mod-rs
  cross-check). V5+ snapshot tests blocked on the iced Table cell-
  bounds panic — see [`ui-gallery-bin/tasks.md` ## Status](ui-gallery-bin/tasks.md)
  and the follow-up `ui-gallery-table-cell` candidate below.
  Original brief: [`ui-testability-deep-dive-2026-05-15.md §3.3`](dev-notes/archive/2026-Q2/ui-testability-deep-dive-2026-05-15.md#33-widget-gallery-binary--ui-gallery-bin).

- **iced 0.14 Table cell-bounds fix
  (`ui-gallery-table-cell`).** _candidate, surfaced 2026-05-15 by
  the [`ui-gallery-bin` v0.1-partial deliverable](ui-gallery-bin/design.md#changelog)_
  — `widgets::strategies::view` uses `iced::widget::table::Table`,
  which produces a degenerate quad and panics `iced_tiny_skia` at
  `engine.rs:686` ("Build quad rectangle") when rendered inside a
  fixed-height `gallery::cell::view` Container. Diagnostic at
  [`crates/ui/tests/gallery_bisect.rs`](../crates/ui/tests/gallery_bisect.rs)
  pinpoints `GALLERY_CELLS[7]`. Two paths: (a) special-case the
  strategies cell wrapper to drop the height constraint, or (b)
  swap strategies for a non-table render in the gallery only. ~1
  dev-day either way. Unblocks V5+ of `ui-gallery-bin` plus any
  future gallery cells that wrap table-based widgets. Analyst spawn
  when operator promotes.

- **AccessKit shadow tree + kittest assertions
  (`ui-a11y-shadow`).** _candidate, surfaced 2026-05-15 by
  [`ui-testability-deep-dive-2026-05-15.md §3.5`](dev-notes/archive/2026-Q2/ui-testability-deep-dive-2026-05-15.md#35-stretch--accesskit-shadow-tree--ui-a11y-shadow)_
  — author `crates/ui/src/a11y.rs` emitting an
  [`accesskit::TreeUpdate`](https://docs.rs/accesskit) for the
  cockpit's widget surface; wire to
  [`kittest`](https://docs.rs/kittest/) for tree-based assertions
  that render zero pixels. Establishes a Layer 2 "widget tree"
  oracle (§2.14 of the dev-note) that catches half the failure
  classes pixel-diff misses (contrast, reachability, focus, label
  drift). ~7 dev-days. **Approach B (in-repo shadow), not
  Approach A (PR iced upstream)** per
  [dev-note §2.7](dev-notes/archive/2026-Q2/ui-testability-deep-dive-2026-05-15.md#27-accessibility-as-a-testing-surface--the-load-bearing-pivot)
  + Q-ACCESSKIT default. Iced upstream
  [issue #552](https://github.com/iced-rs/iced/issues/552)
  remains unmerged as of May 2026.

- **VLM-as-second-opinion judge (`ui-vlm-judge`).** _candidate,
  surfaced 2026-05-15 by
  [`ui-testability-deep-dive-2026-05-15.md §3.2`](dev-notes/archive/2026-Q2/ui-testability-deep-dive-2026-05-15.md#32-vlm-as-second-opinion-judge--ui-vlm-judge)_
  — bolt a Claude Sonnet 4.6 vision-as-oracle onto the existing
  `crates/ui/tests/fixtures/` test infrastructure, runs on
  `matches_image` failure only as a second-opinion forensic. Three
  locked claims: tooltip visibility, no-overlap, contrast ≥ 4.5:1
  per text label. Pinned model + prompt SHA + N=3 majority vote.
  Reuses [`crates/llm`](../crates/llm) provider trait +
  `BudgetedProvider` cap ($0.50/test run). **Mandatory 2-week
  shadow-mode period before any gating;** see dev-note §3.2 (d)
  for the flakiness mitigation. ~3 dev-days. Analyst spawn when
  operator answers Q-VLM (default: adopt for shadow only).

- **Live-cockpit inspect MCP shim (`ui-inspect-mcp`).**
  _candidate, surfaced 2026-05-15 by
  [`ui-testability-deep-dive-2026-05-15.md §3.1`](dev-notes/archive/2026-Q2/ui-testability-deep-dive-2026-05-15.md#31-live-inspect-mcp-shim--ui-inspect-mcp)_
  — feature-gated read-only MCP server inside `cockpit` /
  `cockpit_live` exposing `get_widget_tree()`, `screenshot()`,
  `find_by_label(s)`, `get_widget_bounds(id)`. Listens on
  `127.0.0.1:<port>` + env-var-supplied auth token; off in
  production. Mirrors
  [Slint's testing backend MCP server](https://docs.rs/i-slint-backend-testing/latest/i_slint_backend_testing/).
  Lets the orchestrator (capability-map owner of cockpit launches
  per [AGENT.md ## Capability boundaries](../AGENT.md#capability-boundaries))
  answer structural questions about a running cockpit without a
  manual `Cmd+Shift+4`. ~4 dev-days. Defer to cycle 4+ per dev-note
  §5.2 — close more tactical coverage first. Analyst spawn when
  operator answers Q-MCP (default: defer).

- **Recorded session journal — iced_tester adapter
  (`ui-session-journal-iced-tester`).** _**v0.1 SHIPPED 2026-05-16**
  ([feature.md](ui-session-journal-iced-tester/feature.md) ·
  [tasks.md](ui-session-journal-iced-tester/tasks.md)) — RESCOPED
  2026-05-15 by
  [`iced-014-feature-analysis-2026-05-15.md §5`](dev-notes/archive/2026-Q2/iced-014-feature-analysis-2026-05-15.md#recorder--emulator--iced_testsimulator)_
  — supersedes the original 4-dev-day `ui-session-journal`
  candidate. iced 0.14 already ships `iced_tester` (PR #3059) +
  `.ice` text format for record/replay. Adapter work is: enable the
  `record-tests` cargo feature
  ([Q-TESTER-FEATURE LOCKED](dev-notes/archive/2026-Q2/iced-014-feature-analysis-2026-05-15.md#migration-questions-for-the-operator)),
  add `--record-tests` boolean flag to `cockpit_live`, wire
  `iced_tester::attach()` around the existing `iced::application(...)`
  call, and add a `tests/journal_replay.rs` walker over committed
  `.ice` files. Export path is **operator-driven via `rfd` native
  file dialog** (spike-corrected from the original `--record-tests
  <path>` plan; see [feature.md ## Design](ui-session-journal-iced-tester/feature.md#design)).
  **~1 dev-day / 6.5 hours.** Two open architect questions remain
  (Q-ARCH-1 builder composition, Q-ARCH-2 replay signature) — both
  carry 15-min M0 spikes before code lands. Status: queued, no spawn
  trigger pulled. Developer pipeline at operator promotion.

- **Mutation testing one-shot pass (`ui-mutants-pass`).**
  _candidate, surfaced 2026-05-15 by
  [`ui-testability-deep-dive-2026-05-15.md §3.7`](dev-notes/archive/2026-Q2/ui-testability-deep-dive-2026-05-15.md#37-stretch--mutation-testing-pass--ui-mutants-pass)_
  — one-time `cargo mutants --package ui --file
  crates/ui/src/state.rs` run. Produces a triage report of
  surviving mutants in `ui::state::update`. Pairs with
  `ui-update-proptest` above — proptest writes invariants;
  cargo-mutants surfaces which arms still have surviving mutants.
  ~1 dev-day for the run + triage. Quarterly cadence after the
  one-shot lands. Analyst spawn after `ui-update-proptest` shadow
  bedding-in (operator Q-MUTANTS-CADENCE default: one-shot, then
  quarterly).

- **Test reporter — visual-fail HTML artifact (agent contract
  update; slug `visual-fail-html-reporter`).** _**PROMOTED Queue →
  Active 2026-05-29**_ under Pick A Wave 1 of the test-infra trifecta
  strategic direction at
  [`spec/dev-notes/pick-a-test-infra-trifecta-2026-05-29.md`](dev-notes/archive/2026-Q2/pick-a-test-infra-trifecta-2026-05-29.md).
  Surfaced 2026-05-15 by
  [`ui-testability-deep-dive-2026-05-15.md §4.1`](dev-notes/archive/2026-Q2/ui-testability-deep-dive-2026-05-15.md#41-testermd--emit-a-structured-fail-artifact-not-just-prose).
  On any visual-assertion FAIL the helper writes a self-contained
  `target/visual-diff/<test>-<ts>.html` (default; opt-in `spec/<slug>/
  reports/...` via `EMIT_VISUAL_FAIL_TO_SPEC=1`) with inline baseline +
  actual + perceptual-diff PNG triple + assertion location + assertion
  body + optional VLM verdict slot. ~1 dev day; ~50-80 LoC helper +
  `.claude/agents/tester.md` stanza amendment (owned by this brief for
  the Wave 1 trifecta bundle). Brief:
  [`spec/visual-fail-html-reporter/feature.md`](visual-fail-html-reporter/feature.md).
  See Active section above for the promotion annotation.

- **File the iced strategies-Table tiny-skia panic upstream <!-- # noqa: queue-staleness — reconciled 2026-05-30: cross-refs shipped (`ui-gallery-bin`); this entry is the upstream-panic-filing candidate, see Recent for the gallery feature. -->
  (`ui-iced-table-panic-upstream`).** _candidate, surfaced
  2026-05-15 by
  [`iced-014-feature-analysis-2026-05-15.md §6`](dev-notes/archive/2026-Q2/iced-014-feature-analysis-2026-05-15.md#the-strategies-table-panic)_
  — minimal repro is already in-tree at
  [`crates/ui/tests/gallery_bisect.rs`](../crates/ui/tests/gallery_bisect.rs).
  Extract the minimal repro, file an issue against
  [iced-rs/iced](https://github.com/iced-rs/iced/issues) modeled
  on prior art issue
  [#2311](https://github.com/iced-rs/iced/issues/2311) (closed via
  PR [#2364](https://github.com/iced-rs/iced/pull/2364)), cite our
  `gallery_bisect.rs` line ranges. **Q-PANEL-UPSTREAM LOCKED:**
  bug report only, no fix PR. ~0.5 dev-day. Revisit by 2026-06-26
  if no upstream activity. Unblocks V5+ of
  [`ui-gallery-bin`](ui-gallery-bin/feature.md) eventually (via
  upstream); the in-tree `ui-gallery-table-cell` workaround above
  unblocks sooner. Analyst spawn at operator promotion.

_(historical: the only previously queued item, the presenter smoke test against
operator-success-reports, ran 2026-05-08; surfaced 4 findings, two
of which became skill-plumbing fixes that shipped in commit
`8b139c2`. See Recent below.)_

## Recent (shipped)

Cohorts through 2026-06-08 are archived in
[archive/backlog-recent-2026-05.md](archive/backlog-recent-2026-05.md)
(2026-06-11 cleanup sweep, `CLEANUP-PLAN.md` P2-3). New shipped entries
land below as features ship.

### 2026-06-15 — close-out: contrast asserter enforcing gate (ui-contrast-asserter v0.2.0)

- **ui-contrast-asserter v0.1.0 + v0.2.0 SHIPPED** (operator-approved 2026-06-15) —
  the cockpit's WCAG 2.1 contrast asserter (83 fg/bg Lumen pairs vs AA 4.5:1).
  v0.1.0 (the 83-pair gate, tester PASS 2026-05-29) was never presented; v0.2.0
  flips the default from advisory WARN to an ENFORCING gate — new sub-AA color
  pairings now FAIL the build (`UI_CONTRAST_MODE=warn` is the local-dev/CI-pin
  opt-out). The 6 known sub-AA pairs (5 light + 1 dark) ratified as documented
  OPT_OUTs (operator path A — zero color change, zero visual-baseline churn; 4 are
  trivially darkenable in a future palette-tune). Proven enforcing via a
  white-on-grey falsification panic. Test-tooling only; un-anchored. Pre-existing
  flaky `charts_screen_dark` tests + an `--all-features` deprecation surfaced
  during the close-out are spawned out-of-scope (task_23647c48).

### 2026-06-15 cohort — robustness double-confirm (overfit-guard + bear-survey) + Binance 2021-22 hourly corpus

- **simple-strategy-bear-survey SHIPPED** (operator-approved 2026-06-15) —
  two-stage stress-test of ship-passive on the deep 2021-22 bear corpus. Stage 1
  found 40 apparent winners (all 2022; strategies that sat out the crash, up to
  SOL-2022 RSI "beating" a −94.2% buy-and-hold by +97pp); Stage 2 (N=500
  block-bootstrap, frozen § 0 rule) scored ALL 16 top candidates FRAGILE — the
  apparent bear edge is path-luck, not robust. Up-market contrast (SOL-2021 SMA)
  MARGINAL → discriminates. FIRMS ship-passive on the deepest/widest bear evidence;
  the 2026-06-08 terminal verdict stands. Un-anchored #[ignore] harness; dev-note
  `spec/dev-notes/analysis-2026-06-15-simple-strategy-bear-survey.md`.

- **simple-strategy-overfit-guard SHIPPED** (operator-approved 2026-06-15) —
  N=500 block-bootstrap robustness guard on the survey's down-market
  trend-following "hedge" (AVAX/DOT 2024). Result: **all 9 cells FRAGILE** (p5
  Sharpe < 0) — SMA/MACD median paths are positive but the bad-luck tail is
  negative, so the hedge is **path-fragile**, not a real strategy property.
  Revises the survey's Finding 1 + the passive-baseline runbook: **ship-passive
  is now UNQUALIFIED** on this evidence (reinforces the 2026-06-08 terminal
  verdict). Analysis-only — un-anchored `#[ignore]` harness, zero trading-behavior
  change. Dev-note: `spec/dev-notes/analysis-2026-06-15-simple-strategy-overfit-guard.md`.

- **binance-corpus-expansion SHIPPED** (ADR-0056; operator-approved 2026-06-15) — adds
  `data/binance-2122/` sibling corpus root: 10-symbol Binance OHLCV, 2021-2022
  hourly, 240 parquet files, ~5.5 MB on disk, gitignored except the
  `REVISION.toml` pin. Provides the down-market depth missing from the 2023-24
  corpus (2022 = deep crypto bear: BTC −64%, ETH −67%). `data/binance` pin
  `3a8b96c4…` byte-identical; 119/119 anchors green.

  **Canonical reproduce command:**
  ```bash
  cargo run -p data --bin fetch_binance_klines -- \
    --symbols BTCUSDT,ETHUSDT,BNBUSDT,SOLUSDT,XRPUSDT,ADAUSDT,DOGEUSDT,AVAXUSDT,DOTUSDT,LINKUSDT \
    --start 2021-01-01 --end 2022-12-31 --interval 1h \
    --out data/binance-2122 --emit-revision-manifest
  ```
  **Pinned aggregate SHA:** `4f3906222cbca90c4188443f9a09440c2b7cb72a3a1fa40b7f7598b3fad22a62`

  Downstream follow-on: survey re-run over 2021-22 to re-test the
  trend-following-hedge finding — and per the overfit-guard result it must test
  PATH-ROBUSTNESS (block-bootstrap), not just point returns (out of scope here;
  queue when ready).

### 2026-06-12 → 2026-06-13 cohort — live trading removed; Lab → real-data strategy-checking tool

- **Live execution REMOVED from the project** (`c9c4561`, operator decision
  2026-06-12). The full live-money program (scoped → architected ADR-0054 →
  built to F1 `live-exec-client-binance-spot`, tester PASS) was reverted the
  same day at the operator's direction — "no live trading for a long time."
  Method: restore every touched file to the pre-live `a063d79` + delete the
  live files (byte-exact; recoverable from git history at `edbbb10`/`dc3ef58`).
  ADR-0054 withdrawn (next NEW ADR = 0055). **KEPT in scope:** real market data
  (Binance read-only feed, Yahoo, parquet) for backtesting, paper simulation,
  the cockpit Live view. Record: `spec/dev-notes/live-trading-removed-2026-06-12.md`.
- **lab-run-save-compare SHIPPED** (ADR-0055; `e13cb6c`; operator-approved
  2026-06-13) — the cockpit Lab is now a real-data strategy-checking tool: run
  a strategy on the on-disk Binance data → it PERSISTS to a git-ignored
  `lab-runs/` cache (full-fidelity companion equity CSV; anchor-safe — outside
  the `verify_anchors.sh` glob) → the curve repaints from disk → Compare diffs
  KPIs. Headline: the `lab_run_engine::h3` test flipped skip → PASS (21,601
  equity points round-trip, in-memory == cached-disk). 712 tests, anchors
  119/119.
- **lab-compare-equity-overlay SHIPPED** (`53d5112`; operator-approved
  2026-06-13) — completes the "compare = KPIs + equity overlay" ask: select two
  persisted Lab runs (a `+` chip on the Compare matrix) and overlay their
  equity curves on one chart (ACCENT + ACCENT_2). The visual-regression gate
  correctly caught + rebased 12 stale Compare baselines. 626 tests, H3 intact.
- **Test health: still ZERO known red/flaky tests on `main`**; anchors held at
  119/119 across every ship; spec-lint steady at the 70 baseline.

### 2026-06-09 → 2026-06-12 cohort — cockpit Live completion + repo cleanup

- **cockpit-live-dashboard-wiring v0.1.2 follow-ups CLOSED** — I1 data-date
  x-axis via approach A (`PnlSnapshot.bar_ts` separate from the `as_of`
  delivery key; the rasterizing render harness `live_equity_render.rs` was
  born here and caught a flat-curve NaN panic; `10d1709`), I3 live Trades
  session counter + I4 `Screen::Live` boot (`4996fdb`).
- **live-equity-history-durable SHIPPED** (ADR-0052; `9eef752`; T6b purge
  scheduling `2ec06c6`; operator-approved `737d7bf`) — paper/live per-bar
  equity persists to the audit ledger (`equity_snapshots`, additive 013,
  `LiveEquityStore` trait); `cockpit_live` hydrates the curve on boot with
  the "Since inception" caption; research replay persists nothing (A2).
- **paper-mode-equity-wiring SHIPPED** (ADR-0053; `24c6213`;
  operator-approved `737d7bf`) — `spawn_trading_loop` unified for
  research+paper; paper mode runs the configured `SmaCrossover` against the
  live Binance feed via the deterministic `PaperEngine`, so the equity
  curve MOVES and persists non-constant. First feature since the v3 noop
  precedent where the baseline-equity-divergence gate APPLIES — satisfied
  at data (`paper_loop_produces_moving_equity`) + render
  (`y_variation_gate_moving_passes_flat_fails`, Y-variation bbox) layers.
  Research mode byte-identical (passes `equity_store: None`).
- **Repo cleanup Phase 1+2 EXECUTED** (`1405042`) — 367 files / −51.8k
  lines; tester reports, presentations, dev-notes, design prototypes into
  `spec/archive/`; `git gc` 61→41 MB; `cargo clean` 26 GiB. Phase-3
  ratified 2026-06-12 (`737d7bf`): research-era Rust KEPT (backs the 119
  anchors); history rewrite OFF the table.
- **Test health: ZERO known red/flaky tests on `main`** — `lab_run_engine::h3`
  fixed via the documented Phase-B contract (`2c4a59f`); the "montecarlo
  determinism flakes" root-caused as a `set_current_dir` CWD race × a dead
  config-load in `run_path` (isolation bug, NOT nondeterminism — anchors
  were never at risk; `7390fcb`, 11 consecutive green suites).

## Conventions

- One-line description; deeper context lives in the eventual
  `spec/<slug>/feature.md` brief.
- The orchestrator owns this file; agents may suggest additions but
  the operator approves promotions.
- Items can stay here indefinitely. Stale items get a `_decayed_` tag
  rather than silent deletion so the orchestrator can revisit.

## Changelog

- 2026-05-18 (analyst, backtest-real-binance-data): full analyst pass
  on the just-promoted feature.
  [`feature.md`](backtest-real-binance-data/feature.md) ships R1-R10
  closing Q1-Q8;
  [`tasks.md`](backtest-real-binance-data/tasks.md) ships the M0 →
  M-FINAL skeleton with `T-D-N` decomposition deferred to architect
  (T-AR-2). Recommended direction across the eight design questions:
  ADD a parallel `-realdata` scenario family (Q1, **operator-decide**
  — strong analyst default; in-place would re-anchor 6 v1/v2.5
  anchors); lock the new anchors under version `v2.6.0-realdata` (Q2,
  architect-locks); hard-fail on > 0.5% missing bars across the
  scenario span (Q3, architect-locks); pin the universe to the 10
  USDT pairs currently on disk (Q4, **operator-decide** — soft
  default; matches v1 hard-coded universe by happy coincidence); use
  the full year × 10-symbol bar counts (87 600 / 87 840) for the new
  scenarios while the 4 existing TCN synthetic anchors keep their
  2 208 / 6 600 counts (Q5, architect-locks); read 1h parquet bars
  directly with no aggregation (Q6, architect-locks); pin the data
  revision SHA via a new `data/binance/REVISION.toml` manifest with
  per-file SHAs, recorded once at fetch and verified on every read
  (Q7, architect-locks); scope wire-only — defer the Sharpe-table
  alpha verdict to a follow-on v25-tcn-overlay tester re-spawn (Q8,
  **operator-decide** — strong analyst default; splits plumbing-
  correctness review from signal-quality review). Non-regression
  contract: the 15 existing anchors stay byte-identical (9 strategy
  synthetic + 2 v2.5 passthrough TCN + 2 v2.5 real-weights TCN + 2
  operator-success); 4 new `-realdata` anchors lock at M-FINAL; ship
  count 19. Risk register K1-K10 surfaced (parquet schema drift, data
  gaps, time-alignment ambiguity, REVISION-manifest drift, anchor
  blast-radius on the synthetic paths, etc.) — each carries a named
  mitigation. Trace row `REQ-BACKTEST-REALDATA-001` opened in
  proposed state; `crates` / `tests` / `anchors` fields stubbed for
  the appropriate later owners. Stub entry under
  ## Queue ## Strategy and the placeholder Active row both updated
  to reflect the locked recommendations. HANDOFF → operator-decide
  (Q1 + Q4 + Q8) → architect.
- 2026-05-16 (analyst, Wave 2a spec-hygiene — bookkeeping flips
  per [`spec/dev-notes/feature-triage-2026-05-16.md`](dev-notes/feature-triage-2026-05-16.md)):
  five frontmatter / supersession updates landed, zero code touched,
  zero anchors moved, zero `trace.toml` rows added.
  - **v0-paper-sma** → `shipped` (was `in-progress`; row A1 —
    tasks 0/35, smoke checklist + 2 backtest reports + locked
    anchor on disk; bookkeeping flip).
  - **v05-composed-strategies** → `shipped` (was `in-progress`;
    row A2 — drift reconcile: `tasks.md` already carried
    `status: shipped` from 2026-04-20; `feature.md` now matches).
  - **v1-cross-sectional-momentum** → `shipped` (was
    `in-progress`; row A3 — `T_FINAL_A_v1` / `T_FINAL_B_v1` both
    ticked since 2026-04-30; the single open box **T612** —
    multi-symbol live `BinanceFeed` — stays `[ ]` under v1.5
    lineage with carrier
    [`v1-5b-multi-venue`](v1-5b-multi-venue/feature.md); note
    added to the v1 `tasks.md ## Notes`).
  - **ui-gallery-bin** → `shipped` with body header marking
    **v0.1-partial, terminal** + `version: 0.1.0 →
    0.1.0-partial-terminal` + new `successor:
    ui-gallery-table-cell` frontmatter field (row A4 — V5+
    blocked by `tiny-skia` `Build quad rectangle` panic in
    `widget::table::Table` bisected to `GALLERY_CELLS[7]`). The
    39 open task boxes are marked `[deferred to
    ui-gallery-table-cell]` at the top of
    [`ui-gallery-bin/tasks.md`](ui-gallery-bin/tasks.md) and
    preserved verbatim for trace. **Precedent question** raised
    in the feature.md Changelog: the `spec-update` status enum
    has no `shipped-partial`; this edit took the conservative
    `status: shipped` + version-modifier route and proposed
    formal enum promotion at the next skill revision.
  - **ui-gallery-table-cell** → new draft (successor brief
    opened at
    [`spec/ui-gallery-table-cell/feature.md`](ui-gallery-table-cell/feature.md)
    + [`tasks.md`](ui-gallery-table-cell/tasks.md)). Owns R1
    (restore V5+ render path: full
    `GALLERY_CELLS` set renders without panic). V5+ tasks
    re-keyed `T1..T9` + `T-FINAL-*`; M0 architect block
    re-spawned for Q-FIX-STRATEGY (special-case wrapper /
    swap render / upstream fix). HANDOFF → architect.
- 2026-05-15 (operator, iced-014 lock + re-sequence): locked all 5
  Q-* migration questions from
  [`iced-014-feature-analysis-2026-05-15.md`](dev-notes/archive/2026-Q2/iced-014-feature-analysis-2026-05-15.md)
  at analyst defaults (Q-014-PIN, Q-COMET-EVAL, Q-TESTER-FEATURE,
  Q-PANEL-UPSTREAM, Q-D3-RELITIGATE). Backlog impact: (a) RESCOPED
  `ui-session-journal` → `ui-session-journal-iced-tester` (~4d →
  ~1d) since iced 0.14 ships `iced_tester` + `.ice` format
  natively; (b) CHEAPENED `ui-test-harness-ci` 5d → 4d via the
  shipped `iced_test::emulator::Emulator` + embedded Fira Sans;
  (c) PROMOTED new candidate `ui-iced-table-panic-upstream`
  (0.5d) to file the strategies-Table tiny-skia panic upstream;
  (d) ADDED `ui-comet-eval` as deferred-no-trigger candidate
  (comet requires iced 0.15-dev, not compatible with our pinned
  `=0.14.0`). All other queued items unchanged.
- 2026-05-15 (analyst, ui-testability deep-dive): authored
  [`spec/dev-notes/ui-testability-deep-dive-2026-05-15.md`](dev-notes/archive/2026-Q2/ui-testability-deep-dive-2026-05-15.md)
  — a research dev-note critiquing the four-week plan in
  [`ui-testing-direction-2026-05-12.md`](dev-notes/archive/2026-Q2/ui-testing-direction-2026-05-12.md),
  surfacing 3 structural blind spots (canvas-state ownership
  via H2 caveat, pixels-only oracle, no reachability coverage)
  + 8 schedulable proposals across a re-shaped Layer 0..7
  pyramid. Promoted 8 new candidate features into ## Queue ##
  Process / tooling above (ui-contrast-asserter,
  ui-update-proptest, ui-gallery-bin, ui-a11y-shadow,
  ui-vlm-judge, ui-inspect-mcp, ui-session-journal,
  ui-mutants-pass, plus a tester visual-fail HTML artifact
  agent-contract update). Each item points to a specific §
  anchor in the dev-note for the operator's "schedule which"
  pick. Existing
  [`ui-test-harness-ci`](#queue) candidate annotated with a
  +1-day cross-platform falsifier to revisit operator decision
  D3 (macOS-only CI) per dev-note §5.3 + §2.6. No code
  changes; pure spec output. Six open operator questions
  surfaced in dev-note §6 (Q-VLM, Q-ACCESSKIT, Q-MCP,
  Q-GALLERY-SCOPE, Q-D3-REVISIT, Q-MUTANTS-CADENCE) — each
  has a documented default so the operator may sequence
  features without a separate Q&A round.
- 2026-05-12 (analyst, second pass): promoted
  `ui-test-harness-bootstrap` v0.1 from Queue ## Process / tooling
  → Active. First feature under the new
  [`AGENT.md ## Capability boundaries`](../AGENT.md#capability-boundaries)
  regime. Scope locked to **week-1 only** of the
  [dev-note §6 4-week plan](dev-notes/archive/2026-Q2/ui-testing-direction-2026-05-12.md#6-phased-adoption--4-week-plan)
  per operator decisions D1–D5; weeks 2 / 3 / 4 are separate
  candidate features queued under ## Queue ## Process / tooling for
  later analyst spawn. The week-1 grid-sweep test at 3360×1890
  closes chart-canvas-overhaul V15 (D4) without a manual
  screencapture. Brief at
  [`ui-test-harness-bootstrap/feature.md`](ui-test-harness-bootstrap/feature.md);
  task stubs at
  [`ui-test-harness-bootstrap/tasks.md`](ui-test-harness-bootstrap/tasks.md).
  HANDOFF → architect (Design section + task body fleshing). Q1
  (cockpit factory), Q2 (iced_test snapshot accessor), Q3-Q7
  architect-decide; Q8 (image-compare yes/no), Q9 (fixture
  parity), Q10 (baseline naming) operator-input.
- 2026-05-12 (architect, chart-canvas-overhaul M7): queued
  **v1.11 — Chart x-axis local time (`chart-x-axis-local-time`)**
  under UI / cockpit Queue as a `candidate`. Deferred from v1.10.0
  by operator-locked Q-revised-1 = path (b) (defer Q4 to v1.11
  follow-up). One-line scope: workspace `time` `local-offset`
  feature flag flip + `local_offset_or_utc()` body wire-up; full
  details for the v1.11 analyst.
- 2026-05-08 (orchestrator, post-Phase-5 cleanup): drained the
  Active section. The 5 prior Active entries (`live-cockpit-unified`,
  `real-mtm-unrealized-pnl`, `per-symbol-position-accounts`,
  `tape-row-audit-modal`, `journal-transactions-metadata`) all
  shipped in v1+ → operator-success-reports cycle and are referenced
  as shipped invariants in the Lumen master roadmap's cross-feature
  invariants table; never moved out of Active. Plus the
  `lumen-design-adoption` master-roadmap row dated 2026-05-03 still
  referenced the obsolete 4-phase plan. All six Active entries
  removed (organizational hygiene; their feature briefs remain at
  `spec/<slug>/feature.md`). UI / cockpit Queue subsection
  collapsed to just Phase 6 (the only remaining initiative work).
  Initiative status: 5-of-6 phases shipped; reaches a natural pause
  point absent v2 LLM. No new feature promoted.
- 2026-05-08 (presenter, Phase 5 sprint review APPROVED): operator
  signed the Phase 5 sprint review deck at
  [`lumen-design-adoption/presentations/lumen-design-adoption-2026-05-04-to-05-08.md`](archive/presentations-2026-Q2.tar.gz)
  (`[x] Approved — ship`). Phase 5 closed cleanly: tester second-
  pass PASS (8/8 gates after a one-line `cargo fmt --all` fixup
  between passes; first-pass FAIL preserved on disk for audit), 896
  tests across 110 binaries, 11/11 anchors byte-identical, 86
  baselines attested clean, **TD-1 four-phase deferral CLOSED** via
  Path (b) custom-widget escape hatch, new TD-2 row added for the
  deferred risk-engine veto-emit upstream wiring. **Phase 5 is the
  last shippable phase of the lumen-design-adoption initiative
  absent v2 LLM** — Phase 6 (Assistant slot) is reserved.
  Initiative status: 5-of-6 phases shipped; Phase 6 unlocks when
  v2 LLM is approved. No new active feature promoted from this
  approval; next-step decision is operator's (promote v2 LLM,
  promote a different Active backlog item, or pause the cockpit-
  side initiative as 5-phase-complete).
- 2026-05-06 (presenter, Phase 4 sprint review APPROVED): operator
  signed the Phase 4 sprint review deck at
  [`lumen-design-adoption/presentations/lumen-design-adoption-2026-05-04-to-05-08.md`](archive/presentations-2026-Q2.tar.gz)
  (`[x] Approved — ship`). Phase 4 closed cleanly: tester second-
  pass PASS (8/8 gates after a one-line `match_same_arms` fix
  between passes; first-pass FAIL preserved on disk for audit), 850
  tests across 108 binaries, 11/11 anchors byte-identical, 72
  baselines attested clean, three-bin workspace (`viewer` greenfield
  + `cockpit` + `cockpit_live`). Promoted `lumen-phase-5-humancontrol-agentfeed`
  from Queue → Active per the master-roadmap sequencing constraint
  (Constraint 3). Phase 5 brief stub frontmatter bumped from
  `queued` → `active`; the analyst's next pass expands the stub
  into a full brief. **Phase 5 is the first phase to introduce
  net-new operator-write paths** (pause-strategy, override-risk-
  veto, execution-mode toggle), making it the load-bearing decision
  point for the TD-1 focus-ring deferral. Mechanical pre-tick gate
  at sprint-review time PASSED before approval. HANDOFF → analyst
  (Phase 5 brief expansion).
- 2026-05-06 (presenter, Phase 3 sprint review APPROVED): operator
  signed the Phase 3 sprint review deck at
  [`lumen-design-adoption/presentations/lumen-design-adoption-2026-05-04-to-05-08.md`](archive/presentations-2026-Q2.tar.gz)
  (`[x] Approved — ship`). Phase 3 closed cleanly: tester first-pass
  PASS (8/8 gates), 810 tests across 104 binaries, 11/11 anchors
  byte-identical post-migration, 65 baselines attested clean, two
  developer passes (pass 1 cut at clean tick boundary; pass 2
  finished T1702 migration + remaining 14 tasks). Promoted
  `lumen-phase-4-backtest-panel` from Queue → Active per the
  master-roadmap sequencing constraint (Constraint 3). Phase 4
  brief stub frontmatter bumped from `queued` → `active`; the
  analyst's next pass expands the stub into a full brief with
  R-items / V-items / Q-items + task list contract. **Phase 4 will
  absorb the deferred Q6 sparkline from Phase 3** — both surfaces
  need the same equity-history primitive. Mechanical pre-tick gate
  at sprint-review time PASSED before approval (`PRESENTATION
  CHECK PASS`). HANDOFF → analyst (Phase 4 brief expansion).
- 2026-05-05 (presenter, Phase 2 sprint review APPROVED): operator
  signed the Phase 2 sprint review deck at
  [`lumen-design-adoption/presentations/lumen-design-adoption-2026-05-04-to-05-08.md`](archive/presentations-2026-Q2.tar.gz)
  (`[x] Approved — ship`). Phase 2 closed cleanly: tester first-pass
  PASS (8/8 gates), 781 tests across 98 binaries, 11/11 anchors,
  53 baselines attested clean. Promoted `lumen-phase-3-detail-screens`
  from Queue → Active per the master-roadmap sequencing constraint
  (Constraint 3). Phase 3 brief stub frontmatter bumped from `queued`
  → `active`; the analyst's next pass expands the stub into a full
  brief with R-items / V-items / Q-items + task list contract.
  Mechanical pre-tick gate at sprint-review time PASSED before
  approval (`PRESENTATION CHECK PASS`). HANDOFF → analyst (Phase 3
  brief expansion).
- 2026-05-04 (presenter, Phase 1 sprint review APPROVED): operator
  signed the Phase 1 sprint review deck at
  [`lumen-design-adoption/presentations/lumen-design-adoption-2026-05-04-to-05-08.md`](archive/presentations-2026-Q2.tar.gz)
  (`[x] Approved — ship`). Phase 1 closed cleanly. Promoted
  `lumen-phase-2-shell-ia-charts` from Queue → Active per the
  master-roadmap sequencing constraint (Constraint 3). Phase 2
  brief stub frontmatter bumped from `queued` → `active`; the
  analyst's next pass expands the stub into a full brief.
  Mechanical pre-tick gate at sprint-review time PASSED before
  approval (`PRESENTATION CHECK PASS`). HANDOFF → analyst (Phase 2
  brief expansion).
- 2026-05-04 (analyst, post-Phase-1-ship roadmap revision):
  `lumen-phase-1-foundation` shipped → moved Active → Recent.
  Lumen master roadmap revised from 4 to 6 phases at operator
  request (session of 2026-05-04, after Phase 1 third-pass tester
  PASS). Two new phases inserted ahead of the original phase plan:
  Phase 2 Shell IA + Charts (sidebar nav + Home / Debug / Charts
  screens + price chart with buy/sell markers + read-only audit
  query extension), Phase 3 Detail screens (Strategies / Risk /
  Audit sidebar entries over existing backend data). Original
  Phase 2 (Backtest panel) renumbered to Phase 4; original Phase 3
  (HumanControl + AgentFeed) renumbered to Phase 5; original Phase
  4 (Assistant slot) renumbered to Phase 6 (still reserved for v2
  LLM). Five new feature-brief stubs spawned. Operator decisions
  captured as Q11–Q14 in the master open-questions section
  (sidebar fixed-width; chart in both bins; extend `audit::query`
  for marker filtering; keep Phase 2 / 3 split). Anchor risk per
  phase table extended to 6 rows; cross-feature invariants table
  extended to 6 phase columns. UI / cockpit Queue subsection
  rewritten for the 6-phase plan. HANDOFF → architect at Phase 2
  promote (when the operator signs Phase 1 presentation).
- 2026-05-01 (orchestrator): initial draft. Captures the 5 followups
  surfaced at `operator-success-reports` ship; promotes
  live-cockpit-unified to Active.
- 2026-05-01 (analyst): live-cockpit-unified Active line updated to
  reference the just-written
  [`features/live-cockpit-unified.md`](live-cockpit-unified/feature.md)
  brief.
- 2026-05-02 (analyst): promoted "Real mark-to-market unrealized P&L"
  from Queue → Active. Brief at
  [`features/real-mtm-unrealized-pnl.md`](real-mtm-unrealized-pnl/feature.md);
  HANDOFF → architect.
- 2026-05-02 (analyst): promoted "R10 follow-up:
  per-symbol-position-accounts" from the implicit Queue (deferral
  note in `real-mtm-unrealized-pnl.md` Design § Q3 / R10 verdict) →
  Active. Brief at
  [`features/per-symbol-position-accounts.md`](per-symbol-position-accounts/feature.md);
  HANDOFF → architect.
- 2026-05-03 (orchestrator): new UI / cockpit subsection. Added
  `tape-row-audit-modal` per operator decision on UI principles Q4
  (2026-05-03). Promotes when operator picks it up; analyst → architect
  → developer pipeline standard.
- 2026-05-03 (analyst): promoted `tape-row-audit-modal` from Queue
  (UI / cockpit) → Active. Brief at
  [`features/tape-row-audit-modal.md`](tape-row-audit-modal/feature.md).
  First feature to land against
  [`ui-design-principles.md`](ui-design-principles.md) (the "Show
  the why" cockpit click-through-to-audit path begins here). 15
  R-items, 11 V-items, 9 open questions for the architect. Anchor
  risk: zero (pure UI + new additive audit reader). HANDOFF →
  architect. The now-empty `### UI / cockpit` Queue subsection has
  been removed; future UI additions will recreate it.
- 2026-05-03 (analyst): added `journal-transactions-metadata` to
  Active. Brief at
  [`features/journal-transactions-metadata.md`](journal-transactions-metadata/feature.md).
  Closes the T1206 deviation note in the just-shipped
  [`features/tape-row-audit-modal.md`](tape-row-audit-modal/feature.md)
  (Implementation § Async dispatch, lines 625–635) — live cockpit's
  modal header is empty because T1202's reader stays narrow
  (entries-only). New sibling reader + `core::JournalTransactionMetadata`
  struct + cockpit_live `Task::perform` chain. 7 R-items, 5 V-items,
  6 open questions for the architect. Anchor risk: zero (additive
  read-only). HANDOFF → architect.
- 2026-05-03 (analyst): promoted `v1.5b multi-venue + 1s aggregated
  trades` from Queue (Strategy) → Active. Brief at
  [`features/v1-5b-multi-venue.md`](v1-5b-multi-venue/feature.md).
  **Largest queued backend feature.** Coinbase + Kraken adapters,
  USDC pair mirror set (10 symbols), T612 multi-symbol live
  `BinanceFeed` (the v1 closeout deferral lands here), 1s
  aggregated trades, `Venue` enum on `Tick` / `Bar`, per-venue
  feed-reconnect provenance (T805 extension). Plumbing-only —
  expands the data side, not the execution side. 15 R-items,
  12 V-items, 12 open questions for the architect. Anchor risk:
  zero by construction (architect-confirmed grep of
  `spec/reports/**/*.md` for `venue`/`coinbase`/`kraken`).
  Cost risk: zero — all three venues have free public
  market-data WS APIs. Failover risk: medium — N independent
  failure modes; per-venue tokio tasks the recommended
  isolation strategy. Closes v1.5a Q5 (USDC pairs blocker) and
  v1 closeout T612. HANDOFF → architect.
- 2026-05-03 (analyst): `v1.5b multi-venue + 1s aggregated
  trades` shipped (verdict PASS, presenter approved). Moved
  Active → Recent. Promoted the
  [`lumen-design-adoption`](lumen-design-adoption/feature.md)
  master roadmap + the
  [`lumen-phase-1-foundation`](lumen-design-adoption/phase-1-foundation/feature.md)
  first-phase brief to Active. Master roadmap covers the 4-phase
  Lumen design-system adoption: Phase 1 Foundation (Active —
  tokens + tiers + status bar + principles supersede), Phase 2
  Viewer Backtest (Queue — KPI strip + equity curve + drawdown
  band on the offline review surface), Phase 3 HumanControl +
  AgentFeed rename (Queue — richer override controls + tape →
  AgentFeed module rename), Phase 4 Assistant slot (Reserved —
  depends on v2 LLM strategy ship). Operator-locked constraints
  inherited at every phase: NO brand adoption (no "Lumen" name,
  no eye/lens logo), NO voice rules rewrite (`ui::strings`
  unchanged), sequential phasing (one approval per phase), dark
  as default. Phase 1 brief carries 17 R-items, 9 V-items, 9
  open questions for the architect. Anchor risk per phase:
  zero (Phase 1 / 2 / 3 are UI features, no backtest path
  touched); Phase 4 out of scope. The 11 / 11 backtest body-SHA-256
  anchor regression goal stays byte-identical across the entire
  initiative. Cross-feature invariants documented for the 7 prior
  shipped UI-touching features (operator-success-reports,
  live-cockpit-unified, real-mtm, per-symbol, tape-modal,
  journal-tx-metadata, v1.5b). HANDOFF → architect (Phase 1
  first; master roadmap for orientation).

- 2026-05-12 (operator): v1.10.0 SHIPPED — `chart-canvas-overhaul`.
  Recent entry added above. The architect-misdiagnosis
  retrospective produced
  [`AGENT.md ## Capability boundaries`](../AGENT.md#capability-boundaries-orchestrator-vs-sub-agent)
  amendment +
  [`spec/dev-notes/archive/2026-Q2/ui-testing-direction-2026-05-12.md`](dev-notes/archive/2026-Q2/ui-testing-direction-2026-05-12.md)
  strategy document + queued `ui-test-harness-bootstrap` v0.1
  feature now in Active per operator decisions D1-D5. Anchors
  PASS 11/11 (verbatim line in deck `## Live demo` block).
  Approval evidence:
  [`chart-canvas-overhaul/presentations/chart-canvas-overhaul-2026-05-12.md ## Approval`](archive/presentations-2026-Q2.tar.gz).

- 2026-05-12 (operator): v0.1 SHIPPED — `ui-test-harness-bootstrap`.
  First feature under the new
  [`AGENT.md ## Capability boundaries`](../AGENT.md#capability-boundaries-orchestrator-vs-sub-agent)
  regime AND first run of the test-runner / evaluator split.
  Empirical proof the new workflow holds: 0 capability-boundary
  violations across 6 agent roles; 2 architect/developer-side
  API corrections surfaced cleanly without rework loops; evaluator
  PASS in fresh read-only context with file:line cites; anchors
  PASS 11/11. V8 (render-half of chart-canvas-overhaul V15)
  PASS-with-H2-caveat — queued as
  `ui-test-harness-canvas-state-seeding` candidate. Approval
  evidence:
  [`ui-test-harness-bootstrap/presentations/ui-test-harness-bootstrap-2026-05-12.md ## Approval`](archive/presentations-2026-Q2.tar.gz).

- 2026-05-13 (operator): v2.0.0 SHIPPED — `v2-llm-strategy`.
  Foundation-only per Q1=A. 6 dev passes (`d0bcad2` →
  `faaaec1`) + tester (`8a41b47`). 44/45 dev tasks ticked +
  T_FINAL [x]. 11/11 anchors PASS (2 report-sample-*
  re-locked by tester to v2.0.0 SHAs; 9 strategy anchors
  byte-identical). 1203 workspace tests passing. D1/D2/D3
  operator-locked decisions honored. 3 deferred items
  consolidated into `v2-llm-strategy-v21-followups`
  candidate. Unblocks Kronos v2.5 + Lumen Phase 6 +
  reflection-memory follow-ups. Approval evidence:
  [`v2-llm-strategy/presentations/v2-llm-strategy-2026-05-13.md ## Approval`](archive/presentations-2026-Q2.tar.gz).

- 2026-05-13 (operator): v0.1.0 SHIPPED — `iced-native-widgets`
  (Brief A of iced ecosystem evaluation). 4-lane parallel dev
  fan-out (positions / strategies Tables + kpi_strip Grid +
  journal_modal Float) across commits `3077425` → `970e857` →
  `9e5bd65` → `9027a0d` → tester `1431409`. Second invocation
  of the test-runner / evaluator split; evaluator default-FAIL
  contract held; 20/20 V-items PASS; 11/11 anchors byte-identical;
  bootstrap V8 visual baselines preserved (3 PNG SHAs unchanged);
  1203+ workspace tests passing. New shared
  `crates/ui/src/theme/iced_widget_catalogs.rs` Catalog adapter
  scaffold for Brief B (iced_aw cherry-pick: date_picker +
  spinner + badge). 4 honest architectural divergences from
  architect's brief flagged + corrected inline. Approval
  evidence:
  [`iced-native-widgets/presentations/iced-native-widgets-2026-05-13.md ## Approval`](archive/presentations-2026-Q2.tar.gz).
