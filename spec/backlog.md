---
slug: backlog
status: living
owner: orchestrator
updated: 2026-05-20
---
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
     `spec/dev-notes/ui-testing-direction-2026-05-12.md ## Section 9`
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


<!-- updated 2026-05-21 (analyst, v25-tcn-recalibrate) — analyst pass
     landed for the cheap-first follow-on to v25-tcn-alpha-investigation
     v0.1.0. Brief at `spec/v25-tcn-recalibrate/feature.md` carries
     R1-R8, H1-H3, K1-K5, Q1-Q5 with analyst-recommended defaults.
     Promoted Queue → Active above v25-tcn-alpha-investigation per
     the predecessor's presenter-deck ranked recommendation
     (`v25-tcn-recalibrate` first, then `v25-tcn-horizon-bump-or-retire`
     only if F4 survives recalibration). Trace row
     `REQ-V25-TCN-RECALIBRATE-001` opened `draft`. Diagnostic finding:
     training-time σ_train accumulator at `train_tcn.rs:606,676-678,733-741`
     never resets between epochs → final scalar = std of training
     trajectory, not of converged-model predictions. -->

- **v2.5 alpha-verdict investigation (`v25-tcn-alpha-investigation`).**
  _draft (analyst-recommended scope: MINIMAL; awaiting operator
  scope-decision on Q1)_ — promoted Queue/Strategy → Active 2026-05-18
  by analyst. Read-only, forensic investigation into the persistent
  `dampened=0` finding from
  [`backtest-real-binance-data v0.1.0`](backtest-real-binance-data/feature.md)
  (commit `df73780`, four `-realdata` anchors locked under
  `v2.6.0-realdata`, all reporting `dampened=0` on real Binance
  hourly OHLCV — same as M3 reported on synthetic data, but now on
  the training distribution itself which falsifies the M3 hypothesis).
  Predecessor: `backtest-real-binance-data v0.1.0`. Parent (stays
  `in-progress`): `v25-tcn-overlay v2.5.0`.
  Brief at [`feature.md`](v25-tcn-alpha-investigation/feature.md)
  carries R1-R6 + a four-case failure-mode taxonomy F1-F4 (R4) that
  routes the verdict to a named follow-on feature. ONE
  operator-decide Q: scope —
  **minimal** (a + d: histogram + Sharpe-table, no re-training,
  analyst-recommended); **diagnostic** (a + c + d: adds
  checkpoint-internal inspection); or **full**
  (a + b + c + d: adds horizon-bumped re-train, ~2-3 weeks).
  Default if no answer: minimal. R6 non-regression contract: 19
  originals byte-identical; ≤3 new anchors at ship under version
  `v2.6.0-alpha-investigation`. Trace row
  `REQ-V25-TCN-ALPHA-001` opened proposed.
  HANDOFF → operator-decide (1 Q) → architect.

- **v2.5 — TCN forecast overlay (`v25-tcn-overlay`).** _in-progress
  (CI-baseline + M3 real-weights gates approved 2026-05-18; real-data
  wired but TCN dampened=0 on real OHLCV — alpha-verdict investigation
  promoted to Active above)_ — phase 1 of the
  [4-phase DL roadmap](v25-dl-forecast-overlay/feature.md)
  (operator-locked 2026-05-17 after reading the
  [v25-dl-reading-list](dev-notes/v25-dl-reading-list-2026-05-16.md)
  and deciding to build all three model families for empirical bake-off).
  Model family: Temporal Convolutional Network (Bai, Kolter, Koltun
  2018). Selected first because (a) simplest architecture, fastest to
  a working baseline; (b) establishes the training loop + audit +
  replay infrastructure that v2.5a (PatchTST) and v2.5b (Transformer)
  reuse; (c) deterministic inference (no autoregressive sampling) —
  easier to anchor and audit. Data prerequisite: 10 USDT pairs hourly
  2023+2024 bootstrapped via `cargo run -p data --bin fetch_binance_klines`
  (~72s wallclock, ~15-20 MB). **Analyst pass landed 2026-05-17**
  ([`feature.md`](v25-tcn-overlay/feature.md) §Requirements R1–R12)
  closing Q1-Q8 with defaults (TCN 8-block dilation `[1..128]`, k=3,
  H=96 → ~4.4M params; 256-bar context; 5 features; continuous
  log-return regression with Huber δ=0.001; OneCycle AdamW;
  two-checkpoint walk-forward; SHA-256 provenance over arch+data+seed+weights).
  Two operator-decide questions surface: anchor checkpoint storage
  (LFS vs regen-from-seed) and two-checkpoint backtest split.
  Carry-forward backtest scenarios: BS-1 (2023 full-year top-10 USDT),
  BS-2 (2024 full-year top-10 USDT). HANDOFF → operator-decide → architect.

## Queue

### Strategy

- **v2.5 alpha-verdict investigation (`v25-tcn-alpha-investigation`).**
  _moved Queue → Active 2026-05-18 (analyst pass)_ — see
  [Active section](#active) for the live tracking row and
  [`feature.md`](v25-tcn-alpha-investigation/feature.md) for the full
  brief. The original 4-bucket framing is preserved here as a pivot
  reference in case the analyst-recommended **minimal** scope (buckets
  a + d) needs to be widened post-verdict:
  (a) **ε / confidence-threshold tuning** — are the deadband (0.0005)
      and gating-confidence (0.6) too tight? Histograms of `r_hat`
      and `|r_hat|/sigma_train` across 87 590 BS-1 + 87 840 BS-2 bars
      answer this — covered by **R1 / R3** of the brief (MINIMAL scope).
  (b) **horizon mismatch** — TCN trained at next-1h log-return; v1
      momentum operates on 20-bar lookback. Multi-step / multi-horizon
      heads may be needed — covered only under **FULL** scope (M-HORIZON
      milestone); follow-on feature `v25-tcn-horizon-bump` if R4 returns
      verdict F4 under minimal scope.
  (c) **training pathology** — final val Huber = 1.5e-5 is suspiciously
      tiny on real OHLCV; could be "predict ≈zero" collapse. Held-out
      checkpoint inspection — covered only under **DIAGNOSTIC** scope
      (M-DIAG milestone); follow-on feature `v25-tcn-retrain` if R4
      returns verdict F1 under minimal scope.
  (d) **Sharpe / drawdown table** — TCN vs v1-baseline on the four
      `v2.6.0-realdata` anchors — covered by **R5 / M-SHARPE**
      milestone (MINIMAL scope).
  Predecessor: [`backtest-real-binance-data`](backtest-real-binance-data/feature.md)
  v0.1.0. Blocks: v2.6 forecast bake-off (need a verdict on whether
  TCN's `dampened=0` reflects an envelope-tuning issue, a training
  pathology, or genuine no-signal; bake-off can't compare three
  model families on data where any may report dampened=0).

- **v2.5a — PatchTST / iTransformer forecast overlay
  (`v25a-patchtst-overlay`).** _roadmap_ — phase 2 of the
  [4-phase DL roadmap](v25-dl-forecast-overlay/feature.md). Activates
  after phase 1 (TCN) ships. Patch-based Transformer paradigm; reuses
  training infrastructure from phase 1. Stub at
  [`spec/v25a-patchtst-overlay/feature.md`](v25a-patchtst-overlay/feature.md).
- **v2.5b — Vanilla decoder-only Transformer overlay
  (`v25b-transformer-overlay`).** _roadmap_ — phase 3 of the
  [4-phase DL roadmap](v25-dl-forecast-overlay/feature.md). Activates
  after phases 1+2 ship. Autoregressive Transformer over discretised
  OHLCV tokens (the operator's hand-built Kronos-shape successor —
  full provenance, no pre-trained weights). Stub at
  [`spec/v25b-transformer-overlay/feature.md`](v25b-transformer-overlay/feature.md).
- **v2.6 — Forecast bake-off + retirement (`v26-forecast-bakeoff`).**
  _roadmap_ — phase 4 of the
  [4-phase DL roadmap](v25-dl-forecast-overlay/feature.md). After all
  three phase-1/2/3 forecasters ship, run a head-to-head on BS-1+BS-2
  with identical criteria. Pick the canonical v2.5 overlay; mark the
  other two as research-mode only. Stub at
  [`spec/v26-forecast-bakeoff/feature.md`](v26-forecast-bakeoff/feature.md).
- **Pre-pivot breadcrumb:** the dropped Kronos approach is preserved at
  [`spec/dev-notes/kronos-evaluation-2026-05-10.md`](dev-notes/kronos-evaluation-2026-05-10.md)
  [SUPERSEDED] as a what-not-to-do reference.

### UI / cockpit (Lumen design-system adoption — Phase 6 reserved)

- **Lumen Phase 6 — Assistant slot.** _reserved_ — depends on the
  v2 LLM strategy queued item above. Right-rail collapsible panel
  for the v2 LLM assistant per
  [`spec/design/project/ui_kits/desktop/Assistant.jsx`](design/project/ui_kits/desktop/Assistant.jsx).
  Phase 2 reserved the right-rail column-track in the shell grid
  at `Length::Fixed(0.0)`; the actual Phase 6 brief lands when v2
  LLM is approved. Until then, no analyst spawn. Stub at
  [`features/lumen-phase-6-assistant-slot.md`](lumen-design-adoption/phase-6-assistant-slot/feature.md).
  _(Phases 1–5 of the lumen-design-adoption initiative are shipped
  and live in the Recent section; this Queue entry is the only
  remaining initiative work, gated on v2 LLM.)_

- **TBD — Cockpit Windows / Linux support (`cockpit-cross-platform`).**
  _candidate_ — surfaced 2026-05-12 by operator decision D3 in
  [`spec/dev-notes/ui-testing-direction-2026-05-12.md`](dev-notes/ui-testing-direction-2026-05-12.md#section-9).
  Today the cockpit is macOS-only (Retina assumptions, `screencapture` +
  TCC dependencies, `iced_tiny_skia` chosen partly for CPU determinism on
  Apple Silicon). Scope when promoted: validate `iced_tiny_skia` rendering
  parity on Linux X11/Wayland + Windows; replace `screencapture`-based
  test artifact capture with cross-platform `xcap` or equivalent;
  re-evaluate the `time` `local-offset` feature's Linux multi-threaded
  caveat once v1.11 lands. Analyst spawn deferred — operator triggers
  when external demand (paper-trading on Linux server, Windows operator
  hardware) appears.

### Process / tooling

- **`v2x-trading-state-bus`** (v2 LLM evolution) —
  _candidate, sourced from
  [`spec/dev-notes/external-code-patterns-2026-05-17.md`](dev-notes/external-code-patterns-2026-05-17.md)_.
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
  _candidate, sourced from
  [`spec/dev-notes/external-code-patterns-2026-05-17.md`](dev-notes/external-code-patterns-2026-05-17.md)_.
  After v2.5 / v2.5a / v2.5b ship as TCN / PatchTST / Vanilla
  Transformer, an LLM arbiter reads all three forecasters' outputs
  + the operator's strategy params and produces a tie-break decision
  with a reasoning trace that lands in the audit ledger. Adapts the
  [TradingAgents](https://github.com/TauricResearch/TradingAgents)
  bull/bear adversarial researcher pattern to DL-forecast arbitration.
  Plugs into [`spec/v26-forecast-bakeoff/feature.md`](v26-forecast-bakeoff/feature.md);
  not a separate feature — a v2.6 design refinement the analyst
  considers when the bake-off feature activates.

- **v2.1 — Cockpit LLM-budget tile + tracing-Layer redactor +
  pedantic clippy cleanup (`v2-llm-strategy-v21-followups`).**
  _candidate, surfaced 2026-05-13 by v2-llm-strategy v2.0.0 ship_ —
  three deferred items consolidated:
  (a) **T1938 cockpit "LLM budget" tile** — was deferred in pass 6
      because its dependency `audit::query::llm_spend_this_month`
      isn't implemented. v2.1 ships the audit query helper +
      the right-rail tile (three-color thresholds: green < 60%,
      amber 60-80%, red ≥ 80%; auto-degrade at 80% per Q6).
  (b) **T1915 tracing-Layer redactor half** — pure-fn `redact()`
      landed in pass 3; the `tracing_subscriber::Layer` field-
      visitor side needs `tracing_subscriber = "0.3"` (new dep).
      v2.1 wires the Layer so structured logging redacts
      `Bearer ...` / `sk-...` / `anthropic-...` patterns in
      fields without requiring callers to invoke `redact()`
      explicitly.
  (c) **T1910 pedantic clippy cleanup** — 2 `cast_possible_truncation`
      warnings on `crates/audit/src/query.rs:219, 221` from the
      `cache_hit_ratio_since` query. Non-blocking per v2.0.0
      brief §Critical constraints #2. v2.1 cleans these up via
      `Decimal::try_from` or explicit clamp.
  Analyst spawn when operator promotes; not urgent.

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
  (`ui-test-harness-viewport-matrix`).** _candidate, gated on
  `ui-test-harness-bootstrap` v0.1 ship_ — extends the v0.1 Charts-only
  three-viewport snapshot harness across ALL widget tests (panels,
  modals, status bar, agent feed, debug screen) per
  [dev-note §6 week 2](dev-notes/ui-testing-direction-2026-05-12.md#6-phased-adoption--4-week-plan).
  Analyst spawn when v0.1 ships and H1 (tiny-skia byte determinism)
  is unfalsified.

- **Week-3 follow-up — evaluator subagent + PreToolUse hooks
  (`ui-test-harness-evaluator`).** _candidate, gated on
  `ui-test-harness-bootstrap` v0.1 ship_ — splits the tester role
  into test-runner (writeable) + evaluator (read-only, fresh
  context, default-FAIL PreToolUse hook) per
  [dev-note §4.2](dev-notes/ui-testing-direction-2026-05-12.md#42-default-fail-evaluator-subagent)
  and
  [`AGENT.md ## Test-runner / evaluator split`](../AGENT.md#test-runner--evaluator-split).
  Wires the PreToolUse hooks for `screencapture`, `osascript`, and
  `./target/release/cockpit` denying sub-agents (allowing
  orchestrator). Analyst spawn after v0.1 ships.

- **Week-4 follow-up — GitHub Actions CI + presenter integration
  (`ui-test-harness-ci`).** _candidate, gated on
  `ui-test-harness-viewport-matrix` + `ui-test-harness-evaluator`
  ship_ — macOS runner workflow uploading baseline+actual+diff PNG
  triples on visual snapshot failures; presenter deck format gets a
  fixed "screenshot artifacts" section pointing at the CI artifact
  URL per [dev-note §6 week 4](dev-notes/ui-testing-direction-2026-05-12.md#6-phased-adoption--4-week-plan).
  **Per [`ui-testability-deep-dive-2026-05-15.md §5.3`](dev-notes/ui-testability-deep-dive-2026-05-15.md#53-keep--drop--replace-against-the-existing-weeks-2-4-plan)**
  the analyst recommends pairing this CI brief with the 1-day
  cross-platform falsifier (item O) to retire or confirm operator
  decision D3 (macOS-only CI). **CHEAPENED 2026-05-15:** down to
  ~4 dev-days (from 5) per
  [`iced-014-feature-analysis-2026-05-15.md §4`](dev-notes/iced-014-feature-analysis-2026-05-15.md#headless-mode).
  iced 0.14's `iced_test::emulator::Emulator` (PR #2698) ships
  embedded Fira Sans + a real headless runtime, so we don't need
  to author font-fallback / xvfb plumbing. **FURTHER DECOMPOSED
  2026-05-16:** the Emulator adapter portion shipped standalone as
  [`ui-headless-emulator` v0.1](ui-headless-emulator/feature.md);
  remaining scope is CI workflow + cross-platform falsifier only.

- **comet debugger revisit trigger (`ui-comet-eval`).** _candidate,
  REVISIT-GATED 2026-05-16 by operator decision_ — Q-COMET-EVAL
  LOCKED → defer indefinitely STILL APPLIES. Operator-acknowledged
  revisit trigger added: when iced 0.15.0 **stable** releases (not
  the current `0.15.0-dev` master pin), bump Q-014-PIN consideration
  + re-evaluate this candidate. Until then: no spawn trigger, no
  schedule. See
  [`iced-014-feature-analysis-2026-05-15.md §3`](dev-notes/iced-014-feature-analysis-2026-05-15.md#comet-debugger)
  for the original analysis.

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
  (`ui-contrast-asserter`).** _candidate, surfaced 2026-05-15 by
  [`spec/dev-notes/ui-testability-deep-dive-2026-05-15.md §3.8`](dev-notes/ui-testability-deep-dive-2026-05-15.md#38-stretch--pure-rust-wcag-contrast-asserter--ui-contrast-asserter)_
  — a `crates/ui/tests/contrast.rs` test that enumerates every
  `(fg, bg)` token pair in
  [`crates/ui/src/theme.rs`](../crates/ui/src/theme.rs) and asserts
  WCAG 2.1 contrast ratios per
  [`ui-design-principles.md ## Accessibility minimums`](ui-design-principles.md#accessibility-minimums)
  (4.5:1 AA body, 7:1 AAA equity). Half-day analyzed work. Closes
  an entire class of palette-refactor regression without rendering
  a single pixel. Run in WARN mode for two weeks before promoting
  to gate. Analyst spawn when operator promotes.

- **Pure-state property tests — update + proptest harness
  (`ui-update-proptest`).** _candidate, surfaced 2026-05-15 by
  [`ui-testability-deep-dive-2026-05-15.md §3.4`](dev-notes/ui-testability-deep-dive-2026-05-15.md#34-stretch--update--property-based-state-machine-harness--ui-update-proptest)_
  — drive `ui::state::update` with
  [`proptest-state-machine`](https://crates.io/crates/proptest-state-machine)
  over randomized `Message` sequences. Five invariants to start:
  kill monotonicity, no cross-screen state leakage, PanelState arm
  reachability, subscription-error recoverability, audit-write
  idempotency. ~5 dev-days. Closes ~40 `Message` variants currently
  not directly covered (analysis at
  [`ui-testability-deep-dive-2026-05-15.md §2.10`](dev-notes/ui-testability-deep-dive-2026-05-15.md#210-state-invariant-tests-vs-view-tests--quantifying-the-gap)).
  Analyst spawn when operator promotes; pairs naturally with
  `ui-mutants-pass` below.

- **Storybook-equivalent widget gallery bin
  (`ui-gallery-bin`).** _v0.1-partial shipped 2026-05-15_ —
  V1-V4 green (build, smoke, widget cell exhaustiveness, mod-rs
  cross-check). V5+ snapshot tests blocked on the iced Table cell-
  bounds panic — see [`ui-gallery-bin/tasks.md` ## Status](ui-gallery-bin/tasks.md)
  and the follow-up `ui-gallery-table-cell` candidate below.
  Original brief: [`ui-testability-deep-dive-2026-05-15.md §3.3`](dev-notes/ui-testability-deep-dive-2026-05-15.md#33-widget-gallery-binary--ui-gallery-bin).

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
  [`ui-testability-deep-dive-2026-05-15.md §3.5`](dev-notes/ui-testability-deep-dive-2026-05-15.md#35-stretch--accesskit-shadow-tree--ui-a11y-shadow)_
  — author `crates/ui/src/a11y.rs` emitting an
  [`accesskit::TreeUpdate`](https://docs.rs/accesskit) for the
  cockpit's widget surface; wire to
  [`kittest`](https://docs.rs/kittest/) for tree-based assertions
  that render zero pixels. Establishes a Layer 2 "widget tree"
  oracle (§2.14 of the dev-note) that catches half the failure
  classes pixel-diff misses (contrast, reachability, focus, label
  drift). ~7 dev-days. **Approach B (in-repo shadow), not
  Approach A (PR iced upstream)** per
  [dev-note §2.7](dev-notes/ui-testability-deep-dive-2026-05-15.md#27-accessibility-as-a-testing-surface--the-load-bearing-pivot)
  + Q-ACCESSKIT default. Iced upstream
  [issue #552](https://github.com/iced-rs/iced/issues/552)
  remains unmerged as of May 2026.

- **VLM-as-second-opinion judge (`ui-vlm-judge`).** _candidate,
  surfaced 2026-05-15 by
  [`ui-testability-deep-dive-2026-05-15.md §3.2`](dev-notes/ui-testability-deep-dive-2026-05-15.md#32-vlm-as-second-opinion-judge--ui-vlm-judge)_
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
  [`ui-testability-deep-dive-2026-05-15.md §3.1`](dev-notes/ui-testability-deep-dive-2026-05-15.md#31-live-inspect-mcp-shim--ui-inspect-mcp)_
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
  [`iced-014-feature-analysis-2026-05-15.md §5`](dev-notes/iced-014-feature-analysis-2026-05-15.md#recorder--emulator--iced_testsimulator)_
  — supersedes the original 4-dev-day `ui-session-journal`
  candidate. iced 0.14 already ships `iced_tester` (PR #3059) +
  `.ice` text format for record/replay. Adapter work is: enable the
  `record-tests` cargo feature
  ([Q-TESTER-FEATURE LOCKED](dev-notes/iced-014-feature-analysis-2026-05-15.md#migration-questions-for-the-operator)),
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
  [`ui-testability-deep-dive-2026-05-15.md §3.7`](dev-notes/ui-testability-deep-dive-2026-05-15.md#37-stretch--mutation-testing-pass--ui-mutants-pass)_
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
  update).** _candidate, surfaced 2026-05-15 by
  [`ui-testability-deep-dive-2026-05-15.md §4.1`](dev-notes/ui-testability-deep-dive-2026-05-15.md#41-testermd--emit-a-structured-fail-artifact-not-just-prose)_
  — on any visual assertion FAIL, the test-runner additionally
  writes a self-contained `spec/<slug>/reports/visual-fail-<ts>.html`
  with baseline/actual/diff PNG triple inline, the assertion that
  fired, file:line, and the VLM judge's verbatim verdict (if
  enabled). Adds a stanza to
  [`.claude/agents/tester.md`](../.claude/agents/tester.md) and a
  ~50-LOC helper in `crates/ui/tests/fixtures/`. ~1 dev-day.
  Analyst spawn when `ui-vlm-judge` or
  `ui-test-harness-viewport-matrix` schedules; pairs with whichever
  lands first.

- **File the iced strategies-Table tiny-skia panic upstream
  (`ui-iced-table-panic-upstream`).** _candidate, surfaced
  2026-05-15 by
  [`iced-014-feature-analysis-2026-05-15.md §6`](dev-notes/iced-014-feature-analysis-2026-05-15.md#the-strategies-table-panic)_
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

- **comet debugger evaluation (`ui-comet-eval`).** _candidate,
  deferred 2026-05-15 by
  [`iced-014-feature-analysis-2026-05-15.md §3`](dev-notes/iced-014-feature-analysis-2026-05-15.md#comet-debugger)
  + Q-COMET-EVAL LOCKED_ — comet is pinned at iced
  `0.15.0-dev` (master); does NOT compile against our `=0.14.0`
  pin. **No spawn trigger today.** Revisit when (a) our iced pin
  moves to 0.15.x, OR (b) `ui-inspect-mcp` /
  `ui-session-journal-iced-tester` surface a gap comet would
  close, OR (c) by 2026-11-15 (6-month calendar revisit).

_(historical: the only previously queued item, the presenter smoke test against
operator-success-reports, ran 2026-05-08; surfaced 4 findings, two
of which became skill-plumbing fixes that shipped in commit
`8b139c2`. See Recent below.)_

## Recent (shipped)

- **v2.5 TCN σ_train recalibration (`v25-tcn-recalibrate` v0.1.0)** —
  shipped 2026-05-21 (operator-approved via presenter deck
  [`presentations/v25-tcn-recalibrate-2026-05-21.md`](v25-tcn-recalibrate/presentations/v25-tcn-recalibrate-2026-05-21.md);
  Q1-Q5 = analyst defaults via "Autoapprove all"; tester VERDICT →
  PASS clean — all hard gates green). Predecessor:
  [`v25-tcn-alpha-investigation v0.1.0`](v25-tcn-alpha-investigation/feature.md).
  Parent (stays `in-progress`): `v25-tcn-overlay v2.5.0`. Metadata-
  only fix to the σ_train scalar in the BS-1 + BS-2 TCN anchored
  checkpoints — the predecessor's F-verdict investigation surfaced a
  **608× / 580× σ_train inflation** caused by an in-loop accumulator
  pattern at [`train_tcn.rs:606,676-678,733-741`](../crates/forecast/src/bin/train_tcn.rs)
  that never reset `all_r_hats` between epochs, so the final scalar
  was dominated by pre-convergence trajectory variance instead of
  the converged-model prediction std. Lands a NEW
  [`crates/forecast/src/bin/recalibrate_sigma_train.rs`](../crates/forecast/src/bin/recalibrate_sigma_train.rs)
  (~490 LoC, `--features candle`-gated) + additive `--metadata-path`
  flag on `forecast_distribution.rs` (default behaviour byte-identical
  → 22 anchor SHAs preserved) + 3 new unit tests
  (`recalibrate_sigma_train_readonly`,
  `recalibrate_sigma_train_field_invariance`,
  `sigma_train_not_in_safetensors`). New anchors locked under version
  `v2.6.1-alpha-investigation-recalibrated`: 4 total — 2 forecast-
  distribution recalibrated bodies + 2 derivation reports. Original
  `.metadata.json` + `.safetensors` files **byte-identical**
  (verified: `git diff HEAD -- crates/forecast/checkpoints/anchors/*.metadata.json`
  empty). [ADR-0035](architecture/adr/0035-tcn-sigma-train-recalibration.md)
  codifies the cross-phase σ_train recalibration contract (overlay
  file convention + on-disk JSON number divergence from ADR-0029 §2
  rule 5 + σ_train-not-in-safetensors invariant) so the same bug
  shape can't reappear in v2.5a PatchTST / v2.5b Transformer
  training scaffolds. **Substantive findings:**
  (i) σ_train bug confirmed real, eliminated (BS-1: 10.954 → 0.018;
  BS-2: 6.916 → 0.012). Both recalibrated values in expected
  0.005..0.025 range. (ii) **F-verdict stays F4** per immutable
  [ADR-0033 § D3](architecture/adr/0033-tcn-alpha-investigation-report-shape.md)
  priority tree (`frac_inside_epsilon` 0.031 / 0.057 < 0.5 F3
  threshold). (iii) **BUT gate-survival jumps dramatically**:
  BS-1 τ=0.6: **0% → 40.1%**; BS-1 τ=0.1: **0% → 88.8%**; BS-2
  similar magnitude. Surfaced standalone per Q4=(c) as the
  `## Recalibration delta` section in each recalibrated report.
  σ_train is no longer a confounding variable in the v2.5 TCN model
  assessment. **Routing decided 2026-05-21 — option (c)**: queue
  both `v25-tcn-threshold-tuning` (cheap τ-sweep, hours-not-weeks)
  and `v25-tcn-horizon-bump-or-retire` (multi-week retrain or
  retire v2.5 TCN for v2.5a PatchTST); threshold-tuning ships
  first; horizon-bump-or-retire as fallback if τ-sweep finds no
  alpha. **Anchor risk zero** — 22 originals byte-identical;
  cargo fmt + workspace clippy + `--features candle` clippy all
  clean; 7 new integration tests PASS; spec-lint 87/2 = baseline
  (0 new categories). Trace row `REQ-V25-TCN-RECALIBRATE-001`
  flipped `draft → shipped`.

- **UI rethink Phase F — Memory + Models + Phase-6 Assistant slot
  (`ui-rethink-phase-f-memory-models-assistant` v0.1.0)** —
  shipped 2026-05-21 (operator-approved via "Autoapprove all" against
  presenter deck
  [`presentations/ui-rethink-phase-f-memory-models-assistant-2026-05-21.md`](ui-rethink-phase-f-memory-models-assistant/presentations/ui-rethink-phase-f-memory-models-assistant-2026-05-21.md);
  Q1-Q8 = analyst defaults; tester VERDICT → PASS clean — sole
  deferral is H3 idle-CPU 60-s probe in the display-server class).
  Predecessor:
  [`ui-rethink-phase-e-compare v0.1.0`](ui-rethink-phase-e-compare/feature.md).
  **SIXTH AND FINAL PHASE OF THE UI RETHINK** — closes the
  [`spec/dev-notes/ui-rethink-2026-05-17.md`](dev-notes/ui-rethink-2026-05-17.md)
  redesign per §6 line 1134 ("No cliffs at C, E, F — each phase is
  independently shippable and independently reversible"). Lands all
  three deferred surfaces from dev-note §6 Phase F (lines 1098-1112):
  (i) **Memory screen** (J7) over `crates/reflection` `lesson_cards`
  store via NEW `crates/reflection/src/query.rs`
  (`list_recent_lesson_cards` / `open_and_list_recent`; UI receives
  via `Message::MemoryHydrate` — Phase D `trail_mirror` precedent
  per K1 architect resolution); reverse-chrono list + side drawer
  for entry detail; Memory→Trail chevron back-link via existing
  `OpenTrailFor` compound dispatch (additive, no Phase D body touch).
  (ii) **Models screen** (J8) over
  `crates/forecast/checkpoints/anchors/` — BS-1 + BS-2 TCN
  checkpoints inventoried via hand-parsed JSON (`#[serde(default)]`
  on every non-load-bearing field; 5 H5 unit tests cover schema
  drift); flat list with all entries rendered as "staged" at v0.1.0
  (Q7=(c)). Sparkline DEFERRED to v0.2.0 (replay-cache forecast
  namespace empty per K3 — `—` placeholder + tooltip).
  (iii) **Phase-6 Lumen Assistant slot** wakes structurally — NEW
  additive `RIGHT_RAIL_OPEN_WIDTH_PX = 320.0` (`theme.rs:~644`); old
  `RIGHT_RAIL_WIDTH_PX = 0.0` constant **preserved verbatim** per K6
  Option A so Phase D `trail_drawer.rs` stays byte-identical (R7.2 —
  T-F10 `git diff` confirmed 0 lines). Q4=(a) stub-only content
  ("Assistant offline. v2 LLM wiring lands in v0.2.0."). **K4
  resolution**: Memory drawer (centre body) + Assistant slot (far-
  right shell track) live in DIFFERENT shell columns — no right-side
  conflict. **12 net-new source files** + **6 new snapshot baselines**
  (`memory__cold_boot_empty`, `memory__steady_state_5_cards`,
  `memory__drawer_open_on_card_click`,
  `models__cold_boot_no_checkpoints`,
  `models__steady_state_2_checkpoints`,
  `assistant_slot__open_stub`); zero new external crate deps; zero
  new architecture edges. **311 lib tests PASS** (309 → +2 from
  Phase E); **ANCHORS PASS (22/22)** pre- AND post-sweep;
  layout_invariants 10/10 (7 carry-forward + 3 new = 768 panic-free
  proptest cases for the new screens); shell_grid 3/3
  (`RIGHT_RAIL_WIDTH_PX = 0.0` invariant preserved); 6 snapshot tests
  deterministic on rerun; fmt + clippy clean (default AND
  `--features live`); spec-lint 87 (= Phase E baseline; 0 new).
  H1 (cold-boot read latency) NOT FALSIFIED (reflection.db absent →
  0-row sub-ms path; static argument under load); H2 (checkpoint
  parse) NOT FALSIFIED (855B + 852B JSON ≈ 20 μs, ~50000× headroom
  over 50ms p99 budget); H4/H5/H6 all PASS; H3 idle-CPU deferred
  (display-server class). **v0.2.0 / Phase G candidates surfaced**:
  Memory cluster mode (reflection-memory distillation); Memory
  sparkline (replay-cache forecast namespace population);
  Q4=(b) full v2 LLM text-stream wire for Assistant slot; J5
  writer-side affordances; serving-status pill lifecycle.

- **UI rethink Phase E — Compare matrix (`ui-rethink-phase-e-compare` v0.1.0)** —
  shipped 2026-05-20 (operator-approved via "Autoapprove all" against
  presenter deck
  [`presentations/ui-rethink-phase-e-compare-2026-05-20.md`](ui-rethink-phase-e-compare/presentations/ui-rethink-phase-e-compare-2026-05-20.md);
  Q1-Q8 = analyst defaults; tester VERDICT → PASS clean, **no v0.1.0
  deferrals**). Predecessor:
  [`ui-rethink-phase-d-trail-followup v0.1.1`](ui-rethink-phase-d-trail-followup/feature.md).
  Fifth concrete feature in the UI rethink at
  [`spec/dev-notes/ui-rethink-2026-05-17.md`](dev-notes/ui-rethink-2026-05-17.md).
  Lands the **read-only Compare matrix** (J3) — 6 strategies × ≤10 pairs
  grid that reads cached report frontmatter under `spec/<strategy>/reports/`
  via the new `crates/ui/src/compare/cache.rs` hand-parser (no
  `serde_yaml` dep — K3 architect resolution). Cell click →
  `Message::OpenLabFromCompare { strategy, pair, range }` compound
  dispatch (mirrors Phase D `OpenTrailFor`). Empty cells render a
  per-cell **Run** affordance routed through the Phase B Lab Run
  round-trip (Q4=(b)). Greyed cells for tuples outside a strategy's
  declared universe (Q8=(b)). Universe-aggregate KPI cells (Q6=(a))
  carry a **dual-surface disclaimer** (subtitle + per-cell tooltip)
  per the architect's K7 mitigation upgrade. Sidebar entry already
  reserved by Phase C — only the body route swaps from
  `placeholder::view` to `screens::compare::view` at
  `crates/ui/src/shell.rs:96`. **5 net-new files** (`compare/mod.rs`,
  `compare/state.rs`, `compare/cache.rs`, `widgets/matrix.rs`,
  `screens/compare.rs`). **Zero new external crate deps; zero new
  architecture edges; zero anchor risk by construction.** **946 lib
  tests PASS** (939 baseline → +7 new: 5 cache + 2 H5 round-trip);
  **ANCHORS PASS (22/22)** pre- AND post-sweep; layout_invariants
  7/7 (6 carry-forward + new `compare_screen_no_zero_dim` 256-case
  proptest); 4 new snapshot baselines (`compare__cold_boot_all_empty`,
  `compare__steady_state_populated`,
  `compare__empty_cell_run_affordance`,
  `compare__column_header_hover` — byte-identical to
  cold-boot-all-empty by R2.4 design since v0.1.0 column headers are
  non-interactive); fmt + clippy clean (both default AND
  `--features live`); spec-lint 87 (= predecessor baseline, 0 new).
  H1 = **40 % first-open cache hit rate** (24/60 cells per architect
  static census; ≥30 % threshold); H4 = **≤15 ms p99 by static
  argument** (shell-level glob+head over 32 reports at 0.12 s
  wall; Rust ≥10× faster). **v0.2.0 / Phase E.1 candidates**:
  per-pair backtest decomposition (true per-pair Sharpe, closes Q6
  with (c) fallback); background recompute orchestration (Q2 (a)/(b));
  in-session cache invalidation.

- **UI rethink Phase D+ — Trail follow-up (`ui-rethink-phase-d-trail-followup` v0.1.1)** —
  shipped 2026-05-20 (operator-approved via "Autoapprove all" against
  presenter deck
  [`presentations/ui-rethink-phase-d-trail-followup-2026-05-20.md`](ui-rethink-phase-d-trail-followup/presentations/ui-rethink-phase-d-trail-followup-2026-05-20.md)).
  Predecessor: [`ui-rethink-phase-d-trail v0.1.0`](ui-rethink-phase-d-trail/feature.md).
  Closes T-D-N26 (iced **Subscription bridge** wiring
  `reflection::trail_mirror::TrailMirrorTick` into `Cockpit::subscription`;
  Q3=(c) — handle constructed in `cockpit_live.rs` bootstrap + stored on
  `AppState`), T-D-N27 (**3 new insta snapshot baselines** —
  `trail__steady_state`, `trail__side_drawer_open`,
  `live__recent_activity_with_chevron`; NEW baselines, not changes
  to anchored body-SHAs), and T-D-N29 (**H5 backfill-latency bench**
  `crates/reflection/benches/trail_mirror.rs` — p99 = **0.021 ms** ≪
  50 ms; H5 NOT falsified, ~2380× headroom). UI-local wrapper types
  (`TrailMirrorUiTick` / `TrailStageUi` / `ReconstructedTrailUi`) at
  `crates/ui/src/state.rs:~1340` keep `ui`'s default-build edge graph
  free of `reflection` (Q2=(b)); `reflection` joins as `optional = true`
  behind the existing `live` feature stanza — **zero new architecture
  edges** in the data-flow sense, ADR-0031 carry-forward honored.
  Idle-CPU sampler `scripts/bench_idle_cpu.sh` (Q4=(a) macOS `top`).
  **Deferred to v0.1.2** (sandbox display-server class, same as
  predecessor): T-F6 idle-CPU 60-s sustained probe + T-F7 K7 paper-
  mode ForecastEmitted counter (Q1=YES — deployment-side run by
  operator) + `--features live` clippy hygiene (13 pre-existing
  `needless_pass_by_value` in `crates/ui/src/live.rs:159-428`).
  **939 lib tests PASS** (≥ 937 baseline; +2 new state-tests);
  **ANCHORS PASS (22/22)** pre- AND post-sweep; layout_invariants
  6/6 PASS; 3 snapshot tests deterministic-on-rerun; fmt + default
  clippy clean; spec-lint 87 (0 regression vs predecessor baseline).

- **UI rethink Phase D — Trail view (J4) (`ui-rethink-phase-d-trail` v0.1.0)** —
  shipped 2026-05-20 (operator-approved via "Autoapprove all" against
  presenter deck
  [`presentations/ui-rethink-phase-d-trail-2026-05-20.md`](ui-rethink-phase-d-trail/presentations/ui-rethink-phase-d-trail-2026-05-20.md);
  five deferred items — T-D-N26 Iced Subscription bridge, T-D-N27 3
  snapshot baselines, T-D-N29 H5 backfill-latency bench, T-F6 idle-CPU
  floor, T-F7 K7 paper-mode counter — explicitly accepted as Phase D+
  v0.1.1 follow-up scope; wiring confirmed by inspection).
  Predecessor: [`ui-rethink-phase-c-sidebar-ia v0.1.0`](ui-rethink-phase-c-sidebar-ia/feature.md).
  Fourth concrete feature in the UI rethink at
  [`spec/dev-notes/ui-rethink-2026-05-17.md`](dev-notes/ui-rethink-2026-05-17.md).
  Lands the **decision-trail visualisation** of the multi-agent pipeline
  — Fill → Signal → Forecast as a stacked node graph via new
  `widgets::trail_node` + `widgets::trail_drawer` + `screens::trail`.
  Universal Trail chevron in Live recent-activity + audit table rows.
  **First downstream consumer of `audit-tick-consumer-envelope v0.1.0`**
  — closes T-D-14 via `TcnForecaster::with_ledger` runtime wiring at
  `crates/strategy/src/tcn_overlay_momentum.rs:417-420,434-437` +
  `post_forecast_event` emits at `crates/forecast/src/tcn.rs:861-879,997-1010`.
  **Mig 011 (anchor-safe additive)** — 4 ALTERs (NULL-default) + new
  `forecast_events` table + 4 indexes — `ANCHORS PASS (22/22)` post-mig
  (H2 confirmed). 937 lib + integration tests PASS; trail-reconstruction
  3/3 PASS; M1-C layout invariants 6/6 PASS (CI-safe cockpit-smoke
  proxy); fmt + clippy clean; spec-lint Phase D contribution = 0 new
  categories (91 violations / 2 categories vs 734/3 baseline). ADR
  amendment at
  [`adr/0031-audit-tick-consumer-envelope.md`](architecture/adr/0031-audit-tick-consumer-envelope.md)
  § "Phase D amendment (2026-05-20)".

- **UI rethink Phase C — Sidebar IA flip + Live + Strategy registry + Settings rollup (`ui-rethink-phase-c-sidebar-ia` v0.1.0)** —
  shipped 2026-05-20 (operator-approved via "Autoapprove all" against
  presenter deck
  [`presentations/ui-rethink-phase-c-sidebar-ia-2026-05-20.md`](ui-rethink-phase-c-sidebar-ia/presentations/ui-rethink-phase-c-sidebar-ia-2026-05-20.md);
  K1/K2 gut-check questions accepted as not-blockers — revisitable in
  Phase D). Predecessor:
  [`ui-rethink-phase-b-lab-run v0.2.0`](ui-rethink-phase-b-lab-run/feature.md).
  Third concrete feature in the UI rethink at
  [`spec/dev-notes/ui-rethink-2026-05-17.md`](dev-notes/ui-rethink-2026-05-17.md).
  Lands the **three-group sidebar IA** (Work zone Lab · Live ·
  Compare; Library zone Strategies · Memory · Models · Trail; Chrome
  zone Settings) with hairline `BORDER_1` dividers — entries
  unchanged from `SIDEBAR_ENTRIES_PHASE_A`, only their visual
  relationship changed. **`Live` screen** replaces the deprecated
  `Home` 2×2 grid with the dev-note §J6 layout (system-health strip
  + equity curve + KPI strip + positions + activity + placeholder
  LLM tile). **`Strategy registry`** replaces the panel-style
  `strategies::view` with a list-of-cards layout (status pill +
  universe + last-anchor + last-live-run + "Open in Lab" action).
  **`Settings` rollup** revives the dead-code `risk::view` /
  `control::view` / `debug::view` bodies under a three-tab wrapper
  (Risk · Control · Debug, default tab = Risk). One-cycle compat
  shim for deprecated `Screen::*` variants — Phase D prunes per Q1a.
  **5 net-new files** (3 screens + 2 widgets); **1 new public
  Message variant** (`SwitchSettingsTab(SettingsTab)`); no ADR
  (UI-layout scope). **22 body-SHA anchors byte-identical** (zero
  anchor risk by construction); 287 lib + 101 integration tests
  PASS; 6 new snapshot baselines + 5 refreshed; cockpit-smoke 0
  panics; spec-lint Phase C contribution = 0. **Real-world
  confirmation:** operator exercised the live cockpit this session
  and confirmed chart + hovering work end-to-end (post chart-fixture-
  line-clipping v1.0.0).

- **Audit tick consumer envelope (`audit-tick-consumer-envelope` v0.1.0)** —
  shipped 2026-05-20 (operator-approved via "Autoapprove all" against
  presenter deck
  [`presentations/audit-tick-consumer-envelope-2026-05-20.md`](audit-tick-consumer-envelope/presentations/audit-tick-consumer-envelope-2026-05-20.md);
  open Q on T-D-14 deferred to Phase D per presenter's recommendation).
  Predecessor: ADR-0031 (status `proposed → accepted` at architect
  M-T1). Adds a thin read-direction envelope (`AuditTick<E, C>`) over
  the existing audit journal: 8 in-scope `journal::*` writers enqueue
  `AuditTick`s into a `tokio::broadcast` channel; `crates/reflection`
  carries an observation-only stub consumer (gated by
  `[reflection].audit_tick_consumer_enabled = false` — keeps default
  behaviour bit-identical). **Opt-in by construction:** `Ledger::open`
  produces no tee; only the new `Ledger::open_with_tick_bus`
  constructor wires the channel. **22 body-SHA-256 anchors
  byte-identical**; spec-lint feature contribution = 0; 6 new test
  files + 1 bench file under `crates/audit/`; ForecastEmitted call
  site pinned at `crates/forecast/src/tcn.rs:786-795` (cache-hit) +
  `:889-898` (post-inference), feature-gated `audit-tick`. **Deferred
  runtime wiring** — T-D-14 (`strategy` crate optional `Ledger`
  handle) waits until Lab Trail (Phase D) needs ForecastEmitted at
  runtime; no current consumer reads it, so closing earlier would
  land dead code. ADR-0031 + `01-data-flow.md` updated.

- **Chart fixture line clipping (`chart-fixture-line-clipping` v1.0.0)** —
  shipped 2026-05-20 (operator-directed overnight fix). **Root cause:**
  iced 0.14.0 `tiny_skia` backend has a transformation-order bug in
  `Renderer::draw_primitives` (canvas group primitives applied with
  `group.transformation() * scale_factor` instead of
  `scale_factor * group.transformation()`, plus duplicate clip_bounds
  multiplier). The bug clips canvas geometry to a bottom-right sub-region
  of the canvas widget bounds. **Fix:** backport iced master commit
  [`76b32d4906`](https://github.com/iced-rs/iced/commit/76b32d4906)
  (Jan 28, 2026) via `vendor/iced_tiny_skia/` + workspace
  `[patch.crates-io]`. **Operator-locked 2026-05-20:** the vendored
  fork is the long-term canonical fix (no iced 0.14.x patch branch
  exists; no upgrade expected near-term). Any future iced bump audits
  the `Transformation::scale(scale_factor) * group.transformation()`
  ordering before retiring the fork. **Verification:** 4 visual_snapshots
  baselines refreshed (chart line now spans full 12:00→12:59 width);
  22/22 anchors byte-identical; 279 workspace tests PASS; cockpit-smoke
  0 panics. Diagnostic trail in
  [`spec/chart-fixture-line-clipping/feature.md`](chart-fixture-line-clipping/feature.md)
  preserves the orchestrator's 5-hypothesis probe register + 2 falsified
  fix attempts + final root-cause analysis.

- **Chart x-axis local time (`chart-x-axis-local-time` v1.11.0)** —
  shipped 2026-05-20 (operator-approved via "Autoapprove all"
  against presenter deck
  [`presentations/chart-x-axis-local-time-2026-05-20.md`](chart-x-axis-local-time/presentations/chart-x-axis-local-time-2026-05-20.md)).
  Predecessor: [`chart-canvas-overhaul v1.10.0`](chart-canvas-overhaul/feature.md).
  Closes the operator-friendly local-time landing deferred from
  v1.10.0 by Q-revised-1 = path (b). Trivial direct ship per
  CLAUDE.md (no analyst/architect sub-agent cycle): 1-line
  `Cargo.toml` edit adding `"local-offset"` to the `time` crate's
  features array; ~10 LOC in `crates/ui/src/widgets/chart.rs`
  splitting `local_offset_or_utc()` into a `#[cfg(test)]` UTC branch
  + a `#[cfg(not(test))]` production branch that reads
  `time::UtcOffset::current_local_offset()` with defensive
  `unwrap_or(UtcOffset::UTC)` fallback; 1 new unit test pinning the
  `cfg(test)` UTC contract. **Snapshot determinism preserved across
  host time zones** via a complementary `UI_CHART_FORCE_UTC` env-var
  gate set at the top of both integration test runners
  (`tests/render_snapshots.rs:run_panel_slot` +
  `tests/visual_snapshots.rs:run_slot`) — this corrects a latent
  issue in the predecessor M7 architect's "cfg(test) override
  holds" claim (Cargo only sets `cfg(test)` on a crate when building
  it as a test target; integration tests link against the library
  compiled WITHOUT `cfg(test)`, so the unit-test branch alone is
  insufficient). **22 / 22 anchors byte-identical** (R10.1; no
  strategy / audit / exec / report path touched); 279 workspace
  tests PASS (+1 vs Phase B baseline); cockpit-smoke PASS 0 panics;
  fmt + clippy clean; spec-lint Phase contribution = 0.

- **UI rethink Phase B — Lab Run button (`ui-rethink-phase-b-lab-run` v0.2.0)** —
  shipped 2026-05-19 (operator-approved via "Autoapprove all" against
  presenter deck
  [`presentations/ui-rethink-phase-b-lab-run-2026-05-19.md`](ui-rethink-phase-b-lab-run/presentations/ui-rethink-phase-b-lab-run-2026-05-19.md);
  6 manual `[orchestrator]` acceptance rows — H1 latency p95, H5 idle-CPU
  floor, H7 mirror RSS delta, K3 cancel-on-shutdown live test, Δ-KPI badge
  visual capture, Phase C bar-level cancel-poll scope — auto-cleared by
  the same blanket approval). Predecessor:
  [`ui-rethink-phase-a-lab v0.2.0`](ui-rethink-phase-a-lab/feature.md).
  Second concrete feature in the broader UI rethink at
  [`spec/dev-notes/ui-rethink-2026-05-17.md`](dev-notes/ui-rethink-2026-05-17.md).
  Promotes Phase A's stubbed Lab `Run` button to a real in-process
  backtest call closing the operator's J2 workflow end-to-end.
  **Headline:** `crates/backtest/src/main.rs` collapsed **3417 → 1447
  LOC** (-57%); scenario bodies extracted into
  `crates/backtest/src/scenarios/{momentum,pairs,sma_composed,tcn_overlay,tcn_overlay_weights}.rs`
  and report writers into `crates/backtest/src/report/*`;
  `engine::run_scenario` dispatches via mapping layer
  (`ScenarioConfig` → per-scenario input → unified `RunReport`); new
  `LabState.last_run_report`/`prev_run_report` rotation + new
  `widgets::run_delta_badge` (Δ P&L / Δ MaxDD / Δ Sharpe). **22/22
  anchors byte-identical** (extraction is behaviour-preserving by H2/H4
  construction); cockpit-smoke 0 panics; spec-lint Phase B contribution
  = 0; 278 workspace tests + 10 new engine::tests PASS; 5 operator-
  decide Qs all resolved to analyst-recommended defaults (Q1=A in-memory
  return; Q2=A `ThrottledSpinner` only; Q3=A disabled-while-running +
  internal cancel poll; Q4=A session-local diff; Q5=A preserve all 22
  anchors). **Known deviation (Phase C deferred):** cancel uses wrap-
  and-abort (`tokio::spawn` + drop on cancel) instead of ADR-0035 D6's
  bar-level `bar_idx & 0x7F == 0` polling; bar-level threading deferred
  to a Phase C work item. ADR-0035 (scenario-dispatch extraction pattern)
  landed. See
  [`spec/ui-rethink-phase-b-lab-run/feature.md`](ui-rethink-phase-b-lab-run/feature.md).

- **Cockpit performance + input responsiveness (`cockpit-performance-and-input-responsiveness` v1.0.0)** —
  shipped 2026-05-15 (operator-approved via presenter deck
  [`presentations/cockpit-performance-and-input-responsiveness-2026-05-15.md`](cockpit-performance-and-input-responsiveness/presentations/cockpit-performance-and-input-responsiveness-2026-05-15.md);
  this backlog entry was stale until 2026-05-19 spec-hygiene sweep).
  Predecessor: `ui-quality-gate-overhaul v1.0.0`. **Headline: idle CPU
  dropped from ~66.9% → 2.2-13.1%** on the fixtures-mode cockpit
  (~18× typical / 30× peak). M0 samply 0.13.1 profile identified the
  dominant hot path as `iced_tiny_skia::Compositor::present` at 45.5%
  inclusive + `draw_quad` at 20.5% + tiny-skia pixel pipeline at 27%+
  — i.e. continuous full-frame software-rasterized repaints at idle.
  H-PERF-1 CONFIRMED-INDIRECT, H-PERF-2 + H-PERF-4 CONFIRMED, H-PERF-3
  deferred. **M1 fix (shipped):** new `crates/ui/src/widgets/throttled_spinner.rs`
  wraps `iced_aw::Spinner` and gates its `RedrawRequested` subscription
  from **60 fps → 10 fps** (the spinner still animates smoothly; the
  cockpit's CPU stops melting). **M1B (Table memoization) + M1C
  (hit-test) NOT shipped** — post-fix CPU was already in single-digit
  range so they remain queued in tasks.md as conditional sub-targets
  for any future regression. Evaluator PASS 15/15; 280 default-feature
  tests + 286 under `--features render-debug` = 280/286 PASS.

- **Cockpit training control (`cockpit-training-control` v0.2.0)** —
  shipped 2026-05-19 (operator-approved via "Autoapprove all" against
  presenter deck
  [`presentations/cockpit-training-control-2026-05-19.md`](cockpit-training-control/presentations/cockpit-training-control-2026-05-19.md);
  3 manual `[orchestrator]` acceptance rows auto-cleared by the same
  blanket approval). Predecessor:
  [`ui-rethink-phase-a-lab`](ui-rethink-phase-a-lab/feature.md) v0.2.0.
  Integrates `train_tcn` model training into the cockpit UI as the
  natural workflow surface for the upcoming v2.5 retraining cycle and
  v2.5a/v2.5b future training rounds. Two-tier scope landed:
  **Tier 1** = Lab Train sub-panel (collapsible, bottom of Lab column)
  + subprocess spawn via `lab::trainer::spawn_training_run` (mirrors
  `lab::runner` cancellation-handle pattern) + 200-line ring-buffer
  `training_log` widget + SIGKILL-immediate Cancel semantics.
  **Tier 2** = additive SQLite migration 010 introducing the
  `training_events` table + opt-in `--audit-db <PATH>` flag on
  `train_tcn` (default omitted; byte-identical CI runs preserved) +
  1-Hz audit-DB poller iced Subscription recipe + `widgets::training_plot`
  loss-curve plot + `widgets::axis` shared Lumen primitive + cross-platform
  `pid_alive` helper + status-strip orphan-detect annotation.
  **Non-regression contract (R10) honored:** 22/22 anchors byte-identical
  (zero new anchors locked — training inputs include wall-clock + UUID
  surfaces that preclude byte-identity); cockpit-smoke PASS (0 panics in
  8s window); cockpit-training-control's own spec-lint contribution = 0;
  9 new snapshot tests + 3 new tests for `pid_alive` + 3 for
  `training_subscription` + 3 for `widgets::training_plot` + 4 for
  `training_status_strip` + 6 for `widgets::axis` + golden-CLI gate (K5
  mitigation). T-D-N1..T-D-N18 (all 18 dev rows) ticked at commit `6e5b884`;
  orchestrator-only render-baseline refresh at commits `8d1edf4`+`5ce42e6`
  (legitimate composition drift from Train sub-panel addition).
  See [`spec/cockpit-training-control/feature.md`](cockpit-training-control/feature.md).

- **Real-Binance-data backtest path (`backtest-real-binance-data` v0.1.0)** —
  shipped 2026-05-18 (operator-approved via presenter deck
  [`presentations/backtest-real-binance-data-2026-05-18.md`](backtest-real-binance-data/presentations/backtest-real-binance-data-2026-05-18.md)).
  Predecessor: [`v25-tcn-overlay`](v25-tcn-overlay/feature.md) v2.5.0 M3.
  Wires the backtest harness to read real Binance hourly parquet from
  `data/binance/` via a new `realdata` cargo feature (opt-in; default
  build never compiles the new module). New `data::revision` module
  emits + verifies a `REVISION.toml` per-file SHA-256 manifest. Four
  new `-realdata` scenarios, four new anchors under version
  `v2.6.0-realdata` (`top10-{2023,2024}-fy-tcn-overlay[-weights]-realdata`).
  19/19 anchors total; 15 originals byte-identical. **Open finding:**
  TCN real-weights produces `dampened=0` on real Binance OHLCV too —
  not a regression but unblocks the next investigation (`v25-tcn-alpha-investigation`,
  queued above). See [`spec/backtest-real-binance-data/feature.md`](backtest-real-binance-data/feature.md).

- **UI rethink Phase A — chart-centric Lab (`ui-rethink-phase-a-lab` v0.2.0)** —
  shipped 2026-05-18 (operator-approved via presenter deck
  [`presentations/ui-rethink-phase-a-lab-2026-05-18.md`](ui-rethink-phase-a-lab/presentations/ui-rethink-phase-a-lab-2026-05-18.md)).
  Predecessor: [`chart-canvas-overhaul`](chart-canvas-overhaul/feature.md) v1.10.0.
  Renames `Charts → Lab`, flips Lab to the default boot route, fuses three
  overlay layers on the single canvas (buy/sell markers + equity curve +
  ≤4-strategy comparison), adds pair-chip / strategy-chip / date-range
  widgets, persists `(strategy, pair, range, params)` with cold-start
  defaults `v1.momentum × XRPUSDT × Last 90d` (Q-A3). 358/358 ui tests +
  20/20 determinism + 13/13 anchors. Visual A/B captures deferred to
  operator-local. See
  [`spec/ui-rethink-phase-a-lab/feature.md`](ui-rethink-phase-a-lab/feature.md).

- **Drop iced_aw + iced_fonts (`ui-drop-iced-aw` v0.1.0)** — shipped
  2026-05-16. Strategic decoupling from third-party iced ecosystem
  cadence after the 2026-05-16 aborted comet bump made the
  ecosystem-lag pattern explicit. spinner already self-replaced by
  [`widgets/throttled_spinner`](../crates/ui/src/widgets/throttled_spinner.rs);
  badge replaced with native Container+Text in
  [`widgets/strategies::status_badge_cell`](../crates/ui/src/widgets/strategies.rs)
  using the same Lumen palette pairs; date_picker (smoke-test demo
  per docstring) removed entirely with its state, messages, and
  snapshot test. `cargo tree -p ui` confirms zero iced_aw +
  iced_fonts. 1216 workspace tests pass (-8 deleted-as-expected),
  anchors 11/11 PASS. **Net effort: ~3h actual vs ~18h estimate** —
  the date_picker docstring saved 2 dev-days of mistaken
  reimplementation. See
  [`spec/ui-drop-iced-aw/feature.md`](ui-drop-iced-aw/feature.md).

- **headless emulator adapter (`ui-headless-emulator` v0.1.0)** —
  shipped 2026-05-16. Decomposed out of `ui-test-harness-ci` to
  close the unchecked "headless mode" cell from
  [`iced-014-feature-analysis-2026-05-15.md §4`](dev-notes/iced-014-feature-analysis-2026-05-15.md#headless-mode)
  without waiting on viewport-matrix + evaluator prereqs. Single
  test (`crates/ui/tests/headless_emulator_smoke.rs`) boots the
  cockpit through `iced_test::emulator::Emulator`, drains events
  until `Ready`, takes a 1280×720 screenshot — proves the FULL
  iced subscription pump runs without a window server. 1224
  workspace tests pass (+1). ~1 hour actual vs ~2.25h estimate.
  See [`spec/ui-headless-emulator/feature.md`](ui-headless-emulator/feature.md).

- **session journal — iced_tester adapter
  (`ui-session-journal-iced-tester` v0.1.0)** — shipped 2026-05-16
  (commit `218cab3`). Adapter for iced 0.14's `iced_tester::attach`
  (recorder overlay) + `iced_test::run` (replay). Built with
  `--features record-tests` auto-attaches overlay; production
  builds untouched. Empty `recorded-sessions/` ships; operator
  populates post-ship via the recorder workflow. See
  [`spec/ui-session-journal-iced-tester/feature.md`](ui-session-journal-iced-tester/feature.md).

- **iced native widgets (v0.1.0)** — shipped 2026-05-13
  (operator approval recorded as `[x] Approved — ship` in
  [`spec/iced-native-widgets/presentations/iced-native-widgets-2026-05-13.md ## Approval`](iced-native-widgets/presentations/iced-native-widgets-2026-05-13.md#approval);
  evaluator `VERDICT → PASS` at
  [`reports/evaluation-2026-05-13T10-45Z.md`](iced-native-widgets/reports/evaluation-2026-05-13T10-45Z.md)
  on commit `1431409`). Brief A of the iced ecosystem evaluation
  ([predecessor v0.2.0](iced-ecosystem-evaluation/feature.md)) — 4
  hand-rolled cockpit widgets migrated to iced 0.14 native widgets:
  - `crates/ui/src/widgets/positions.rs` → native `Table`
    (commit `9027a0d`, M1 / R1)
  - `crates/ui/src/widgets/strategies.rs` → native `Table` with
    Button-in-column-1 row-click + sibling `Column<error_badges>`
    (commit `3077425`, M2 / R2)
  - `crates/ui/src/widgets/kpi_strip.rs` → native `Grid::new()
    .columns(6).spacing(space::M).height(Length::Shrink)`
    (commit `970e857`, M3 / R3)
  - `crates/ui/src/widgets/journal_transaction_modal.rs` → native
    `Float` positioning wrapping a 3-layer `Stack` (commit
    `9e5bd65`, M4 / R4)
  New shared theme submodule: `crates/ui/src/theme/iced_widget_catalogs.rs`
  exposes `cockpit_table_style_fn` factory (commit `3077425`, T2.0)
  for Brief B `iced_aw` adoption to consume. Native v0.14 `Table::new`
  has no `.style()` setter, so the factory is unused in v0.1.0 —
  consumption deferred to Brief B / v0.2.
  **4-lane parallel dev fan-out worked**: Lanes 2/3/4 spawned in
  parallel (different files, zero overlap); Lane 1 sequenced after
  Lane 2's T2.0 Catalog adapter committed. Each lane = one
  per-widget commit (4 dev commits + 1 tester commit `1431409`).
  Workflow firsts proven:
  - **Second invocation of the test-runner / evaluator split** (first
    was `ui-test-harness-bootstrap` v0.1). Evaluator default-FAIL
    contract held; 20/20 V-items (V1A-V4E) PASS in fresh context.
  - **Orchestrator-direct M0 falsifier batch** (T-M0-J through
    T-M0-N) — 5 grep checks the sub-agent sandbox couldn't run,
    completed in one orchestrator shell pass before dev fan-out
    spawned. Caught 2 architect-spec corrections (`Float::new(1
    arg)` not 2; orphan-rule violation on `impl Catalog`) before
    code was written.
  - **`scripts/orch_supplement_log.sh`** (tooling extracted from
    bootstrap v0.1) supplemented 3 sandbox-denied checks
    (`cargo doc`, shasum, clocks-grep) into the test-runner's
    log — pattern repeatable.
  4 honest architectural divergences flagged inline (orphan-rule
  pivot to StyleFn factory; `Grid::height(Shrink)` AspectRatio
  override; `Float::new(1 arg)` not 2; `Table::new` accepts
  `IntoIterator<Item=T>` looser than `Vec<T>`).
  **Net LOC** +154 (+47 positions / −30 strategies / +8 kpi /
  +29 journal / +100 new Catalog adapter) — the predecessor brief's
  "−900-1100 LOC retired" framing measured file span, not glue
  layer; **actual value is standardization** (idiomatic iced
  widgets, future-proof AccessKit hooks, less hand-rolled
  responsibility, theme adapter scaffold for Brief B). Anchor
  neutrality preserved: 11/11 byte-identical; bootstrap V8 visual
  baseline check carry-forward — 3 PNG SHAs byte-identical
  (Charts screen unaffected). 1203+ workspace tests passing.

- **v2 LLM strategy (v2.0.0)** — shipped 2026-05-13
  (operator approval recorded as `[x] Approved — ship` in
  [`spec/v2-llm-strategy/presentations/v2-llm-strategy-2026-05-13.md ## Approval`](v2-llm-strategy/presentations/v2-llm-strategy-2026-05-13.md#approval);
  tester `VERDICT → PASS` at
  [`reports/test-2026-05-12-2219-v2-llm-strategy-final.md`](v2-llm-strategy/reports/test-2026-05-12-2219-v2-llm-strategy-final.md)
  on commit `8a41b47`). Foundation-only per **Q1=A** —
  `Strategy` trait unchanged; first consumer briefs queued
  (reflection-memory-llm-enrichment + reflection-memory-trader-wiring).
  Ships the LLM substrate as callable: real `LlmProvider`
  trait + 3 provider impls (Anthropic / OpenAI-compat /
  Ollama) + retry helper + Anthropic prompt-cache builder
  + `BudgetedProvider` decorator enforcing the $200/mo
  ceiling with auto-degrade at 80% (Q6 + Q11) + strict
  SQLite replay cache (D2 / Q8) + 9-row fixture cache + V9
  secrets grep + 3-provider × 3-role `llm-smoke` harness +
  two operator runbooks (`spec/runbooks/llm-{cost,replay}.md`).
  Q4 bonus rename **`cost::LlmProvider` enum → `ProviderKind`**
  (D1) freed the `LlmProvider` name for the trait; 5 call
  sites + serde wire shape preserved → zero on-disk ledger
  byte change. Q5d "Cache hit ratio" System Health row +
  Q11 denominator `$135 → $200` regenerated both
  `success-fixed-report-sample-{7d,90d}.md` bodies; tester
  re-locked the 2 corresponding anchors at T_FINAL
  (`520b1f29…` / `c656414e…`). The 9 strategy backtest
  anchors at `spec/anchors.toml:15-58` stay byte-identical
  (R14.2 / V8 enforced by T1937 negative-invariant — 11/11
  PASS).
  **Workflow shape**: 6 multi-pass developer cycles
  (`d0bcad2` → `c61afa5` → `441c136` → `f1dbe05` →
  `f1128e9` → `faaaec1`) over 2 days + tester gate
  (`8a41b47`). Two `[~]` partials flipped to `[x]` mid-
  cycle as their dependencies landed (T1912 audit-memo in
  pass 4; T1913 factory Research/Recording arms in pass 5).
  **44/45 dev tasks ticked** (T1938 cockpit "LLM budget"
  tile deferred to v2.1 + T1915 tracing-Layer half deferred
  + 2 pedantic clippy on `audit/src/query.rs:219,221` to
  v2.1 — all consolidated into `v2-llm-strategy-v21-followups`
  candidate). **1203 workspace tests passing, 0 failed.**
  **Unblocks**: Kronos v2.5 forecast overlay, Lumen Phase 6
  Assistant slot, reflection-memory follow-up briefs.
  Brief carries the architect-misdiagnosis-prevention
  workflow rules (Capability boundaries amendment from
  2026-05-12) only informally — the feature shipped on the
  pre-amendment single-tester model; future features apply
  the new test-runner/evaluator split.

- **UI test harness bootstrap (v0.1)** — shipped 2026-05-12
  (operator approval recorded as `[x] Approved — ship` in
  [`spec/ui-test-harness-bootstrap/presentations/ui-test-harness-bootstrap-2026-05-12.md ## Approval`](ui-test-harness-bootstrap/presentations/ui-test-harness-bootstrap-2026-05-12.md#approval);
  evaluator `VERDICT → PASS` in
  [`reports/evaluation-2026-05-12T13-15Z.md`](ui-test-harness-bootstrap/reports/evaluation-2026-05-12T13-15Z.md)).
  **First feature under the new `AGENT.md ## Capability boundaries`
  regime AND the first run of the test-runner / evaluator split.**
  Implemented week 1 of the 4-week dev-note adoption plan:
  `iced_test::screenshot` smoke test at three operator viewport
  slots (1280×720 / 1920×1080 / 3360×1890 @ 2.0); `image-compare`
  perceptual-diff forensics on snapshot failure; canvas hit-test
  grid sweep over every marker centroid at every viewport — closes
  detection-half of chart-canvas-overhaul V15; viewport-parametric
  helper on `dispatch_canvas_event_for_test`;
  `scripts/check_no_clocks_in_ui_tests.sh` determinism gate.
  Three baseline PNGs committed at
  [`crates/ui/tests/visual-baselines/charts_screen_dark_{floor,typical,operator}.png`](../crates/ui/tests/visual-baselines/).
  **V8 PASS-with-H2-caveat**: render-half of chart-canvas-overhaul
  V15 (visible tooltip card in baseline) deferred to
  `ui-test-harness-canvas-state-seeding` candidate (Queue) per
  operator decision "Commit — V14 covered, V15 partial-accept"
  2026-05-12. New deps: `iced_test = 0.14.0`, `image-compare = 0.4`,
  `image = 0.25.6` (all dev-deps; zero production runtime impact).
  **Workflow meta-deliverables proven**: (a) architect's M0 API audit
  caught a load-bearing `iced_test::Snapshot::png()` assumption
  (method doesn't exist) before code was written; (b) developer
  caught a second `iced_test::screenshot → iced::window::Screenshot`
  API correction during M1 implementation and adjusted cleanly;
  (c) test-runner emitted raw log with honest `[~]` partial for
  4 sandbox-denied checks; (d) orchestrator supplemented those
  checks verbatim; (e) evaluator (read-only, fresh context,
  default-FAIL contract) emitted PASS with file:line cites for
  every V-item. Zero capability-boundary violations across all 6
  agent roles. Anchors PASS 11/11 byte-identical; 818 existing
  tests stay green; 8 net-new tests added; zero non-UI-crate
  changes. Weeks 2 / 3 / 4 follow-ups queued in
  [`## Process / tooling`](#process--tooling).

- **Chart canvas overhaul (v1.10.0)** — shipped 2026-05-12
  (operator approval recorded as `[x] Approved — ship` in
  [`spec/chart-canvas-overhaul/presentations/chart-canvas-overhaul-2026-05-12.md ## Approval`](chart-canvas-overhaul/presentations/chart-canvas-overhaul-2026-05-12.md#approval);
  no overrides, no follow-up notes). Closes the six operator-
  reported items from the v1.9.0 retrospective: price axis on
  the LEFT gutter (USD labels), time axis on the BOTTOM gutter
  (HH:MM UTC), TradingView-style centering via
  `inner_rect_with_gutters`, top-right legend card
  (`PANEL_SUNKEN` fill + `BORDER_STRONG` outline at
  [`crates/ui/src/widgets/chart_legend.rs:156-160`](../crates/ui/src/widgets/chart_legend.rs#L156)),
  viewer parity for `equity_curve` + `drawdown_band`, default
  window bump to 1920×1080 (min stays 1280×720), tooltip card
  clamp to inner-rect bounds. V15 (live tooltip-hover screenshot
  at 3360×1890) DEFERRED to the queued
  `ui-test-harness-bootstrap` v0.1 feature per operator decision
  D4 in [`spec/dev-notes/ui-testing-direction-2026-05-12.md ## Section 9`](dev-notes/ui-testing-direction-2026-05-12.md#9-open-decisions-for-the-operator)
  — the first `iced_test::Simulator::snapshot().matches_image()`
  chart-hover test in that feature replaces the manual capture.
  Q4 local-time x-axis labels DEFERRED to v1.11
  `chart-x-axis-local-time` (shipped 2026-05-20, see Recent;
  UTC fallback shipped in v1.10.0 was the bridge). Retrospective surfaced the architect's
  "iced 0.14 canvas-scale bug" misdiagnosis (empirically
  disproved by orchestrator's red-rect + cyan-dot probe; T3002 /
  T3003 / T3007 / T3008 closed as no-op) — produced the
  `AGENT.md ## Capability boundaries` amendment (D5,
  load-bearing) + [`spec/dev-notes/ui-testing-direction-2026-05-12.md`](dev-notes/ui-testing-direction-2026-05-12.md)
  strategy document + `ui-test-harness-bootstrap` v0.1 follow-on
  feature now in Active. Anchor neutrality preserved: 11/11
  byte-identical (`bash scripts/verify_anchors.sh → ANCHORS PASS
  (11 / 11)`); zero changes to `crates/strategy/`, `crates/risk/`,
  `crates/backtest/`, `crates/reports/`, `crates/exec/`,
  `crates/audit/`, `crates/agent/`, `crates/core/`,
  `crates/reflection/`.

- **Chart buy/sell emphasis (v1.9.0)** — shipped 2026-05-11
  (operator verbal approval recorded as `[x] Approved — ship` in
  the presenter deck at
  [`spec/chart-buy-sell-emphasis/presentations/chart-buy-sell-emphasis-2026-05-11.md`](chart-buy-sell-emphasis/presentations/chart-buy-sell-emphasis-2026-05-11.md);
  no overrides, no follow-up notes). UI feature opened directly
  from operator visual feedback on the v1.8 cockpit — markers
  bigger + outlined + line-anchored + 6-field hover tooltip + click-
  through to existing journal_transaction_modal (R4.5 second
  consumer of the tape-row-audit-modal pattern); layered
  fills+signals ghost markers (R5 — signal source default-off via
  new `SignalLogConfig`); three counter views (cumulative-window
  volume tile + per-bar histogram + open-position mirror) above /
  below chart in Layout β; min window size 1280×720; Lumen window
  icon plumbing (macOS dock-icon limitation documented — needs
  `.app` bundle, candidate stub at
  [`spec/cockpit-app-bundle/feature.md`](cockpit-app-bundle/feature.md)).
  Anchor neutrality preserved: zero changes to `spec/anchors.toml`
  (11/11 byte-identical); zero modifications to `crates/strategy/`,
  `crates/risk/`, `crates/backtest/`, `crates/reports/`,
  `crates/exec/`. New additive surface: `crates/audit/migrations/009_strategy_signals.sql`
  + `audit::query::recent_signals` reader + `core::SignalView` type
  + `agent::SignalLogConfig { enabled: false }` + 2 new widgets
  (`chart_tooltip`, `volume_histogram`) + shared window-chrome helper
  (`window_icon`) + Lumen mark RGBA asset. Tester report:
  [`spec/chart-buy-sell-emphasis/reports/test-2026-05-11-2103-chart-buy-sell-emphasis-final.md`](chart-buy-sell-emphasis/reports/test-2026-05-11-2103-chart-buy-sell-emphasis-final.md)
  (V1–V13 all PASS; 1000 / 0 / 4 tests across 144 binaries;
  11/11 anchors PASS). Brief:
  [`spec/chart-buy-sell-emphasis/feature.md`](chart-buy-sell-emphasis/feature.md)
  (status: shipped, version: 1.9.0). Multi-cycle implementation
  arc: initial dev+ui-designer parallel ship + M6 follow-up
  (T2028–T2030) + M6.2 second follow-up (T2031–T2033) + hardening
  pass (corrected T2032 doc rationale + screenshot evidence). The
  iterative loop reflected headless-agent's inability to visually
  verify; addressed long-term by Screen Recording permission grant
  to the host IDE + documented screenshot-verification gate in
  M6.2 task bodies.
- **Reflection memory (v1.8.0)** — shipped 2026-05-10 (operator
  verbal approval recorded as `[x] Approve with notes` in the
  presenter deck at
  [`spec/reflection-memory/presentations/reflection-memory-2026-05-08.md`](reflection-memory/presentations/reflection-memory-2026-05-08.md);
  one note: flip `ReflectionConfig::enable_writer` default from
  `false` to `true` — applied in the same commit as approval).
  Replaces the R6 placeholder body in
  [`crates/reports/src/render/memory_highlights.rs`](../crates/reports/src/render/memory_highlights.rs)
  with real reflection-memory output. New leaf crate
  [`crates/reflection/`](../crates/reflection/) (lib only — types,
  regime + outcome classifiers, deterministic 32-dim embedding,
  post-mortem-analyst card generator, `ReflectionStore` trait + a
  `SqliteReflectionStore` linear-scan top-K impl, bounded mpsc
  writer task with Prometheus drop counter, retrieval API). Wired
  through `crates/agent/src/{config,main}.rs` + `crates/exec/src/paper.rs`.
  Re-locked the two `report-sample-*` anchors at
  `spec/anchors.toml:67-75`; the 9 strategy-backtest anchors at
  lines 15–58 are byte-identical (negative-invariant test t1812
  enforces). Q-resolutions: Q1 = Option A (deterministic v1, no
  LLM dependency); Q4 = report-only (Strategy trait unchanged);
  Q5 = distillation deferred to follow-up brief
  `reflection-memory-distillation`. Tester report:
  [`spec/reflection-memory/reports/test-2026-05-08-2114-reflection-memory-final.md`](reflection-memory/reports/test-2026-05-08-2114-reflection-memory-final.md)
  (V1–V10 all PASS; 952 / 0 / 3 tests across 124 binaries; 11/11
  anchors PASS; cargo deny advisories/bans/licenses/sources all
  ok). Brief: [`spec/reflection-memory/feature.md`](reflection-memory/feature.md)
  (status: shipped, version: 1.8.0).
- **Presenter smoke test on `operator-success-reports`** — shipped
  2026-05-08 (operator verbal approval recorded as `[x] Approved —
  ship` in commit `587dad7`). Deck at
  [`spec/operator-success-reports/presentations/operator-success-reports-2026-05-08.md`](operator-success-reports/presentations/operator-success-reports-2026-05-08.md);
  pulled evidence from the archived final tester PASS (extracted
  from `spec/archive/pre-lumen-tester-reports-2026-04-to-05-03.tar.gz`)
  + a fresh `cargo test -p reports --test report_scenarios` re-run
  (4/4 PASS, body SHAs match anchors) + a fresh
  `scripts/verify_anchors.sh` PASS (11/11). Surfaced 4 smoke-test
  findings: (1) `present-results` skill missed the archive
  fallback for pre-Lumen tester reports — fixed in `8b139c2`;
  (2) `capture-screenshot` skill defaulted to a manual-capture
  instruction for non-UI features — fixed in `8b139c2` with a third
  "non-UI feature" branch; (3) backlog Recent section had stale
  relative paths inside link parens (cosmetic, fixed in `1a63156`);
  (4) confirmed the audit-immutability call on archived tester
  reports is correct (their internal `spec/features/...` /
  `spec/tasks/...` references describe the layout at time of
  writing). The presenter pipeline is now battle-tested before the
  next real-feature fire.

- **Lumen Phase 5 — HumanControl + AgentFeed rename** — shipped
  2026-05-07 (tester second-pass PASS, presenter approved
  2026-05-08). Tester reports at
  [`spec/lumen-design-adoption/phase-5-humancontrol-agentfeed/reports/test-2026-05-07-lumen-phase-5-humancontrol-agentfeed.md`](lumen-design-adoption/phase-5-humancontrol-agentfeed/reports/test-2026-05-07-lumen-phase-5-humancontrol-agentfeed.md)
  (first-pass FAIL on fmt drift) and
  [`spec/lumen-design-adoption/phase-5-humancontrol-agentfeed/reports/test-2026-05-07b-lumen-phase-5-humancontrol-agentfeed.md`](lumen-design-adoption/phase-5-humancontrol-agentfeed/reports/test-2026-05-07b-lumen-phase-5-humancontrol-agentfeed.md)
  (second-pass PASS); brief at
  [`features/lumen-phase-5-humancontrol-agentfeed.md`](lumen-design-adoption/phase-5-humancontrol-agentfeed/feature.md)
  (status: `shipped`); presenter deck at
  [`spec/lumen-design-adoption/presentations/lumen-design-adoption-2026-05-04-to-05-08.md`](lumen-design-adoption/presentations/lumen-design-adoption-2026-05-04-to-05-08.md).
  **First phase to ship net-new operator-write surfaces since v0**:
  HumanControl panel widget (execution-mode segmented control +
  daily loss limit / max position / used-today P&L mirror rows +
  kill button as bottom action) on a new "Control" sidebar entry
  (7th); single-click pause-strategy toggle on Strategies-detail;
  typed-confirm `OVERRIDE` flow for risk-veto override per surfaced
  veto event. Two new audit writers (`strategy_paused`,
  `risk_veto_overridden`) — additive `StrategyEventKind` variants
  with no SQL migration (column already TEXT). Module rename
  `tape` → `agent_feed` via `mv` + git rename detection;
  `Cockpit::tape` field name preserved (Q14) to avoid 100+ test
  ripple. **TD-1 four-phase deferral CLOSED** via Path (b) —
  `crates/ui/src/widgets/focus_ring.rs` Subscription-driven
  custom-widget escape hatch wraps all four destructive surfaces
  with a visible accent-bordered halo on focus. New TD-2 row
  added: risk-engine veto-emit upstream wiring deferred (Phase 5
  ships override surface over an empty live `Vec<VetoEvent>`;
  not a safety primary, an observability gap). Anchor risk: zero
  — verified PASS at ship (11/11 byte-identical post additive
  audit-writer additions). 896 tests passed across 110 binaries
  (46 + 2 net-new vs Phase 4); rust-validate clean (after one-line
  `cargo fmt --all` fixup between tester passes); 86 baselines
  attested by ui-designer (67 panel + 17 widget + 2 audit; 13
  net-new + 9 renamed); R16.3 brand-bleed grep clean. Architect
  ratified 15/15 Q-items with zero principled overrides.
  **Phase 5 is the last shippable phase of the lumen-design-adoption
  initiative absent v2 LLM** — Phase 6 (Assistant slot) is reserved
  until the v2 LLM strategy lands.

- **Lumen Phase 4 — Backtest panel (`viewer` bin)** — shipped
  2026-05-06 (tester second-pass PASS, presenter approved
  2026-05-06). Tester reports at
  [`spec/lumen-design-adoption/phase-4-backtest-panel/reports/test-2026-05-06-lumen-phase-4-backtest-panel.md`](lumen-design-adoption/phase-4-backtest-panel/reports/test-2026-05-06-lumen-phase-4-backtest-panel.md)
  (first-pass FAIL on `clippy::match_same_arms`) and
  [`spec/lumen-design-adoption/phase-4-backtest-panel/reports/test-2026-05-06b-lumen-phase-4-backtest-panel.md`](lumen-design-adoption/phase-4-backtest-panel/reports/test-2026-05-06b-lumen-phase-4-backtest-panel.md)
  (second-pass PASS); brief at
  [`features/lumen-phase-4-backtest-panel.md`](lumen-design-adoption/phase-4-backtest-panel/feature.md)
  (status: `shipped`); presenter deck at
  [`spec/lumen-design-adoption/presentations/lumen-design-adoption-2026-05-04-to-05-08.md`](lumen-design-adoption/presentations/lumen-design-adoption-2026-05-04-to-05-08.md).
  Adds the new `viewer` binary at `crates/ui/src/bin/viewer.rs`
  (workspace now ships 3 bins), KPI strip + equity curve + drawdown
  band widgets sharing a refactored `widgets::canvas_chart` core
  with Phase 2's price chart, the cross-phase `core::EquitySeries`
  primitive (rich struct with precomputed drawdown vector inside
  `EquityPoint`), additive `audit::query::equity_curve_for_strategy(strategy_id, since, until)`
  sibling of Phase 2/3 filtered queries, markdown summary parser
  with graceful "—" fallback for missing fields (the 11 anchored
  sample reports omit CAGR + Win rate by design), and **closes the
  Phase 3 deferral** (Strategies-detail sparkline placeholder
  retires; real `widgets::sparkline` lands fed by the new audit
  query). Anchor risk: zero — verified PASS at ship (11/11 byte-
  identical, viewer reads existing committed reports). 850 tests
  passed across 108 binaries (40 + 4 net-new vs Phase 3); rust-
  validate clean (after orchestrator's one-line `match_same_arms`
  fix between tester passes); 72 baselines attested by ui-designer;
  R16.3 brand-bleed grep clean. Architect ratified 12/12 Q-items
  with zero principled overrides (Q1 shape refinement: drawdown_pct
  nested inside `EquityPoint` rather than parallel Vec — eliminates
  length-coupling). **Phase 5 inherits the TD-1 tightening point**:
  Phase 5 ships net-new operator-write controls with typed-confirm
  flows, making the focus-ring deferral (iced still pins `=0.14.0`)
  load-bearing — Phase 5 either folds the iced 0.15+ upgrade or
  commits to the custom-widget escape hatch.

- **Lumen Phase 3 — Detail screens (Strategies / Risk / Audit)** —
  shipped 2026-05-05 (tester first-pass PASS, presenter approved
  2026-05-06). Tester report at
  [`spec/lumen-design-adoption/phase-3-detail-screens/reports/test-2026-05-05-lumen-phase-3-detail-screens.md`](lumen-design-adoption/phase-3-detail-screens/reports/test-2026-05-05-lumen-phase-3-detail-screens.md);
  brief at
  [`features/lumen-phase-3-detail-screens.md`](lumen-design-adoption/phase-3-detail-screens/feature.md)
  (status: `shipped`); presenter deck at
  [`spec/lumen-design-adoption/presentations/lumen-design-adoption-2026-05-04-to-05-08.md`](lumen-design-adoption/presentations/lumen-design-adoption-2026-05-04-to-05-08.md).
  Adds Strategies / Risk / Audit sidebar entries (3 → 6) + per-screen
  detail bodies, additive `008_journal_transactions_venue.sql`
  migration (default `'binance'` backfill), `post_fill` writer's
  new `venue: Venue` parameter wired across ~25 call-sites,
  `RiskTelemetry` channel on `agent::EventBus` mirroring `MarketHealth`,
  sibling audit query `recent_journal_filtered` (additive to
  `recent_fills_filtered`), kill-threshold proximity gauge with
  tri-band ramp (`UP_500` ≤70% → `WARN_500` >70% → `DOWN_500` >90%),
  Audit screen filter chip-row (venue · symbol · kind · time-range)
  + fixed 250-row pagination + reuse of T1208 modal, cross-link
  Home → Strategies-detail. Two developer passes (pass 1 cut at
  clean tick boundary after T1701 + T1703 due to context budget;
  pass 2 ticked T1702 migration + T1704–T1716). Architect ratified
  11/11 Q-items with one deferral (Q6 equity-since-deploy sparkline
  → Phase 4, since the cheap path doesn't exist on the current
  state shape and Phase 4 needs the same equity-history primitive).
  Anchor risk: zero — verified PASS at ship (11/11 byte-identical
  post-migration, verified twice during dev pass + once at tester
  gate). 810 tests passed across 104 binaries (29 + 6 net-new vs
  Phase 2); rust-validate clean (fmt + clippy `-D warnings` +
  cargo-deny + docs); 65 baselines attested by ui-designer
  (zero `unknown` token escapes); R16.3 brand-bleed grep clean.

- **Lumen Phase 2 — Shell IA + Charts** — shipped 2026-05-05.
  Tester first-pass `VERDICT → PASS`; report at
  [`spec/lumen-design-adoption/phase-2-shell-ia-charts/reports/test-2026-05-05-lumen-phase-2-shell-ia-charts.md`](lumen-design-adoption/phase-2-shell-ia-charts/reports/test-2026-05-05-lumen-phase-2-shell-ia-charts.md).
  Brief at
  [`features/lumen-phase-2-shell-ia-charts.md`](lumen-design-adoption/phase-2-shell-ia-charts/feature.md)
  (status: `shipped`). Presenter deck approved by operator at
  [`spec/lumen-design-adoption/presentations/lumen-design-adoption-2026-05-04-to-05-08.md`](lumen-design-adoption/presentations/lumen-design-adoption-2026-05-04-to-05-08.md).
  Adds left-sidebar shell (fixed 180 px, T1507-styled, no icons),
  Screen routing (`Cockpit::current_screen` × six variants — Home /
  Debug / Charts wired in Phase 2; Strategies / Risk / Audit
  declared for Phase 3), Home screen (Phase 1 widgets re-housed),
  Debug screen (kill / latency / market health / server time /
  version / placeholder logs), Charts screen (chip-row symbol
  selector + canvas line-series price plot + buy/sell triangle
  markers from `recent_fills_filtered`), per-`(Venue, Symbol)`
  rolling `ChartBuffer` (cap 60 1-min bars), live mode via existing
  `bars_tx`, fixtures mode via deterministic `synthetic_candles`,
  additive `audit::query::recent_fills_filtered(venue, symbol,
  since, until)`, right-rail Phase 6 Assistant slot reservation
  (`Length::Fixed(0.0)`). Anchor risk: zero — verified PASS at
  ship (11/11 anchors byte-identical). 781 tests passed across 98
  binaries (24 + 2 net-new vs Phase 1); rust-validate clean
  (fmt + clippy `-D warnings` + cargo-deny + docs); 53 baselines
  attested by ui-designer (zero `unknown` token escapes); R16.3
  brand-bleed grep clean. All 11 architect Q-resolutions ratified
  with zero deviations from analyst recommendation. **Phase 3
  prerequisite carried forward**: additive `journal_transactions.venue`
  column migration needed before non-Binance fills can populate
  the chart's marker query.

- **Lumen Phase 1 — Foundation (tokens + tiers + status bar)** —
  shipped 2026-05-04. Tester third-pass `VERDICT → PASS`; report at
  [`spec/lumen-design-adoption/phase-1-foundation/reports/test-2026-05-04c-lumen-phase-1-foundation.md`](lumen-design-adoption/phase-1-foundation/reports/test-2026-05-04c-lumen-phase-1-foundation.md).
  Brief at
  [`features/lumen-phase-1-foundation.md`](lumen-design-adoption/phase-1-foundation/feature.md)
  (status: `shipped`). Replaced the 12-token theme with the full
  Lumen palette (warm + cool neutrals, accent ramp, sage / clay /
  warn / info semantics, both light and dark modes); added Tier
  0/1/2/3 elevation surface tokens; added whisper shadows + sunken
  inset; added focus ring (Q11 / TD-1 deviation: hover-state ring
  on buttons + ACCENT border-shift on focused inputs as bounded
  approximation, two named upgrade triggers); extended spacing /
  radii / typography ladders to the full Lumen scale; added motion
  tokens; applied Tier 1 styling to existing 6 widgets; applied
  sunken styling to the kill-confirm input; applied the active-row
  2 px left rule to tabular widgets; added a new
  `widgets::status_bar` widget rendering connection / latency /
  account / server time always-visible at the bottom of the shell;
  refreshed the existing 36 panel snapshot baselines (5 net-new for
  T1506 / T1508 = 41 total); superseded
  [`spec/ui-design-principles.md`](ui-design-principles.md) with a
  Lumen-anchored rewrite. Anchor risk: zero — verified PASS at
  ship (11/11 anchors byte-identical). 757 tests passed across 96
  binaries; rust-validate clean (fmt + clippy + cargo-deny + docs);
  R16.3 brand-bleed grep clean. Unblocked the 2026-05-04 master
  roadmap revision (4 → 6 phases).
- **v1.5b multi-venue + 1s aggregated trades** — shipped 2026-05-03.
  Brief at
  [`features/v1-5b-multi-venue.md`](v1-5b-multi-venue/feature.md).
  Coinbase + Kraken adapters, USDC pair mirror set (10 symbols),
  T612 multi-symbol live `BinanceFeed`, 1 s aggregated trades,
  `Venue` enum on `Tick` / `Bar`, per-venue feed-reconnect
  provenance. Plumbing-only — expanded the data side, not the
  execution side. 15 R-items, 12 V-items, 12 open questions
  resolved. Anchor risk: zero by construction (no `venue` strings
  in any committed report body). Closes v1.5a Q5 (USDC pairs
  blocker) and v1 closeout T612. The cockpit / `cockpit_live` is
  now stable on the data side; this is the clean window the
  Lumen design adoption initiative lands into.

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
  [`iced-014-feature-analysis-2026-05-15.md`](dev-notes/iced-014-feature-analysis-2026-05-15.md)
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
  [`spec/dev-notes/ui-testability-deep-dive-2026-05-15.md`](dev-notes/ui-testability-deep-dive-2026-05-15.md)
  — a research dev-note critiquing the four-week plan in
  [`ui-testing-direction-2026-05-12.md`](dev-notes/ui-testing-direction-2026-05-12.md),
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
  [dev-note §6 4-week plan](dev-notes/ui-testing-direction-2026-05-12.md#6-phased-adoption--4-week-plan)
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
  [`lumen-design-adoption/presentations/lumen-design-adoption-2026-05-04-to-05-08.md`](lumen-design-adoption/presentations/lumen-design-adoption-2026-05-04-to-05-08.md)
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
  [`lumen-design-adoption/presentations/lumen-design-adoption-2026-05-04-to-05-08.md`](lumen-design-adoption/presentations/lumen-design-adoption-2026-05-04-to-05-08.md)
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
  [`lumen-design-adoption/presentations/lumen-design-adoption-2026-05-04-to-05-08.md`](lumen-design-adoption/presentations/lumen-design-adoption-2026-05-04-to-05-08.md)
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
  [`lumen-design-adoption/presentations/lumen-design-adoption-2026-05-04-to-05-08.md`](lumen-design-adoption/presentations/lumen-design-adoption-2026-05-04-to-05-08.md)
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
  [`lumen-design-adoption/presentations/lumen-design-adoption-2026-05-04-to-05-08.md`](lumen-design-adoption/presentations/lumen-design-adoption-2026-05-04-to-05-08.md)
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
  [`spec/dev-notes/ui-testing-direction-2026-05-12.md`](dev-notes/ui-testing-direction-2026-05-12.md)
  strategy document + queued `ui-test-harness-bootstrap` v0.1
  feature now in Active per operator decisions D1-D5. Anchors
  PASS 11/11 (verbatim line in deck `## Live demo` block).
  Approval evidence:
  [`chart-canvas-overhaul/presentations/chart-canvas-overhaul-2026-05-12.md ## Approval`](chart-canvas-overhaul/presentations/chart-canvas-overhaul-2026-05-12.md#approval).

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
  [`ui-test-harness-bootstrap/presentations/ui-test-harness-bootstrap-2026-05-12.md ## Approval`](ui-test-harness-bootstrap/presentations/ui-test-harness-bootstrap-2026-05-12.md#approval).

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
  [`v2-llm-strategy/presentations/v2-llm-strategy-2026-05-13.md ## Approval`](v2-llm-strategy/presentations/v2-llm-strategy-2026-05-13.md#approval).

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
  [`iced-native-widgets/presentations/iced-native-widgets-2026-05-13.md ## Approval`](iced-native-widgets/presentations/iced-native-widgets-2026-05-13.md#approval).
