---
slug: cockpit-training-control
status: in-progress
owner: architect
updated: 2026-05-19
---

# Tasks — cockpit-training-control

> Milestone skeleton only. Per-task `T-D-N` decomposition deferred to
> the architect. Each milestone has an acceptance gate; the gates form
> the orchestrator's hand-off contract between architect → developer →
> tester.

## M0 — Architect synthesis

_owner: architect_

- [x] **T-OP-1..4** (2026-05-19) — Operator confirmed ALL FOUR analyst
  defaults via orchestrator AskUserQuestion:
  - **Q1** RESOLVED: new `training_events` table (additive migration
    `010_training_events.sql`).
  - **Q2** RESOLVED: SIGKILL-immediate on Cancel.
  - **Q3** RESOLVED: no panel hyperparam editing; defer to a follow-on
    `cockpit-training-hyperparams` feature if/when needed.
  - **Q4** RESOLVED: no auto-focus on orphan-detect; status-strip
    annotation only. Train panel stays closed on cold-start.
- [ ] Translate R1-R10 into a `T-D-N` task graph in this file (replace
  the M-T1 / M-T2 / M-FINAL stubs below with concrete `T-D-N` rows).
- [ ] Pick the `train_tcn` binary path-resolution strategy
  (`current_exe`-relative vs. `cargo run` fallback — see R2.2). Hard
  cap on architect time: one paragraph in `feature.md § Design`.
- [ ] Pick the K5 mitigation (R1.4 alternatives) — schema-print
  validation vs. golden-CLI CI diff. Document the choice in
  `feature.md § Design`.
- [ ] Decide whether to ship M-T1 + M-T2 in one branch or sequence
  M-T1 → land → M-T2. Analyst-recommend: **sequence** (M-T1 ships in
  ~2 days; the operator can validate the Tier 1 shape before paying
  for Tier 2's audit-schema cost).
- **Acceptance:** `feature.md § Design` populated; this file populated
  with `T-D-N` rows; trace.toml `REQ-COCKPIT-TRAIN-001.arch` array
  populated.

## M-T1 — Tier 1: launch button + log tail

_owner: developer (UI focus)_

Maps to **R1, R2, R3, R8.1, R9.1, R9.3, R9.4, R10.4**.

- [ ] **T-D-N₁** New module `crates/ui/src/lab/trainer.rs` —
  `spawn_training_run` + `TrainingHandle` + cancellation pair.
  Reuses the `RunCancelHandle` shape from `lab::runner` but spawns
  via `tokio::process::Command`. Includes `Drop` impl that calls
  `child.start_kill()`.
- [ ] **T-D-N₂** New widget
  `crates/ui/src/widgets/training_log.rs` — 200-line ring buffer +
  auto-scroll + "Jump to bottom" chip.
- [ ] **T-D-N₃** Lab screen integration in
  `crates/ui/src/screens/lab.rs` — collapsible panel below the
  volume histogram. Update `chart_canvas_height_for_body` to subtract
  the panel's expanded height.
- [ ] **T-D-N₄** Cockpit `Message` arms: `TrainingPressed`,
  `TrainingCancelPressed`, `TrainingLogLine(SmolStr)`,
  `TrainingExited(ExitStatus)`, `TrainingPanelToggled`.
- [ ] **T-D-N₅** Cold-start persistence: extend
  `LabStateJson` with `training_panel_collapsed: bool` (additive,
  `#[serde(default)]`).
- [ ] **T-D-N₆** insta snapshots
  `lab__training_panel_collapsed_default`,
  `lab__training_panel_expanded`,
  `training_log__ring_buffer_200_lines`.
- **Acceptance:** `cargo test -p ui` 100% PASS; manual cockpit run
  shows Train → log streams → Cancel kills the subprocess →
  panel state survives restart. Cockpit-smoke gate green.
  No anchor file touched (R10.4 + R10.5).

## M-T2 — Tier 2: audit events + live curves

_owner: developer (UI + audit + forecast focus). Architect may
declare this parallelizable into UI vs. backend lanes._

Maps to **R4, R5, R6, R7, R8.2, R8.3, R9.2, R9.5, R10.1-R10.3**.

- [ ] **T-D-N₇** Audit migration `010_training_events.sql` (additive
  `CREATE TABLE IF NOT EXISTS` per R4.2).
- [ ] **T-D-N₈** Audit writers in `crates/audit/src/journal.rs`:
  `post_training_start`, `post_training_epoch`,
  `post_training_finish`, `post_training_failed`.
- [ ] **T-D-N₉** Audit readers in `crates/audit/src/query.rs`:
  `recent_training_events`, `latest_training_run`. Plus the
  `TrainingEventRow` / `TrainingRunSummary` value types.
- [ ] **T-D-N₁₀** `train_tcn` instrumentation in
  `crates/forecast/src/bin/train_tcn.rs` — add `--audit-db <PATH>`
  flag (R5.1); emit start/epoch/finish/failed rows (R5.2). Default
  off so existing CI / manual / scripted runs stay byte-identical
  (R5.4 + R10.2).
- [ ] **T-D-N₁₁** Cockpit subscription
  `crates/ui/src/lab/training_subscription.rs` — 1 Hz audit-DB poller
  (R7.2). New `Message::TrainingEventsRefreshed`.
- [ ] **T-D-N₁₂** Training-curve plot — reuse `widgets::chart`
  overlay shape (R6.1-R6.3). New `TrainingChartFixture` data
  source feeding into the existing chart widget under the Train
  panel's plot region.
- [ ] **T-D-N₁₃** Status strip wiring (R3.5) — "Idle / Training
  epoch N/M, t=Ts / Failed: … / Done: <model_revision short>".
- [ ] **T-D-N₁₄** Orphan-detection on cockpit boot (R9.5): on Lab
  module init, query `latest_training_run`; if the latest row is
  `kind='start'` with no `finish` / `failed` within 24h,
  surface "Orphan run detected" in the status strip.
- [ ] **T-D-N₁₅** Unit + integration tests per acceptance gates of
  R4-R7 (`journal::tests::*`, `query::tests::*`,
  `training_subscription::tests::*`,
  `train_tcn_audit_emits.rs`,
  `train_tcn_no_audit_db_writes_nothing`).
- [ ] **T-D-N₁₆** insta snapshots
  `training_plot__two_lines_5_epochs`,
  `training_plot__empty_state`.
- **Acceptance:** all of M-T1 PASS + `cargo test -p audit -p forecast
  -p ui` 100% PASS + `scripts/verify_anchors.sh` 19/19 PASS (R10
  contract) + manual cockpit run shows live loss curves
  advancing during a fixture training run.

## M-FINAL — Tester sweep

_owner: tester_

- [ ] Run the full validate gate: `cargo fmt`, `cargo clippy -- -D warnings`,
  `cargo test --workspace`, `scripts/verify_anchors.sh`,
  `scripts/cockpit_smoke.sh`.
- [ ] Verify the 19 body-SHA-256 anchors are byte-identical to the
  pre-feature baseline (R10).
- [ ] Author `spec/cockpit-training-control/reports/test-final-<YYYY-MM-DD>.md`
  per the
  [test-report template](../../.claude/skills/rust-test/templates/test-report.md).
  Include: anchor diff, training-DB-roundtrip log, orphan-detection
  manual run notes.
- [ ] Open the trace.toml `tests` array against
  `REQ-COCKPIT-TRAIN-001` once tests are co-located.
- [ ] Verdict: PASS / FAIL / REGRESSION per the template.
- **Acceptance:** verdict PASS; report written; trace.toml updated;
  presenter spawn unblocked.

## Notes

- **No new anchors expected** — see `feature.md § R10.5`. If the
  tester finds a stable deterministic surface that warrants an anchor
  (unlikely given the wall-clock + UUID inputs), document it in the
  M-FINAL report.
- **Determinism contract for `train_tcn`** — the `<sha>.metadata.json`
  bytes stay byte-identical with `--audit-db` enabled vs. disabled
  (R5 emits to a sidecar table, not into the metadata). Architect
  must verify this assumption holds for the actual implementation;
  the integration test in T-D-N₁₅ covers the byte-identity check.
- **Sequencing with `v25-tcn-alpha-investigation`** — this feature's
  M-T1 can ship before the alpha-investigation lands a verdict
  (it's a UI handle, value-add regardless of the verdict). M-T2's
  schema work is the foundation for the actual retraining cycle
  the F4 verdict will trigger; sequencing M-T1 first lets the
  operator validate the UI shape on the existing BS-1 / BS-2 fixture
  runs before paying for the audit layer.
