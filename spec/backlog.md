---
slug: backlog
status: living
owner: orchestrator
updated: 2026-05-27
---
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
  [`spec/cockpit-activity-status-bar/presentations/cockpit-activity-status-bar-2026-05-26.md`](cockpit-activity-status-bar/presentations/cockpit-activity-status-bar-2026-05-26.md)).
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
     [`testing-framework-audit-2026-05-25 § R1`](dev-notes/testing-framework-audit-2026-05-25.md)
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
     [`spec/dev-notes/testing-framework-audit-2026-05-25.md § R1`](dev-notes/testing-framework-audit-2026-05-25.md).
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
  [`spec/reflection-memory-trader-wiring/presentations/reflection-memory-trader-wiring-2026-05-26.md`](reflection-memory-trader-wiring/presentations/reflection-memory-trader-wiring-2026-05-26.md).
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

<!-- PROMOTED Queue → Active 2026-05-27 (analyst M0). Brief authored at
     spec/v5-latency-slippage-sim-v0.3.0-full-path-wiring/feature.md;
     tasks at spec/v5-latency-slippage-sim-v0.3.0-full-path-wiring/tasks.md;
     trace row REQ-V5-FULL-PATH-WIRING-001 at EOF of spec/trace.toml in
     proposed state. See Active section above for the live tracking row. -->
<!-- - **v5-latency-slippage-sim v0.3.0 (full-path wiring + data-source-drift decision + t1937 test refresh).**
  (Queue stub preserved as comment for archeology; live row now in Active.) -->

- **v2.5 TCN horizon-bump or retire (`v25-tcn-horizon-bump-or-retire`).**
  _moved Queue → Active 2026-05-21 (analyst pass)_ — see
  [Active section](#active) for the live tracking row and
  [`feature.md`](v25-tcn-horizon-bump-or-retire/feature.md) for the
  full brief. The original 4-bucket scope framing is preserved here
  as a pivot reference: (a) horizon-bump retrain (~5-21 days);
  (b) retire-promote-PatchTST (~4-6 weeks); (c) both in parallel
  (~6-9 weeks); (d) defer-on-live (~30-90 days). Q1 (primary scope)
  is operator-decide HARD BLOCKER with no safe analyst default.

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

- **v2.5a — PatchTST forecast overlay (`v25a-patchtst-overlay`).**
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

- **v3 — Regime classifier (`v3-regime-classifier`).**
  _Queued DEFERRED-2026-05-22 retained pending C5 ship — NOT Active._
  Original deferral framing was "C1 ships first; C2 + C5 analyst-only
  spec until C1 verdict" (operator Q-SEQ HYBRID 2026-05-22 AM session).
  The C1 programme retired 2026-05-22 PM with NEGATIVE-NET-DELTA real
  evidence after the v3-volatility-forecaster-noop-fix v0.1.0 fix
  wave; operator picked **C5 over C2 at C1's retirement** for moat-
  alignment + crates/llm infra reuse. C2 stays in Queue as the next-
  in-line fallback pick if C5 returns F-equivalent (R-O3 routing
  cell of the C5 active block) — explicit precedent for "if C5
  fails, re-route to C2" preserved in the C5 active comment block
  above. Candidate 2 of three picks ({C1 volatility + C2 regime +
  C5 LLM-as-forecaster}) from the
  [strategy-reformulation survey](dev-notes/strategy-reformulation-survey-2026-05-22.md)
  resolution. Per operator-decide Q-SEQ = **HYBRID**: C1 (volatility)
  ships first; this Candidate-2 analyst pass produced a spec-only
  design brief at
  [`feature.md`](v3-regime-classifier/feature.md) (R1-R8, H1-H6,
  Q1-Q7 with analyst-recommended defaults, 8-item non-regression
  contract, deferred-milestone activation contract) and a `[[req]]`
  row `REQ-V3-REGIME-CLASSIFIER-001` in `draft` state. **Architect
  M-T1 + developer waves DEFERRED** — activation gate is C1 verdict
  landed AND (operator routing = promote-C2 OR Sharpe-delta on C1
  ≥ +0.10 auto-progression). **Load-bearing finding:**
  [`crates/reflection/src/regime.rs`](../crates/reflection/src/regime.rs)
  already ships a pure-fn 3-state BTC daily-close regime tagger
  (`RegimeTag { Bull, Bear, Chop }`, `REGIME_THRESHOLD_RATIO =
  dec!(0.02)`, `classify_regime` fn) which 7+ downstream test files
  + lesson-card embedding + Phase F Memory/Models renderer depend on
  byte-identically; the feature extends rather than reinvents.
  Q1-Q7 surfaced: regime taxonomy, classifier architecture (analyst
  default HMM with rule-based fallback), nowcast vs forecast horizon,
  strategy consumer shape (default regime-conditional position
  sizing on v1 momentum), verdict shape (analyst proposes new
  sibling ADR-0037 NOT ADR-0033 extension), anchor pin (default
  `v2.7.0-regime`), and in-place vs sibling-file vs new-crate
  disposition (default extend-in-place to preserve lesson-card
  embedding determinism). Cost ~4-6 weeks from activation gate;
  cumulative budget across {C1 + C2 + C5} cap ~16 weeks per
  Q-BUDGET. Sibling picks: `v3-volatility-forecast` (C1, ships
  first) + `v3-llm-as-forecaster` (C5, parallel analyst pass).

- **v3 — LLM-as-forecaster (`v3-llm-forecaster`).**
  _moved Queue → Active 2026-05-22 (analyst-bridge)_ — see
  [Active section](#active) for the live tracking comment block
  and [`feature.md`](v3-llm-forecaster/feature.md) for the full
  v0.1.0 brief (R1-R10, H1-H5, Q1-Q8 with analyst-recommended
  defaults, K1-K10 risk register, 8-item non-regression contract).
  Candidate 5 of three picks ({C1 volatility + C2 regime + C5
  LLM-as-forecaster}) from the
  [strategy-reformulation survey](dev-notes/strategy-reformulation-survey-2026-05-22.md#candidate-5--reflection-memory-as-forecaster-v2-llm-signal);
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
  [`spec/dev-notes/archive/2026-Q2/ui-testing-direction-2026-05-12.md`](dev-notes/archive/2026-Q2/ui-testing-direction-2026-05-12.md#section-9).
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
  [dev-note §6 week 2](dev-notes/archive/2026-Q2/ui-testing-direction-2026-05-12.md#6-phased-adoption--4-week-plan).
  Analyst spawn when v0.1 ships and H1 (tiny-skia byte determinism)
  is unfalsified.

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

- **Week-4 follow-up — GitHub Actions CI + presenter integration
  (`ui-test-harness-ci`).** _candidate, gated on
  `ui-test-harness-viewport-matrix` + `ui-test-harness-evaluator`
  ship_ — macOS runner workflow uploading baseline+actual+diff PNG
  triples on visual snapshot failures; presenter deck format gets a
  fixed "screenshot artifacts" section pointing at the CI artifact
  URL per [dev-note §6 week 4](dev-notes/archive/2026-Q2/ui-testing-direction-2026-05-12.md#6-phased-adoption--4-week-plan).
  **Per [`ui-testability-deep-dive-2026-05-15.md §5.3`](dev-notes/ui-testability-deep-dive-2026-05-15.md#53-keep--drop--replace-against-the-existing-weeks-2-4-plan)**
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
  LOCKED → defer indefinitely STILL APPLIES. Operator-acknowledged
  revisit trigger added: when iced 0.15.0 **stable** releases (not
  the current `0.15.0-dev` master pin), bump Q-014-PIN consideration
  + re-evaluate this candidate. Until then: no spawn trigger, no
  schedule. See
  [`iced-014-feature-analysis-2026-05-15.md §3`](dev-notes/archive/2026-Q2/iced-014-feature-analysis-2026-05-15.md#comet-debugger)
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

- **comet debugger evaluation (`ui-comet-eval`).** _candidate,
  deferred 2026-05-15 by
  [`iced-014-feature-analysis-2026-05-15.md §3`](dev-notes/archive/2026-Q2/iced-014-feature-analysis-2026-05-15.md#comet-debugger)
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

### 2026-05-28 cohort

- **lab-recipe-test-harness v0.1.0** — shipped 2026-05-28 (operator-
  approved). P1 tooling investment closing the channel/subscription
  test gap exposed by the Bug #64 D.1.1+D.2.1 revert (commit
  `05937e4`). Architect pattern (d) Combination: Surface 1 boundary-
  test for `spawn_lab_run` with `MockLabYahooBarSource` + Surface 2
  Stop-button gating state-machine test against `model.lab_run_inflight`.
  New `pub trait LabYahooBarSource` extraction in `crates/ui/src/lab/runner.rs:194-260`;
  `Box<dyn>` for ergonomic test construction; production path
  backwards-compatible via `None` injection. 6 new tests across 2 new
  files (3 in `spawn_lab_run_yahoo_harness.rs` + 3 in
  `lab_stop_button_gating.rs`). **T-T4 falsification CONFIRMED**:
  tester independently commented out `state.rs:2147` and verified 2
  Surface 2 tests fail at `lab_stop_button_gating.rs:133` + `:182`;
  restore verified 3/3 PASS. Zero anchor delta (channel-only events,
  no file output); 70/70 byte-identical. K5 regression intact (5/5).
  411 lib tests PASS; clippy 0 new (9 pre-existing). Unblocks AND
  gates the Bug #64 D.1.1+D.2.1 re-attempt. Future UI Recipe touches
  can opt into the same harness pattern. Commits: `a971008`, `648d470`,
  `dbe1609`, `aaa5bc9`. ADR-0048. See
  [`spec/lab-recipe-test-harness/feature.md`](lab-recipe-test-harness/feature.md).

- **lab-yahoo-realdata-v0.1.2-eth-usd-anchor-and-cache-badge v0.1.0** —
  shipped 2026-05-28 (operator-approved). Closes Q1 + Q3 of v0.1.1
  presenter deck's open list. Anchor count **69 → 70**:
  `eth-yahoo-2024-1d-sma-cross` locked under namespace
  `lab-yahoo-realdata-v0.1.2` (SHA `e59a5f87daf0cc58ce8be2e1695dfc2c
  cc3ab76bd976b54c957e9e3c5ed4199a`). `run_yahoo_sma.rs` extended with
  `--ticker <TICKER>` Clap arg (default BTC-USD; scales DRY across the
  remaining 8 crypto-mirror tickers; `ALLOWED_YAHOO_TICKERS` 10-row
  validation surface). NEW aggregate cache-state SUMMARY badge widget
  (`cache_state_summary_badge`) in a NEW Lab tab toolbar row (operator
  Q2 override) — "Yahoo cache: N tickers · last fetch YYYY-MM-DD".
  Cached on `LabState::cache_summary` with invalidation hooks in
  `LabSelectDataSource` + `LabRunCompleted` per ADR-0040 § Changelog
  D-V0.1.2-1. Two-lane parallel ship (M-DEV backtest + M-DEV-UI; zero
  file overlap). H1 PASS at 0.84% via K1 synthetic fallback (Yahoo
  ETH vs Yahoo BTC same-window); H2 ×5 determinism PASS. UI lib 411
  PASS (+14); panel snapshots 90 PASS (+4); cross-feature canary
  (cockpit_training_pressed_wiring) 5/5 PASS. **SOFT-PASS** qualifier:
  BTC body SHA drifted from `8045623b...` to `d2a709ef...` because
  `REVISION.toml` aggregate changed when ETH-USD was fetched
  (`rev=` line in report body). Verify_anchors.sh still resolves
  70/70 correctly per ADR-0038 § D6 byte-immutability via on-disk
  file. Pattern flagged for v0.1.3 architect attention. Commits:
  `cf7015c`, `d4c4c45`, `bd7e04b`, `9638ff8`, `1fd72b7`. ADR-0040
  Changelog. See
  [`spec/lab-yahoo-realdata-v0.1.2-eth-usd-anchor-and-cache-badge/feature.md`](lab-yahoo-realdata-v0.1.2-eth-usd-anchor-and-cache-badge/feature.md).

- **cockpit-toast-queue v0.2.0 cleanup v0.1.0** — shipped 2026-05-28
  (operator-approved). Closes the v0.1.0 ship's architecture-deviation
  footnote: retires the `pub toast_message: Option<SmolStr>` FIELD,
  the `toast_message()` METHOD shim, and the 2-line stale comment in
  `cockpit_live.rs:1181-1182` — all eliminated. Post-cleanup
  `grep -rn "toast_message" crates/` → **0 matches** anywhere.
  Sub-route (b) FULL REMOVAL chosen (analyst recommendation aligned;
  audit confirmed only test code referenced the method shim). 2 test
  field-WRITE sites migrated to `Message::ShowToastWithSeverity` /
  `Message::ShowToast` dispatch (mirrors production `cockpit_live.rs`
  pattern); 5 field-READ assertions flipped to direct
  `toast_queue.front()` access. K5 regression 5/5 PASS; v0.1.0
  integration 4/4 PASS; 397 ui lib tests PASS; 69/69 anchors
  byte-identical (UI-only). ADR-0046 § T-AR-5 one-cycle migration
  commitment honored. Commits: `8ebc12a`, `2dcb112`, `8c074bd`. See
  [`spec/cockpit-toast-queue-v0.2.0-cleanup/feature.md`](cockpit-toast-queue-v0.2.0-cleanup/feature.md).

### 2026-05-27 cohort

- **v5-latency-slippage-sim-v0.3.0-full-path-wiring v0.1.0** —
  shipped 2026-05-27 (operator-approved). Closes v0.2.0's accepted
  scope gap: friction-real anchored scenarios **2 → 11** (was 2 of 34
  at v0.2.0; momentum-only). `LatencySlippageSimConfig` now plumbs
  through 6 strategy paths via new `crates/backtest/src/scenarios/sim.rs`
  shared helper (lifted from momentum.rs as anchor-additive per
  ADR-0038 § D6.a). New `--force-synthetic-bars` CLI flag (~5 LoC)
  honours operator Q1=(a) revert-to-synthetic for Group A SMA/Composed
  re-emission — preserves friction-free oracle for all 69 anchors;
  v0.2.0's Group A canonical SHAs become stranded artifacts. t1937
  test refactored to namespace-aware resolver (Namespace::Noop /
  Namespace::Canonical) — mirrors `verify_anchors.sh` v0.2.0 pattern;
  future-proof against subsequent canonical re-emissions. 11
  canonical reports re-emitted; 9 SHAs overwritten in `spec/anchors.toml`
  in-place under same canonical namespace pin `v5-realdata-medium-2026-05`
  (Q3=(a)). **0 K1 surprises** across all 11 re-emitted scenarios.
  Cross-feature e2e tests all PASS (latency_slippage_sim_e2e 3/3,
  vol_targeting 1/1, vol_killswitch 4/4). **SOFT-PASS qualifier**:
  8 candle/realdata-feature-gated scenarios deferred to v0.4.0
  (plumbing wired; feature-flagged rebuild needed): TCN-weights × 2,
  TCN-realdata × 4, PatchTST × 1, VolTarget-GARCH × 1. Commits:
  `1267d39`, `4fd1095`, `275a6d0`, `21bda41`, `61db5f9`, `fe6b14a`.
  ADR-0047. See
  [`spec/v5-latency-slippage-sim-v0.3.0-full-path-wiring/feature.md`](v5-latency-slippage-sim-v0.3.0-full-path-wiring/feature.md).

- **vol-killswitch-overlay-noop-fix v0.1.0 (Bug #65 — P0 safety
  wiring-bug recovery)** — fix landed 2026-05-26 in tree; **spec
  retroactively closed 2026-05-27** by orchestrator during the
  audit-2026-05-27 P0 triage after the auditor flagged the
  paperwork drift (feature.md was `status: draft`, trace.toml was
  `state: proposed`, but bug-log #65 records FIXED 2026-05-26 with
  Q4=(p3) "Both" — fix test fixture AND broaden overlay filter).
  Verified at retroactive close: `cargo test -p strategy --test
  vol_killswitch_overlay_end_to_end` → 4/4 PASS. No formal
  test-final/<DATE>.md was authored at original ship; bug-log #65
  entry is the authoritative shipping record (precedent for
  Bug-fix briefs whose scope makes a full test-final overkill).
  **Trace**: `REQ-VOL-KILLSWITCH-NOOP-FIX-001` flipped
  `proposed → passed`. **Bug log**: [`spec/bug-log.md` § #65](bug-log.md).

- **cockpit-toast-queue v0.1.0** — shipped 2026-05-27 (operator-approved).
  Replaces the cockpit's single-slot toast REPLACE semantic with a
  bounded multi-toast queue. `VecDeque<ToastEntry>` capped at 5 with
  drop-oldest FIFO; stacked Lumen-card overlay in the bottom-right via
  `iced::widget::Stack`; 5 s auto-dismiss via shared 500 ms
  `ToastDismissRecipe` (6th cockpit subscription) + per-card `×` button.
  Severity tokens reuse existing Lumen palette (`FG_2 / UP_500 /
  INFO_400 / DOWN_500`) — zero new design tokens. K5 back-compat shim
  keeps `cockpit_training_pressed_wiring` regression 5/5 green.
  Architecture deviation flagged: `pub toast_message: Option<SmolStr>`
  FIELD kept alongside queue + method shim (dead-store relative to
  queue; annotated `// MIGRATION: remove at v0.2.0`). 4 integration +
  4 unit + 86 panel-snapshot tests PASS; 69/69 anchors byte-identical
  (UI-only). Operator-side smoke tests T-D-N16/T-D-N17 deferred per
  AGENT.md human-verification recipe contract. Commits: `8480ded`,
  `9cf813a`, `a723d24`, `896baab`. ADR-0046. See
  [`spec/cockpit-toast-queue/feature.md`](cockpit-toast-queue/feature.md).

- **lab-yahoo-realdata v0.1.1 (live-cache + Yahoo anchor lock)** —
  shipped 2026-05-27 (operator-approved follow-up to v0.1.0). First
  Yahoo Finance anchor locked: BTC-USD 2024 1d SMA cross.
  Operator-populated cache at `data/yahoo/BTC-USD/1d/2024/` (12
  parquets, 366 bars, REVISION.toml SHA `7b33166e1eb8...`). New
  `crates/backtest/src/bin/run_yahoo_sma.rs` binary (247 LoC, gated by
  `yahoo` feature). Anchor count 68 → 69; new row
  `btc-yahoo-2024-1d-sma-cross` under namespace
  `lab-yahoo-realdata-v0.1.1`. **H1 PASS** at 9.03% Yahoo-vs-Binance
  equity divergence (well below 30% threshold). **H2 PASS** at 100%
  fetch success (trivially satisfied at scale=1). Determinism
  confirmed via 2 independent re-runs of the new binary. Tester
  formal FAIL on workspace fmt + gallery test was *external* — both
  blockers attributable to in-flight cockpit-toast-queue dev; resolved
  by toast-queue landing. v0.1.2 follow-on: T-D2 cache-state badge UI,
  multi-ticker fetch, T-T5 cockpit-smoke, T-T8 idle-CPU. Commits:
  `bb14e11`, `8bd6b5c`, `9cf813a`, `a723d24`. See
  [`spec/lab-yahoo-realdata/feature.md`](lab-yahoo-realdata/feature.md).

- **cockpit-activity-audit-ledger-producer v0.1.0** — shipped 2026-05-27
  (operator-approved). Closes the activity-tape producer trio (LLM + Training
  + audit-ledger). New `crates/agent/src/activity_audit_aggregator.rs` (~210
  LoC) subscribes to existing `crates/audit/src/tick.rs` `AuditTick<AuditEvent>`
  broadcast — ZERO changes to `crates/audit/`. 100 ms time-window envelope
  (Q1=(b)); PII-redacted `"Audit: N writes"` label (Q2=(a)); separate-handle
  Failed-event emission (Q3=(a)). Long-lived `ActivityHandle` with idle-end
  semantics. Criterion benches: counter increment 1.797 ns / fan-out 46.81 ns
  / idle-end 131.98 ns / K3-discharge anchor-replay parity 0.12 % < 1 % budget.
  6/6 audit-ledger tests + 2/2 UI tests PASS (1 ignored by K5 design).
  Anchor-additivity preserved by construction (zero crates/backtest|strategy|
  audit/journal|exec|cost changes). M-FINAL FAIL on 3 housekeeping issues
  (fmt + 1 clippy + 1 frontmatter status) inline-fixed by orchestrator at
  commit 6b494aa; tester K3-collision-note in addendum is informational
  (resolves at v5 v0.2.0 Wave B namespace-aware verify_anchors). Commits:
  `8b67669`, `6b494aa`. ADR-0044. See
  [`spec/cockpit-activity-audit-ledger-producer/feature.md`](cockpit-activity-audit-ledger-producer/feature.md).

- **v5-latency-slippage-sim-v0.2.0-anchor-migration v0.1.0** — shipped
  2026-05-27 (operator-approved Ship Route (a) — partial migration accepted).
  Anchor count doubled 34→68: 34 noop-baseline rows preserve original SHAs
  as friction-free oracle; 34 canonical rows under namespace pin
  `v5-realdata-medium-2026-05` carry re-emitted SHAs under
  `LatencySlippageSimConfig { latency_ms_min: 30, latency_ms_max: 80,
  slippage_bps: 8 }`. `scripts/verify_anchors.sh` extended namespace-aware
  (T-AR-3 step 5 escape hatch invoked). Sharpe-delta table at
  [`spec/v5-latency-slippage-sim-v0.2.0-anchor-migration/reports/sharpe-delta-table-2026-05-27.md`](v5-latency-slippage-sim-v0.2.0-anchor-migration/reports/sharpe-delta-table-2026-05-27.md)
  documents 8 scenario groups: only Group B (top-10 momentum, 2/34 scenarios)
  received real v5-sim migration (Δequity -$3.5k to -$5.4k); Group A
  (5x SMA/Composed) Δequity +$48k to +$83k driven by synthetic→real-Binance
  data-source auto-switch, NOT v5 sim; Groups C-F (12 scenarios — Pairs /
  TCN / PatchTST / VolTarget) canonical = noop SHA byte-identical (sim
  not wired into those construction sites); Groups G-H (15 analysis /
  success reports) no equity metrics. **0 K1 surprises** across all 34
  scenarios. Cross-feature e2e tests 8/8 PASS (latency_slippage_sim_e2e
  3/3 + vol_targeting 1/1 + vol_killswitch 4/4). Operator-accepted scope
  gap → v0.3.0 Queue row covers (a) wire LatencySlippageSimConfig into the
  6 remaining strategy paths; (b) operator-decide for Group A re-anchor
  (revert to synthetic OR accept real-Binance baseline); (c) refresh
  `t1937_nine_strategy_anchors_unchanged` test resolver. Commits:
  `d2cc343`, `c223d11`, `4dfa2d8`, `d191227`. ADR-0045. See
  [`spec/v5-latency-slippage-sim-v0.2.0-anchor-migration/feature.md`](v5-latency-slippage-sim-v0.2.0-anchor-migration/feature.md).

- **v5-latency-slippage-sim v0.1.0** — shipped 2026-05-27 (operator-approved
  triple-batch). Deterministic latency + slippage simulator in `crates/exec`
  + `crates/cost`. Default-zero noop preserves all 34 anchors byte-identically;
  CLAUDE.md non-negotiable overlay-e2e divergence test (R5) shipped from day 1
  (3/3 PASS). Murmur3-style finalizer keyed on `(scenario_seed, order_id)`
  (D2 deviation accepted via ADR-0043 Changelog amendment — ChaCha20Rng
  replaced for hot-path perf). Criterion baselines: `apply_latency_noop`
  2.35 ns, `apply_latency_jitter` 2.50 ns, `apply_slippage_10bps` 22.7 ns,
  `noop_8760_fills` 73.9 µs, `enabled_8760_fills` 171.6 µs. New
  `AuditEvent::SimulatedExecMetrics` variant with skip-when-zero guard.
  v0.2.0 anchor-migration brief at
  [`spec/v5-latency-slippage-sim-v0.2.0-anchor-migration/`](v5-latency-slippage-sim-v0.2.0-anchor-migration/feature.md)
  picks the canonical non-zero friction profile (operator-decide Q1-Q4
  pending). Commits: `a5f8647`, `c46fd45`. ADR-0043. See
  [`spec/v5-latency-slippage-sim/feature.md`](v5-latency-slippage-sim/feature.md).

- **cockpit-activity-llm-producer v0.1.0** — shipped 2026-05-27 (operator-
  approved triple-batch). v0.1.1 follow-on of `cockpit-activity-status-bar
  v0.1.0` closing the parent's Q8 forward-list. `ActivityHandle` wired
  around `crates/trader/src/llm_forecaster/anthropic_impl.rs:412-516`
  via new `with_activity_sender()` builder setter. PII-redacted label
  `"LLM call: <model_id>"` enforced at producer boundary (no prompt /
  completion leakage). RAII handle ensures Completed/Failed on drop;
  failure mapping inherits parent R2.5 red 3 s hold. 159 trader-crate
  tests green; 34/34 anchors byte-identical (anchored bins never
  construct an EventBus). Open H3 (per-variant failure reason chip)
  deferred to v0.1.2. Commit: `c46fd45`. See
  [`spec/cockpit-activity-llm-producer/feature.md`](cockpit-activity-llm-producer/feature.md).

- **cockpit-training-pressed-wiring v0.1.0** — shipped 2026-05-27
  (operator-approved triple-batch). v0.1.1 follow-on of
  `cockpit-activity-status-bar v0.1.0` closing the Wave C T-D-N9
  ship-time open question — the Train button now actually trains.
  Binds `Message::TrainingPressed` in
  `crates/ui/src/bin/cockpit_live.rs::AppState::update` to call
  `lab::trainer::spawn_training_run` with default config
  `crates/forecast/train_tcn.toml` (Q1=(a)). New
  `crates/ui/src/lab/training_log.rs` (183 LoC) `TrainingLogRecipe`
  bridges std-mpsc training logs into the tokio runtime via
  spawn_blocking, surfacing per-epoch progress in the activity tape.
  Double-press inert per parent R3.4 (Q2=(a)). 34/34 anchors
  byte-identical; 5/5 integration tests · 0.31 s. K5 multi-toast
  follow-on opened as `cockpit-toast-queue v0.1.0` (Active). Commits:
  `28db398`, `c46fd45`. See
  [`spec/cockpit-training-pressed-wiring/feature.md`](cockpit-training-pressed-wiring/feature.md).

### 2026-05-24 cohort

- **lab-yahoo-realdata v0.1.0** — shipped 2026-05-24 (operator-approved).
  Yahoo Finance pivot for the Lab UI: 10-ticker crypto-mirror universe
  (`BTCUSDT` … `LINKUSDT`), Binance-style symbols converted to Yahoo
  (`BTC-USD` …) at the dispatch boundary (Q6=(a) operator-override);
  adaptive cadence (1m ≤7d, 1h 7-60d, 1d >60d, Q4=(c)); parquet cache
  pattern + revision-pin mirroring the Binance precedent. New widgets:
  Source toggle (Synthetic / YahooCache), cadence badge. New crate path:
  `crates/data/src/yahoo.rs` + `fetch_yahoo_klines` CLI. New Venue::Yahoo.
  Anchor-additive contract preserved per ADR-0038 § D6.b — all 34
  anchors byte-identical (ScenarioConfig extensions use
  `#[serde(default, skip_serializing_if)]`). Tester Wave E PASS: 878+
  tests, T-C3.7 7/7 (yahoo-gated), clippy clean on touched crates,
  spec-lint baseline-stable. ADR-0040 codifies Yahoo realdata path +
  revision pin (`yahoo_finance_api = "=4.1.0"`). H1/H2 + cockpit-smoke
  + idle-CPU deferred to v0.1.1 per R6.3. Commits: `7ab924e`,
  `04e059f`, `a87bbc4`, `899c2a0`. See
  [`spec/lab-yahoo-realdata/feature.md`](lab-yahoo-realdata/feature.md).

### 2026-05-22 cohort

> 4 ships that day: 1 partial + 2 retire-with-evidence + 1 P0 wiring-fix.
> The Active blocks above (lines 376-742) carry full details; this section
> is the chronological pointer for future audits.

- **v3-llm-forecaster v0.1.0-PARTIAL** — shipped 2026-05-22 (operator-approved).
  First-of-kind `shipped-partial` precedent (code gates clean; Wave D deferred
  to v0.1.1 pending ANTHROPIC_API_KEY). 6 waves clean (A+B+C+E+F+G); 34/34
  anchors byte-identical; R9.3 byte-identity proven via SHA-256 match. ADR-0039
  LLM-forecaster verdict criteria L0-L4 codified. Wave D paused indefinitely
  per operator routing pick 2026-05-22. See `spec/v3-llm-forecaster/feature.md`.

- **v3-volatility-forecaster-noop-fix v0.1.0 (P0)** — shipped 2026-05-22.
  P0 wiring-bug fix: GARCH vol-target overlay was a no-op
  (`scale` computed but never applied to fill quantities). Discovery via
  orchestrator caveman probe (σ_hat × 2.95 → byte-identical equity → code review).
  Fix: `Strategy::quantity_scale` defaulted trait method; sizing hook at Buy arm;
  scale_cache + R2 forensic-gate test. ADR-0038 § D6.b anchor re-emission protocol
  amendment. 3 anchors re-emitted in-place (top10-2023-fy-vol-target-overlay-realdata
  + 2 sharpe-comparisons); vol-verdict-bs1-realdata stayed byte-identical.
  Post-fix equity: $113,479.98 → $62,807.89 (overlay actively destroys equity
  via GARCH-under-prediction × upper-clamp saturation; NEGATIVE-NET-DELTA confirmed).
  See `spec/v3-volatility-forecaster-noop-fix/feature.md`.

- **v3-volatility-forecaster v0.1.0** — shipped 2026-05-22; RETIRED same day.
  Hand-rolled GARCH(1,1) MLE + Parkinson estimator + V-verdict + 3 strategy
  builders + backtest scenario. Joint MODEL-BROKEN / NO-ALPHA / NEGATIVE-NET-DELTA
  verdict under real-wired overlay (post-noop-fix); operator routing R-O1 = (a)
  RETIRE C1. Code stays in tree; anchors locked. ADR-0038 V-verdict shape
  (now historical). See `spec/v3-volatility-forecaster/feature.md`.

- **v3-volatility-forecaster-rebaseline v0.1.0** — shipped 2026-05-22; RETIRED
  same day (with parent). Re-baseline pass per operator (b) routing pick
  from parent deck: new `top10-2023-fy-momentum-realdata` scenario + 1 anchor
  re-emitted. Architect locked NEW `ScenarioFamily::VolTargetRebaseline` to
  preserve parent anchor immutability (ADR-0038 § D6 contract held).
  Confirmed NO-ALPHA on real-vs-real comparison BEFORE the noop-fix discovery
  (the rebaseline verdict was correct conclusion, fortuitously — the noop-fix
  caveman probe later revealed the underlying bug). See
  `spec/v3-volatility-forecaster-rebaseline/feature.md`.

- **v2.5a — PatchTST forecast overlay (`v25a-patchtst-overlay` v0.1.0)** —
  shipped 2026-05-22 (operator-approved via presenter deck
  [`presentations/v25a-patchtst-overlay-2026-05-22.md`](v25a-patchtst-overlay/presentations/v25a-patchtst-overlay-2026-05-22.md);
  Q1-Q8 = analyst defaults via "Autoapprove all"; tester VERDICT →
  PASS after one-line K4 test-harness fix). Phase 2 of the
  [4-phase DL roadmap](v25-dl-forecast-overlay/feature.md).
  Predecessor: [`v25-tcn-horizon-bump-or-retire v0.1.0`](v25-tcn-horizon-bump-or-retire/feature.md).
  Parent: `v25-dl-forecast-overlay v0.0.0` (now → terminal-retired per
  routing (a); see follow-on commit). **Substantive finding: F-verdict
  F4 with Sharpe-delta only +0.006144 vs v1 momentum baseline** — well
  below the +0.10 T-ALPHA-UNLOCKED threshold AND LOWER than retired
  v2.5 TCN (BS-1 @ 1h: +0.018; BS-2 @ 1h: +0.045). **Joint F4-F4-F4
  verdict across 3 model checkpoints / 2 model families (convolutional
  TCN + patch-attention PatchTST) / 2 horizons (1h + 24h)** establishes
  high-confidence retirement of the entire 4-phase DL forecast overlay
  roadmap (operator-decided routing (a) at presenter approval).
  Lands NEW [`crates/forecast/src/patchtst.rs`](../crates/forecast/src/patchtst.rs)
  (PatchTST model in candle; d_model=128, n_heads=4, n_layers=3,
  d_ff=256, patch_len=16, stride=8, context_len=336, dropout=0.2;
  ~431k params) + NEW
  [`crates/forecast/src/bin/train_patchtst.rs`](../crates/forecast/src/bin/train_patchtst.rs)
  (training scaffold with ADR-0035 § D1 post-training σ_train pattern
  from the start — NOT the deprecated in-loop accumulator) + 4
  unit tests (forward_determinism / sigma_train_not_in_safetensors /
  tcn_byte_identity / patchtst_overlay_neutrality K4 anchor-
  neutrality test) + NEW `crates/strategy/src/patchtst_sync.rs` +
  NEW `crates/strategy/src/patchtst_overlay_momentum.rs` (sibling
  strategy mirror of `tcn_overlay_momentum.rs`) + NEW
  `crates/backtest/src/scenarios/patchtst_overlay_weights.rs`
  (sibling backtest scenario) + additive enum variants in
  `forecast_distribution.rs` + `sharpe_comparison.rs` + `backtest`
  Scenario enum. **2 new anchors locked under version
  `v2.5a.0-patchtst`** (30 total; 28 originals byte-identical):
  `forecast-distribution-patchtst-bs1-realdata` SHA `c55c6c51…` +
  `top10-2023-fy-patchtst-overlay-realdata` SHA `5f303cc0…`.
  Training stats: 30 epochs / 7h 45min wall-clock on Apple Silicon
  Metal / final train_loss 2.6e-5 (67× from epoch 1) / σ_train
  derived post-training 0.007053 (well-calibrated; in expected
  0.005-0.025 range). Checkpoint SHA `62520db9…` at
  `crates/forecast/checkpoints/anchors/patchtst-bs1-62520db9….{safetensors,metadata.json}`.
  [ADR-0036](architecture/adr/0036-patchtst-training-contract.md)
  codifies the PatchTST training contract.
  **Hypothesis status:** H1 (24h horizon unlocks signal where 1h
  failed) **FALSIFIED** — PatchTST @ 24h scored LOWER than TCN @ 1h
  on Sharpe-delta. H2 (attention captures session structure) =
  INCONCLUSIVE; F4 stays. H3 (4-6 week scope feasible) = CONFIRMED
  (actual <1 day end-to-end). H4 (σ_train post-training pattern
  works) = CONFIRMED. **Strategic implication**: v2.5-era DL
  approaches exhausted; pivot research budget per routing (a).
  Anchor risk zero — 28 originals + TCN checkpoint files byte-
  identical (verified via K4 neutrality test PASS on TCN scenario
  body SHA `8fa47f49…`); cargo fmt + workspace clippy +
  `--features candle` clippy + `--features candle,realdata`
  clippy + `--features forecast,forecast-audit-tick` clippy all
  clean; spec-lint 86/3 = baseline (0 new regressions); 2-run
  determinism PASS on all 3 substantive reports
  (forecast_distribution + backtest + sharpe_comparison).

- **v2.5 TCN horizon-bump or retire (`v25-tcn-horizon-bump-or-retire` v0.1.0)** —
  shipped 2026-05-21 as a **policy/decision feature** (no code change,
  no new anchors). Operator-decided Q1=(b) at the hard-blocker scope
  prompt: **retire v2.5 TCN at 1h horizon; pivot the multi-week budget
  to v2.5a PatchTST** (phase 2 of the 4-phase DL roadmap). Q2-Q7 MOOT
  under (b) — no retrain, no checkpoint, no new training anchor. The
  v2.5 TCN journey across 3 substantive ships
  ([alpha-investigation v0.1.0](v25-tcn-alpha-investigation/feature.md) F4 verdict +
  [recalibrate v0.1.0](v25-tcn-recalibrate/feature.md) σ_train bug eliminated +
  [threshold-tuning v0.1.0](v25-tcn-threshold-tuning/feature.md) Joint T-MARGINAL
  +0.018 / +0.045) established that 1h-horizon TCN cannot extract alpha
  on real Binance OHLCV. **Decision rationale**: marginal +0.018 / +0.045
  Sharpe-delta is below the +0.10 alpha-unlock threshold AND a noise-floor
  question; ~4-6 weeks of PatchTST investigation is higher EV than ~2-3
  more weeks chasing a 24h-horizon TCN retrain when we already have
  evidence the model family struggles on hourly crypto bars. **What
  stays**: 28 existing anchors byte-identical (8 v2.5 TCN anchors +
  4 backtest-realdata + 4 v2.6.1-alpha-investigation-recalibrated +
  2 v2.6.2-threshold-tuning + 10 non-TCN); additive
  `with_tcn_bs{1,2}_ledger_tuned` builders shipped at threshold-tuning;
  ADR-0033 § D3 F-verdict + ADR-0035 σ_train recalibration contract
  remain in force as cross-phase invariants. **What promotes**:
  [`v25a-patchtst-overlay`](#queue) flagged ACTIVATION TRIGGERED in
  Queue § Strategy — promotes Queue → Active on next "next" directive.
  Trace row `REQ-V25-TCN-HORIZON-BUMP-OR-RETIRE-001` flipped
  draft → shipped (operator-decide as the load-bearing M-FINAL).

- **v2.5 TCN threshold tuning (`v25-tcn-threshold-tuning` v0.1.0)** —
  shipped 2026-05-21 (operator-approved via presenter deck
  [`presentations/v25-tcn-threshold-tuning-2026-05-21.md`](v25-tcn-threshold-tuning/presentations/v25-tcn-threshold-tuning-2026-05-21.md);
  Q1-Q6 = analyst defaults via "Autoapprove all"; tester VERDICT →
  PASS clean — all 9 T-F + 6 T-T gates green). Predecessor:
  [`v25-tcn-recalibrate v0.1.0`](v25-tcn-recalibrate/feature.md).
  Parent (stays `in-progress`): `v25-tcn-overlay v2.5.0`. **Cheap τ × ε
  sweep follow-on** to the recalibrate ship — ran 90 backtests (9 τ ×
  5 ε × 2 checkpoints) over the recalibrated TCN checkpoints on real
  Binance OHLCV. **Substantive finding: Joint T-MARGINAL + T-MARGINAL**
  — headline cell on BOTH checkpoints is τ=0.1 / ε=0.001 with BS-1
  Sharpe-delta = **+0.018** and BS-2 = **+0.045**; both below the
  +0.10 T-ALPHA-UNLOCKED threshold. **No (τ, ε) tuple unlocks alpha.**
  F-verdict stays F4 per immutable
  [ADR-0033 § D3](architecture/adr/0033-tcn-alpha-investigation-report-shape.md);
  σ_train recalibration was necessary but not sufficient. Lands NEW
  [`crates/backtest/src/bin/threshold_sweep.rs`](../crates/backtest/src/bin/threshold_sweep.rs)
  (4-way `rayon::par_iter`, `(τ, ε)`-sorted assembly for byte-
  deterministic output) + NEW
  [`crates/backtest/src/scenarios/threshold_sweep.rs`](../crates/backtest/src/scenarios/threshold_sweep.rs)
  per-cell helper + **4 additive `_tuned` builders** on
  [`tcn_overlay_momentum.rs`](../crates/strategy/src/tcn_overlay_momentum.rs)
  (`with_tcn_bs{1,2}_tuned` + `with_tcn_bs{1,2}_ledger_tuned`; explicit
  args required; `TcnSyncForecaster::with_direction_epsilon` builder +
  `direction_epsilon: Option<f32>` field with const-fold-default fallback
  so existing `_ledger` builders stay **literal `dec!(0.6)` + literal
  `forecast::tcn::DIRECTION_EPSILON`** — 26 predecessor anchors stay
  byte-identical, R-3 const-fold-default contract preserved). 2 new
  anchors locked under version `v2.6.2-threshold-tuning`:
  `threshold-sweep-bs1-realdata-recalibrated` (SHA `551cc2ab…`) +
  `threshold-sweep-bs2-realdata-recalibrated` (SHA `755bc380…`). T-
  classifier (T-ALPHA-UNLOCKED ≥+0.10 / T-MARGINAL [0, +0.10) /
  T-NO-ALPHA <0) embedded in report body per Q4=(c); ADR-0036 NOT
  written (deferred until empirical alpha-unlock evidence justifies
  codification). **Operator decided routing (c)** at presenter approval
  — ship advisory (additive `_tuned` builders + 2 new anchors) AND
  queue [`v25-tcn-horizon-bump-or-retire`](#queue) (currently
  ACTIVATION-TRIGGERED in Queue § Strategy; promotes on next "next"
  directive). H1 FALSIFIED (no tuple unlocked alpha); H2 confirmed
  (heatmap smoothness statistic in body); H3 confirmed (cheap sweep
  delivered clear verdict in hours). Anchor risk zero — 26 originals
  byte-identical; 28 total; cargo fmt + workspace clippy +
  `--features candle,realdata,forecast,forecast-audit-tick` clippy
  all clean; spec-lint 87/2 = baseline; 2-run determinism gate PASS;
  `git diff` over `crates/forecast/checkpoints/anchors/*.metadata*.json
  + *.safetensors` empty (ADR-0035 D4 invariant).

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
  [`iced-014-feature-analysis-2026-05-15.md §4`](dev-notes/archive/2026-Q2/iced-014-feature-analysis-2026-05-15.md#headless-mode)
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
  D4 in [`spec/dev-notes/archive/2026-Q2/ui-testing-direction-2026-05-12.md ## Section 9`](dev-notes/archive/2026-Q2/ui-testing-direction-2026-05-12.md#9-open-decisions-for-the-operator)
  — the first `iced_test::Simulator::snapshot().matches_image()`
  chart-hover test in that feature replaces the manual capture.
  Q4 local-time x-axis labels DEFERRED to v1.11
  `chart-x-axis-local-time` (shipped 2026-05-20, see Recent;
  UTC fallback shipped in v1.10.0 was the bridge). Retrospective surfaced the architect's
  "iced 0.14 canvas-scale bug" misdiagnosis (empirically
  disproved by orchestrator's red-rect + cyan-dot probe; T3002 /
  T3003 / T3007 / T3008 closed as no-op) — produced the
  `AGENT.md ## Capability boundaries` amendment (D5,
  load-bearing) + [`spec/dev-notes/archive/2026-Q2/ui-testing-direction-2026-05-12.md`](dev-notes/archive/2026-Q2/ui-testing-direction-2026-05-12.md)
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
  [`spec/dev-notes/ui-testability-deep-dive-2026-05-15.md`](dev-notes/ui-testability-deep-dive-2026-05-15.md)
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
  [`spec/dev-notes/archive/2026-Q2/ui-testing-direction-2026-05-12.md`](dev-notes/archive/2026-Q2/ui-testing-direction-2026-05-12.md)
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
