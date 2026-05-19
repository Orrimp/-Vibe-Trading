---
slug: cockpit-training-control
status: in-progress
owner: architect
updated: 2026-05-19
---

# Tasks — cockpit-training-control

> T-AR-2 decomposition complete (2026-05-19). Architect-locked task
> graph below; developer executes via the wave plan in
> [feature.md § Design / Parallelism map](feature.md#parallelism-map-for-the-orchestrator).
> Each row honours the honest-tick contract: Owner / Milestone /
> Depends on / Blocks / file:line / test cmd / expected output line.

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
- [x] **T-AR-1** (2026-05-19) — `train_tcn` binary path-resolution
  strategy locked at three-tier precedence (`current_exe`-relative →
  workspace-relative → `cargo run` dev fallback). See
  [feature.md § Design D10](feature.md#d10--path-resolution-for-the-train_tcn-binary).
- [x] **T-AR-2** (2026-05-19) — R1-R10 translated into the T-D-N task
  graph below; M-T1 / M-T2 / M-FINAL stubs replaced.
- [x] **T-AR-3** (2026-05-19) — Authored
  [ADR-0034](../architecture/adr/0034-cockpit-training-control.md);
  updated `spec/architecture/adr/README.md` registry. Locks
  audit-DB-as-seam, schema, subprocess lifecycle, R6 in-panel curves,
  K5 mitigation.
- [x] **T-AR-4** (2026-05-19) — K5 mitigation locked: golden-CLI
  insta snapshot test (rejected: runtime `--print-config-schema`
  validation — ~5× LOC for same guarantee). See
  [feature.md § Design D9](feature.md#d9--k5-mitigation-golden-cli-ci-diff-snapshot).
- [x] **T-AR-5** (2026-05-19) — Sequencing locked: M-T1 ships before
  M-T2 lands. Operator validates Tier 1 shape on existing BS-1 /
  BS-2 fixture before paying for Tier 2's audit-schema cost.
- [x] **T-AR-6** (2026-05-19) — Migration
  `crates/audit/migrations/010_training_events.sql` committed (the
  schema body lives in the file; the SQL is byte-equal to what
  ADR-0034 § D2 specifies).
- [x] **T-AR-7** (2026-05-19) — trace.toml `arch` column populated
  for `REQ-COCKPIT-TRAIN-001`; state `proposed → in-progress`.
- **Acceptance:** `feature.md § Design` populated (DONE); this file's
  T-D-N rows populated (DONE); trace.toml `REQ-COCKPIT-TRAIN-001.arch`
  populated (DONE); ADR-0034 committed (DONE); migration 010 file
  committed (DONE).

## M-T1 — Tier 1: launch button + log tail

_owner: developer (UI focus). Wave-parallelizable per
[feature.md § Parallelism map](feature.md#parallelism-map-for-the-orchestrator)._

Maps to **R1, R2, R3, R8.1, R9.1, R9.3, R9.4, R10.4**.

### Wave A — pure new modules (parallel-safe)

- [ ] **T-D-N1** Module `crates/ui/src/lab/trainer.rs` (new).
  - Owner: developer • Milestone: M-T1 • Depends on: T-AR-* • Blocks: T-D-N3, T-D-N4
  - File:line: `crates/ui/src/lab/trainer.rs` (new file ~250 LOC)
  - Body: `spawn_training_run(rt_handle, cfg, cancel) -> iced::Task<Message>`
    mirroring `lab::runner::spawn_lab_run` shape (`runner.rs:158`).
    Spawns `tokio::process::Command::new(train_tcn_path).args([...])`
    with stdout/stderr piped, streams lines via `tokio::io::BufReader`
    into bounded `std::sync::mpsc::SyncSender<TrainingLogLine>` (capacity
    256). `TrainingHandle { _stdin: ChildStdin, _kill: ChildKillHandle }`
    Drop impl calls `child.start_kill()` (R2.4). Reuses
    `RunCancelHandle`/`RunCancelReceiver` (imported, NOT copied) from
    `lab::runner`. Path resolution per D10.
  - Inline `#[cfg(test)] mod tests`:
    - `cancel_handle_drop_kills_child` — spawns `sleep 60`,
      drops handle, asserts exit within 200ms.
    - `stdout_lines_pipe_to_channel` — spawns `echo line1; echo line2`,
      asserts both lines surface in mpsc.
    - `binary_missing_returns_err_sync` — passes a nonsense binary
      path, asserts the spawn returns `Err(_)` synchronously
      (no panic, no subprocess).
  - Test cmd: `cargo test -p ui --lib lab::trainer`
  - Expected: `test result: ok. 3 passed; 0 failed`
- [ ] **T-D-N2** Widget `crates/ui/src/widgets/training_log.rs` (new).
  - Owner: developer • Milestone: M-T1 • Depends on: T-AR-* • Blocks: T-D-N3, T-D-N6
  - File:line: `crates/ui/src/widgets/training_log.rs` (new file ~180 LOC)
  - Body: 200-entry `VecDeque<SmolStr>` ring buffer. Vertical
    `Column` of `Text` rows. Auto-scroll anchored to bottom by
    default; click-to-freeze (state in widget) + Lumen "Jump to
    bottom" chip (hidden when anchored). Lumen Phase 1 tokens only
    (no new tokens). Public API: `TrainingLog::new(lines: &[SmolStr],
    anchored: bool) -> Element`, `push_line(buf: &mut RingBuffer,
    line: SmolStr)`.
  - Inline `#[cfg(test)] mod tests`:
    - `ring_buffer_evicts_oldest` — push 250 lines, assert
      `pop_front` semantics + last 200 visible.
    - `freeze_on_click_unsticks_autoscroll` — assert state transition.
  - Test cmd: `cargo test -p ui --lib widgets::training_log`
  - Expected: `test result: ok. 2 passed; 0 failed`

### Wave B — Lab screen integration (sequential within wave)

- [ ] **T-D-N3** Lab screen integration in `crates/ui/src/screens/lab.rs`.
  - Owner: developer • Milestone: M-T1 • Depends on: T-D-N1, T-D-N2 • Blocks: T-D-N4, T-D-N6
  - File:line: `crates/ui/src/screens/lab.rs:97` (extend
    `chart_canvas_height_for_body` for ~240 px expanded / ~32 px
    collapsed Train panel allocation); `screens/lab.rs::view` (insert
    Train panel below volume histogram per R1.2).
  - Buttons (R3.4): **Train** (primary; disabled when
    `LabState::training_inflight.is_some()`), **Cancel** (visible only
    when `training_inflight.is_some()`), **Clear log** (clears ring
    buffer). Lumen `run_button` token shape (no new tokens). Status
    strip above log per R3.5 (at Tier 1: Idle / Training… / Done /
    Failed: …; no epoch counts yet — those arrive at T-D-N13).
  - Test cmd: `cargo test -p ui --test panel_snapshots -- training_panel`
  - Expected: 2 PASS (collapsed-default + expanded variants)
- [ ] **T-D-N4** Cockpit `Message` arms in `crates/ui/src/state.rs`.
  - Owner: developer • Milestone: M-T1 • Depends on: T-D-N3 • Blocks: T-D-N6
  - File:line: `crates/ui/src/state.rs` (`Message` enum + `update`)
  - Variants: `TrainingPressed`, `TrainingCancelPressed`,
    `TrainingLogLine(SmolStr)`, `TrainingExited(std::process::ExitStatus)`,
    `TrainingPanelToggled`, `TrainingClearLog`.
  - Update body: wire each variant to the appropriate `LabState`
    mutation. `TrainingPressed` calls
    `lab::trainer::spawn_training_run` and stashes the returned
    `TrainingHandle` in `LabState::training_inflight`.
    `TrainingCancelPressed` drops the handle (Drop impl SIGKILLs).
  - Test cmd: `cargo test -p ui --lib state::tests::training_arms`
  - Expected: ≥ 3 PASS (one per state transition arm).
- [ ] **T-D-N5** `LabStateJson` persistence extension in
  `crates/ui/src/lab/persistence.rs`.
  - Owner: developer • Milestone: M-T1 • Depends on: T-D-N4 • Blocks: T-D-N6
  - File:line: `crates/ui/src/lab/persistence.rs` (add
    `training_panel_collapsed: bool` field, `#[serde(default)]` so a
    pre-feature JSON loads with `true` per R8.1).
  - Inline tests:
    - `training_panel_collapsed_roundtrips`
    - `pre_feature_json_loads_collapsed_true`
  - Test cmd: `cargo test -p ui --lib lab::persistence::tests -- training_panel`
  - Expected: 2 PASS.

### Wave C — snapshots (parallel; gated on Wave B)

- [ ] **T-D-N6** insta snapshots (Tier 1):
  `lab__training_panel_collapsed_default`,
  `lab__training_panel_expanded`,
  `training_log__ring_buffer_200_lines`.
  - Owner: developer • Milestone: M-T1 • Depends on: T-D-N3, T-D-N4, T-D-N5 • Blocks: M-T1 acceptance
  - File:line: `crates/ui/tests/panel_snapshots.rs` (insta fixtures);
    `crates/ui/tests/snapshots/*.snap` (generated).
  - Test cmd: `cargo test -p ui --test panel_snapshots`
  - Expected: all training-panel snapshots PASS; no `.snap.new` left
    behind.

### M-T1 Acceptance

- [ ] `cargo test -p ui` 100% PASS (incl. T-D-N1..N6 new tests).
- [ ] Manual cockpit run (`cargo run --bin cockpit --features fixtures`):
  Train → log streams → Cancel kills subprocess →
  `training_panel_collapsed` state survives cockpit restart.
- [ ] `scripts/cockpit_smoke.sh` exit 0.
- [ ] Zero anchor files touched (R10.4 + R10.5).

## M-T2 — Tier 2: audit events + live curves

_owner: developer. Three independent lanes in Wave D per the
parallelism map._

Maps to **R4, R5, R6, R7, R8.2, R8.3, R9.2, R9.5, R10.1-R10.3**.

### Wave D — audit + train_tcn + axis-helper (3 parallel lanes)

**Lane 1 — audit schema + writers + readers** (sequential within lane):

- [ ] **T-D-N7** Migration `crates/audit/migrations/010_training_events.sql`.
  - Owner: developer • Milestone: M-T2 • Depends on: T-AR-6 (already
    committed by architect) • Blocks: T-D-N8, T-D-N9, T-D-N10
  - File:line: ALREADY COMMITTED at T-AR-6. Developer verifies the
    migration applies cleanly on a fresh DB + idempotent on re-apply.
  - Test cmd: `cargo test -p audit --lib bootstrap::tests -- migration_010`
  - Expected: 1 PASS (migration applies cleanly + idempotent).
- [ ] **T-D-N8** Audit writers in `crates/audit/src/journal.rs`.
  - Owner: developer • Milestone: M-T2 • Depends on: T-D-N7 • Blocks: T-D-N10, T-D-N11
  - File:line: `crates/audit/src/journal.rs` (append-only; follow
    `post_strategy_signal` precedent at `journal.rs:266`).
  - Functions: `post_training_start`, `post_training_epoch`,
    `post_training_finish`, `post_training_failed` (signatures
    locked in feature.md § Design D5). Each `#[instrument]`'d.
  - Inline tests:
    - `post_training_start_writes_row`
    - `post_training_epoch_writes_row_with_losses_as_text`
    - `post_training_finish_sets_model_revision`
    - `post_training_failed_sets_error_message`
  - Test cmd: `cargo test -p audit --lib journal::tests -- training`
  - Expected: 4 PASS.
- [ ] **T-D-N9** Audit readers in `crates/audit/src/query.rs` +
  value types in `crates/core/src/lib.rs`.
  - Owner: developer • Milestone: M-T2 • Depends on: T-D-N8 • Blocks: T-D-N11, T-D-N13, T-D-N14
  - File:line: `crates/audit/src/query.rs` (append; follow
    `recent_signals` at `query.rs:470` shape); `crates/core/src/lib.rs`
    (add `TrainingEventRow`, `TrainingRunSummary`, `OrphanTrainingRun`
    structs).
  - Functions: `recent_training_events(ledger, since, until)`,
    `latest_training_run(ledger)`,
    `orphan_training_runs(ledger, fresh_window)`. Use the
    parameterised orphan query from feature.md § Design D5.
  - Inline tests:
    - `recent_training_events_filters_by_window`
    - `recent_training_events_empty_window_returns_ok_empty`
    - `latest_training_run_returns_none_on_empty_db`
    - `latest_training_run_returns_most_recent`
    - `orphan_training_runs_excludes_completed_runs`
    - `orphan_training_runs_excludes_failed_runs`
    - `orphan_training_runs_respects_fresh_window`
  - Test cmd: `cargo test -p audit --lib query::tests -- training`
  - Expected: 7 PASS.

**Lane 2 — `train_tcn` instrumentation** (depends on Lane 1's T-D-N7-N8):

- [ ] **T-D-N10** `train_tcn` instrumentation in
  `crates/forecast/src/bin/train_tcn.rs`.
  - Owner: developer • Milestone: M-T2 • Depends on: T-D-N8 (writers
    exist) • Blocks: T-D-N15
  - File:line:
    - `train_tcn.rs:117` — add `--audit-db <PATH>` arg at the END of
      the `Cli` struct (after `scenario: Option<String>`).
    - `train_tcn.rs:280-292` — `start` emission edge AFTER the
      existing `info!("train_tcn starting", ...)`. Capture `run_id =
      Uuid::new_v4().to_string()`, `pid = std::process::id() as i32`.
    - `train_tcn.rs:523-530` — `epoch` emission edge ALONGSIDE the
      existing `info!(epoch = ..., "epoch complete")`. Capture
      `wall_clock_ms` via `Instant::now() - epoch_start`.
    - `train_tcn.rs:565-577` (end of `write_checkpoint`) — `finish`
      emission edge with the canonical `model_revision` SHA.
    - `train_tcn.rs:236` (`fn main` entry) — wrap the body in
      `std::panic::catch_unwind` boundary; on `Err(_)` OR panic,
      open the Ledger (if not already open) and write a
      `kind='failed'` row before re-raising (R5.2 + D4).
    - New helper: `audit_writer.rs` next to `train_tcn.rs` (lazy
      `Ledger` instantiation; per-run `Runtime::new()`; `block_on`'d
      writes).
  - Integration test
    `crates/forecast/tests/train_tcn_audit_emits.rs` (new file):
    - `train_tcn_dry_run_with_audit_db_emits_start_and_finish_only`
      (1 start + 1 finish, 0 epoch rows in dry-run mode).
    - `train_tcn_audit_db_byte_identical_metadata_json` (assert
      `<sha>.metadata.json` is byte-equal with and without
      `--audit-db`). This is the R5.4 + R10.2 anchor-neutrality check.
  - Non-regression test:
    `crates/forecast/tests/train_tcn_no_audit_db_writes_nothing.rs`
    asserts no SQLite handle is opened when `--audit-db` is omitted.
  - Test cmd: `cargo test -p forecast --test train_tcn_audit_emits --test train_tcn_no_audit_db_writes_nothing`
  - Expected: 3 PASS total.

**Lane 3 — axis-helper extraction** (independent of Lanes 1-2):

- [ ] **T-D-N17** Extract axis-rendering helpers from
  `widgets::chart` into new shared
  `crates/ui/src/widgets/axis.rs` (`pub(crate)`).
  - Owner: developer • Milestone: M-T2 • Depends on: T-AR-* • Blocks: T-D-N12
  - File:line: `crates/ui/src/widgets/chart.rs` (extract ~30 LOC of
    tick spacing, label formatting, line-tessellation helpers) into
    `crates/ui/src/widgets/axis.rs` (new file). Update `mod.rs` to
    expose `pub(crate) mod axis`. `widgets::chart` imports the
    extracted helpers; `widgets::training_plot` (T-D-N12) will too.
  - **Invariant:** the chart's render output is byte-identical
    before/after extraction (pure refactor). Inline tests assert
    `axis::tick_positions(scale, max_ticks)` produces the same Vec
    as the previous in-chart helper.
  - Test cmd: `cargo test -p ui --lib widgets::axis && cargo test -p ui --test render_snapshots -- chart`
  - Expected: axis tests PASS; chart render snapshots remain
    unchanged (the refactor is a no-op for chart output).

### Wave E — subscription + plot + status strip + orphan-detect (sequential)

- [ ] **T-D-N11** Subscription
  `crates/ui/src/lab/training_subscription.rs` (new module).
  - Owner: developer • Milestone: M-T2 • Depends on: T-D-N9 (readers
    exist), T-D-N4 (Message arms exist) • Blocks: T-D-N13, T-D-N14, T-D-N18
  - File:line: `crates/ui/src/lab/training_subscription.rs` (new file
    ~150 LOC). Mirror the `cockpit_live::subscription_for` shape
    (`crates/ui/src/live.rs:104`). Recipe identity:
    `("training_events", run_id)` per ADR-0034 § D6.
  - New Message arm: `Message::TrainingEventsRefreshed(Vec<TrainingEventRow>)`
    (extends T-D-N4's arm list).
  - Update body: append rows to
    `LabState::training_events: VecDeque<TrainingEventRow>` (capacity
    1024). Compute `latest_summary` for status strip.
  - Inline tests:
    - `polls_at_1hz_when_inflight` (fake clock — advance 5 ticks,
      assert 5 messages).
    - `stops_when_training_completes` (set `training_inflight = None`,
      assert recipe terminates).
    - `last_seen_ts_advances_only_on_new_rows` (assert idempotent
      polling does not re-emit rows already seen).
  - Test cmd: `cargo test -p ui --lib lab::training_subscription`
  - Expected: 3 PASS.
- [ ] **T-D-N12** New `crates/ui/src/widgets/training_plot.rs` module.
  - Owner: developer • Milestone: M-T2 • Depends on: T-D-N11, T-D-N17
    (axis helpers exist) • Blocks: T-D-N18
  - File:line: `crates/ui/src/widgets/training_plot.rs` (new file
    ~250 LOC). Public API per feature.md § Design D8.
  - Render path: tiny-skia canvas with Lumen `color::ACCENT_2` /
    `ACCENT_3` for train/val loss lines. Auto-scaled y-axis
    `[0, max * 1.1]`. Pre-first-epoch state: `iced_aw::spinner` +
    "Warming up — first epoch landing shortly". Empty state:
    "No training run in flight". Composes inside the Train panel's
    column (NOT on the main chart canvas).
  - Inline tests:
    - `y_axis_scales_to_max_plus_10_pct`
    - `empty_series_renders_placeholder_only`
    - `single_epoch_renders_two_dots` (degenerate case).
  - Test cmd: `cargo test -p ui --lib widgets::training_plot`
  - Expected: 3 PASS.
- [ ] **T-D-N13** Status strip wiring (R3.5) in
  `crates/ui/src/screens/lab.rs`.
  - Owner: developer • Milestone: M-T2 • Depends on: T-D-N11 • Blocks: T-D-N18
  - File:line: `crates/ui/src/screens/lab.rs` (Train-panel status
    strip — Tier 2 wiring upgrades the Tier 1 strip from
    "Idle/Training…/Done" to "Idle / Training (epoch N/M, t=Ts) /
    Cancelled / Failed: <err> / Done: <model_revision short SHA>").
  - Derives from `LabState::training_events.last()` filtered to
    appropriate kind.
  - Test cmd: `cargo test -p ui --test panel_snapshots -- training_status_strip`
  - Expected: 4 PASS (one snapshot per status variant).
- [ ] **T-D-N14** Orphan-detect on cockpit boot.
  - Owner: developer • Milestone: M-T2 • Depends on: T-D-N9 (orphan
    query exists), T-D-N11 (subscription) • Blocks: T-D-N18
  - File:line:
    - `crates/ui/src/bin/cockpit.rs` (boot path) — call
      `query::orphan_training_runs(&ledger, Duration::hours(24))`
      once at startup; collect orphans into
      `Cockpit::orphan_training_annotations: Vec<OrphanAnnotation>`.
    - `crates/ui/src/widgets/cockpit_chrome.rs` (or wherever the
      cockpit chrome status strip lives — confirm exact module on
      first read) — render the annotation per ADR-0034 § D7.
    - New helper `crates/ui/src/lab/pid_alive.rs` — Unix
      `libc::kill(pid, 0)` semantics + Windows `OpenProcess` /
      `GetExitCodeProcess` (gated via `#[cfg(unix)]` / `#[cfg(windows)]`).
    - Click-target: clicking the **Train** chip expands the panel
      AND (for live orphans only) spawns the training subscription
      against the orphan's `run_id`.
  - Inline tests:
    - `pid_alive_returns_true_for_self`
    - `pid_alive_returns_false_for_nonexistent`
    - `orphan_annotation_renders_when_pid_alive`
    - `orphan_annotation_renders_dead_when_pid_dead`
  - Test cmd: `cargo test -p ui --lib lab::pid_alive --lib screens::lab::tests::orphan`
  - Expected: 4 PASS.
- [ ] **T-D-N16** Status-strip strings in `crate::strings`.
  - Owner: developer • Milestone: M-T2 • Depends on: T-D-N13, T-D-N14 • Blocks: T-D-N18
  - File:line: `crates/ui/src/strings.rs` (add constants:
    `TRAINING_STATUS_IDLE`, `TRAINING_STATUS_TRAINING_FMT`,
    `TRAINING_STATUS_CANCELLED`, `TRAINING_STATUS_FAILED_FMT`,
    `TRAINING_STATUS_DONE_FMT`, `ORPHAN_LIVE_FMT`, `ORPHAN_DEAD_FMT`).
  - **No string literals** in source per Lumen contract — replace any
    inline strings introduced in T-D-N13 / T-D-N14 with `strings::*`
    refs.
  - Test cmd: existing snapshot tests catch any drift; explicit grep
    invariant: `! grep -rn '"Training (' crates/ui/src/screens crates/ui/src/widgets`.
  - Expected: grep returns 0 results.

### Wave F — test sweep + snapshots (parallel; gated on E)

- [ ] **T-D-N15** Full unit + integration test sweep per R4-R7
  acceptance gates.
  - Owner: developer • Milestone: M-T2 • Depends on: T-D-N8, T-D-N9,
    T-D-N10, T-D-N11 • Blocks: M-T2 acceptance
  - File:line: this row is the orchestrating cmd — the individual
    tests already land at T-D-N8/N9/N10/N11. The row exists to make
    the orchestrator's gate explicit.
  - Test cmd: `cargo test -p audit -p forecast -p ui`
  - Expected: 100% PASS workspace-subset.
- [ ] **T-D-N18** insta snapshots (Tier 2):
  `training_plot__two_lines_5_epochs`,
  `training_plot__empty_state`,
  `training_plot__warming_up_with_spinner`,
  `cockpit_chrome__orphan_live_annotation`,
  `cockpit_chrome__orphan_dead_annotation`.
  - Owner: developer • Milestone: M-T2 • Depends on: T-D-N12, T-D-N14, T-D-N16 • Blocks: M-T2 acceptance
  - File:line: `crates/ui/tests/panel_snapshots.rs` + fixture rows in
    `crates/ui/tests/fixtures/training_plot_5_epochs.json`.
  - Test cmd: `cargo test -p ui --test panel_snapshots -- training_plot cockpit_chrome::orphan`
  - Expected: 5 snapshot PASS; no `.snap.new` left behind.

### M-T2 Acceptance

- [ ] All of M-T1 acceptance PASS.
- [ ] `cargo test -p audit -p forecast -p ui` 100% PASS.
- [ ] `scripts/verify_anchors.sh` 19/19 PASS (R10 contract).
- [ ] Manual cockpit run shows live loss curves advancing during a
  fixture training run.
- [ ] Manual cockpit-crash + restart test shows the orphan-detect
  status-strip annotation.

## M-FINAL — Tester sweep

_owner: tester_

- [ ] Run the full validate gate: `cargo fmt`, `cargo clippy -- -D warnings`,
  `cargo test --workspace`, `scripts/verify_anchors.sh`,
  `scripts/cockpit_smoke.sh`.
- [ ] Verify the 19 body-SHA-256 anchors are byte-identical to the
  pre-feature baseline (R10).
- [ ] Verify `<sha>.metadata.json` byte-identity with vs. without
  `--audit-db` (T-D-N10's `train_tcn_audit_db_byte_identical_metadata_json`
  integration test gate).
- [ ] Author `spec/cockpit-training-control/reports/test-final-<YYYY-MM-DD>.md`
  per the
  [test-report template](../../.claude/skills/rust-test/templates/test-report.md).
  Include: anchor diff, training-DB-roundtrip log, orphan-detection
  manual run notes (one cockpit-kill mid-training + restart).
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
  (R5 emits to a sidecar table, not into the metadata). T-D-N10's
  `train_tcn_audit_db_byte_identical_metadata_json` test is the gate
  enforcing this assumption.
- **Sequencing with `v25-tcn-alpha-investigation`** — this feature's
  M-T1 can ship before the alpha-investigation lands a verdict
  (it's a UI handle, value-add regardless of the verdict). M-T2's
  schema work is the foundation for the actual retraining cycle
  the F4 verdict will trigger; sequencing M-T1 first lets the
  operator validate the UI shape on the existing BS-1 / BS-2 fixture
  runs before paying for the audit layer.
- **WAL-mode latent gap** — ADR-0034 § D1 surfaces a latent gap in
  `Ledger::open` (`crates/audit/src/ledger.rs:20`): the `?mode=rwc`
  URL does not issue `PRAGMA journal_mode = WAL;`. Non-blocking for
  this feature given the 1-write-per-5-30-min cadence and 1 Hz
  indexed reader. Filed as a workspace-wide follow-up; tracked in
  `spec/backlog.md`.
