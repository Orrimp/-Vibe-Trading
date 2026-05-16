---
slug: ui-gallery-table-cell
status: draft
owner: analyst
updated: 2026-05-16
---

# Tasks — `ui-gallery-table-cell` v0.1

> **Status as of 2026-05-16:** analyst initial draft (Wave 2a
> spec-hygiene). Architect Design pass owes
> Q-FIX-STRATEGY / Q-RENDER-SHAPE / Q-CELL-HEIGHT resolutions
> ([`feature.md ## Open questions for architect`](feature.md#open-questions-for-architect))
> plus an H-TC-1 / H-TC-2 falsifier spike before T1 lands. Tasks
> below are seeded from the
> [`ui-gallery-bin/tasks.md`](../ui-gallery-bin/tasks.md) V5+
> migration; re-keyed `T1`..`T9` to the new slug, with verbatim
> acceptance lines preserved where applicable.

## M0 — Architect design + falsifier spike (architect-owned)

- [ ] **T1** *(architect)* — Resolve **Q-FIX-STRATEGY**
  (one of: special-case strategies cell wrapper; swap
  `widget::table::Table` for non-table render in gallery only;
  fix table-cell bounds upstream). Cite resolution at
  [`feature.md ## Open questions for architect`](feature.md#open-questions-for-architect).
  _Acceptance: a chosen path is committed to design.md; if (c)
  upstream is chosen, the
  [`ui-iced-table-panic-upstream`](../backlog.md#queue) candidate
  is promoted to Active by the orchestrator before T2 lands._
- [ ] **T2** *(architect)* — Run **H-TC-1 falsifier**: minimal
  iced 0.14 program wrapping `widget::table::Table` in
  `Container::height(Length::Fixed(N))` for N ∈ {260, 360, 500,
  720}; report first non-panicking N (if any).
  _Acceptance: a one-row finding in
  [`feature.md ## Hypothesis register`](feature.md#hypothesis-register)
  marking H-TC-1 RESOLVED-UNFALSIFIED or FALSIFIED._
- [ ] **T3** *(architect)* — Run **H-TC-2 falsifier**: render
  the strategies cell with `fake_cockpit_v1_steady_state()`
  (loaded rows) vs `fake_cockpit_loading()` (empty rows); report
  which branch panics.
  _Acceptance: same shape as T2._

## M1 — Apply fix (developer-owned, gated on T1 resolution)

- [ ] **T4** *(developer)* — Implement the Q-FIX-STRATEGY path
  chosen in T1. Touch only `crates/ui/` (per R-TC-2 anchor-risk
  mitigation). Verbatim acceptance migrated from
  [`ui-gallery-bin/tasks.md T17`](../ui-gallery-bin/tasks.md#m4--exhaustiveness--snapshot-tests-075d):
  _V6 — `cargo test -p ui --features fixtures --test
  gallery_snapshots` exits 0; three baseline PNGs land under
  `tests/visual-baselines/`._
- [ ] **T5** *(developer)* — Drop `#[ignore]` from the three
  `gallery_snapshots` tests
  ([`crates/ui/tests/gallery_snapshots.rs`](../../crates/ui/tests/gallery_snapshots.rs)).
  Verbatim from
  [`ui-gallery-bin/tasks.md T18`](../ui-gallery-bin/tasks.md#m4--exhaustiveness--snapshot-tests-075d):
  _V10 — two hash runs match; `git status tests/visual-baselines/`
  shows zero modifications between runs._
- [ ] **T6** *(developer)* — Operator-slot PNG size check.
  Verbatim from
  [`ui-gallery-bin/tasks.md T19`](../ui-gallery-bin/tasks.md#m4--exhaustiveness--snapshot-tests-075d):
  _If > 10 MB, escalate to architect for the gallery-split
  design (six baselines instead of three). If ≤ 10 MB, proceed._

## M2 — Quality gates + workspace green

- [ ] **T7** *(developer)* — Anchors PASS gate. Verbatim from
  [`ui-gallery-bin/tasks.md T20`](../ui-gallery-bin/tasks.md#m4--exhaustiveness--snapshot-tests-075d):
  _V8 — `bash scripts/verify_anchors.sh` prints
  `ANCHORS PASS (11/11)`._
- [ ] **T8** *(developer)* — Workspace-test green gate. Verbatim
  from
  [`ui-gallery-bin/tasks.md T23`](../ui-gallery-bin/tasks.md#m5--readme--presenter-deck-artifact-05d):
  _V7 — `cargo test --workspace --features fixtures` zero
  failures and ≥ (prior pass count + V3 + V4 + V6 sub-tests)
  green._
- [ ] **T9** *(developer)* — `cargo fmt` + `cargo clippy
  --workspace --features fixtures -- -D warnings` clean.
  Verbatim from
  [`ui-gallery-bin/tasks.md T24`](../ui-gallery-bin/tasks.md#m5--readme--presenter-deck-artifact-05d).

## M_FINAL_TEST_RUN — test-runner pass

Test-runner spawn (per
[`AGENT.md ## Test-runner / evaluator split`](../../AGENT.md#test-runner--evaluator-split)).

- [ ] **T-FINAL-TEST-1** — V5 / V6 / V10 (snapshot tests +
  determinism). Migrated from
  [`ui-gallery-bin/tasks.md T-FINAL-TEST-1`](../ui-gallery-bin/tasks.md#m_final_test_run--test-runner-pass):
  two consecutive `cargo test ... --test gallery_snapshots`
  runs; SHA-compare baselines between runs.
- [ ] **T-FINAL-TEST-2** — V7 (workspace green) + V8 (anchors
  11/11). Migrated verbatim from
  [`ui-gallery-bin/tasks.md T-FINAL-TEST-4`](../ui-gallery-bin/tasks.md#m_final_test_run--test-runner-pass).
- [ ] **T-FINAL-TEST-3** — V9 (file-list gate). Migrated from
  [`ui-gallery-bin/tasks.md T-FINAL-TEST-5`](../ui-gallery-bin/tasks.md#m_final_test_run--test-runner-pass):
  `git diff --name-only` against the pre-feature commit; verify
  only in-scope paths changed.

## M_FINAL_EVAL — evaluator pass

- [ ] **T-FINAL-EVAL-1** — Read test-runner report; cross-check
  V5..V10 ticks against
  [`feature.md ## Requirements`](feature.md#requirements). Emit
  `VERDICT → PASS | FAIL | REGRESSION` at
  `spec/ui-gallery-table-cell/reports/evaluation-<ts>.md`.

## Notes

- **Migration is partial.** The predecessor
  [`ui-gallery-bin/tasks.md`](../ui-gallery-bin/tasks.md) had
  ~39 open boxes spanning M0..M5 + M_FINAL. The V1–V4 work
  (build / smoke / widget exhaustiveness / mod-rs parity) is
  **done green in v0.1-partial** and is NOT re-tasked here.
  The V5+ work — snapshot rendering, determinism, anchors,
  workspace green, FINAL passes — is the entire content above,
  re-keyed `T1..T9` + `T-FINAL-*`. The M0 design / falsifier
  block is re-spawned because the
  [`feature.md ## Open questions for architect`](feature.md#open-questions-for-architect)
  Q-FIX-STRATEGY decision is new for this brief and gates
  everything downstream.
- **Acceptance-line verbatim policy.** Where a task copies
  from the predecessor, the `_acceptance: ..._` line is
  preserved verbatim per the operator instruction; deltas
  (e.g., dropping `--ignore` flags) are scoped to file-list and
  invocation, not to V-item content.
- **Honest-tick discipline** (per
  [`AGENT.md ## Process discipline`](../../AGENT.md#process-discipline-lessons-from-v0--v15a)
  rule 1): developer MUST NOT tick `[x]` without citing
  (a) file:line of change, (b) test command, (c) test-output
  line — same convention inherited from the predecessor's
  tasks.md.
- **Effort budget** to be set by architect at design pass;
  analyst preliminary estimate is 0.5–1.5d depending on
  Q-FIX-STRATEGY (upstream-fix path is highest cost).

## Changelog

- 2026-05-16 (analyst, Wave 2a spec-hygiene): tasks file
  opened alongside
  [`feature.md`](feature.md) per
  [`spec/dev-notes/feature-triage-2026-05-16.md`](../dev-notes/feature-triage-2026-05-16.md)
  row A4. V5+ tasks migrated from
  [`spec/ui-gallery-bin/tasks.md`](../ui-gallery-bin/tasks.md)
  with re-keyed identifiers `T1..T9` + `T-FINAL-*` and verbatim
  acceptance lines. M0 architect block re-spawned for
  Q-FIX-STRATEGY. HANDOFF → architect.
