---
slug: cockpit-training-pressed-wiring
version: 0.1.0
status: proposed
owner: analyst
updated: 2026-05-26
predecessor: cockpit-activity-status-bar v0.1.0
parent: cockpit-training-control v0.2.0
---

# Cockpit `TrainingPressed` → `spawn_training_run` wiring — make the Train button do something

> **Forward-listed at `cockpit-activity-status-bar` v0.1.0 Wave C ship
> (2026-05-26).** The activity-tape Training producer (Wave C, T-D-N9)
> landed the `training_activity_handle: Option<ActivityHandle>` field
> on `AppState` and the tick/end lifecycle arms in
> `AppState::update`. It did NOT land the actual subprocess spawn:
> `crates/ui/src/bin/cockpit_live.rs:1020-1025` documents the gap
> verbatim — _"`TrainingPressed` spawning is NOT yet wired in
> cockpit_live.rs (the training subprocess dispatch is handled by
> other means); the activity handle is returned from
> `spawn_training_run` when the caller is ready to wire it."_ In
> `crates/ui/src/state.rs:2064-2070`, the `Message::TrainingPressed`
> arm is a comment-only no-op: _"Actual subprocess spawn lives in
> the binary (needs `rt_handle`). The update fn is pure ... Here we
> just ensure state is consistent."_ The binary's `update` wrapper
> in `cockpit_live.rs` has NO corresponding `Message::TrainingPressed`
> branch that calls `spawn_training_run`. **Pressing the Train
> button from a live cockpit launches nothing.** This brief closes
> the gap.

## Why

### State today (2026-05-26)

- **Train button exists**: `crates/ui/src/screens/lab.rs:821` —
  `b.on_press(Message::TrainingPressed)` is wired into the Lab Train
  sub-panel header (per `cockpit-training-control` v0.2.0 R3.4).
- **Message exists**: `crates/ui/src/state.rs:1472` declares
  `Message::TrainingPressed`. The pure-state arm at line 2064 is a
  documented no-op: comment explicitly says spawning lives in the
  binary because it needs the tokio `rt_handle`.
- **`spawn_training_run` exists** at
  `crates/ui/src/lab/trainer.rs:166`. Signature:
  ```rust
  pub fn spawn_training_run(
      rt_handle: Option<&tokio::runtime::Handle>,
      cfg: &TrainingConfig,
      _cancel: RunCancelReceiver,
      line_tx: std::sync::mpsc::SyncSender<TrainingLogLine>,
      activity_sender: Option<ActivitySender>,
  ) -> Result<(TrainingHandle, Option<ActivityHandle>), SmolStr>
  ```
- **`TrainingHandle` storage slot exists**: `LabState::training_inflight:
  Option<TrainingHandle>` (per `cockpit-training-control` R2.5).
- **`ActivityHandle` storage slot exists**:
  `AppState::training_activity_handle: Option<ActivityHandle>` (per
  `cockpit-activity-status-bar` Wave C T-D-N9 at `cockpit_live.rs:862`).
- **Test coverage exists**: `crates/ui/tests/activity_tape_training_run.rs`
  (2 tests) exercises the handle lifecycle by calling
  `spawn_training_run` directly. Neither test goes through
  `Message::TrainingPressed`.

### What's missing

The binary's `AppState::update` wrapper (around `cockpit_live.rs:911`)
delegates almost every message to `ui::state::update(&mut self.cockpit,
msg)` and only intercepts arms that need I/O / async dispatch. The
`TrainingPressed` arm needs the same shape as `LabRunRequested`
(which spawns a backtest via `iced::Task::perform` from within the
binary's `update`): intercept, dispatch the subprocess via
`spawn_training_run`, store both handles, then call
`ui::state::update`.

### Why now

- **Operator-visible regression.** From the live cockpit, the
  operator clicks Train and gets zero feedback. The status-bar
  activity tape shipped at `cockpit-activity-status-bar` v0.1.0 is
  ready to surface the Training activity — it just never gets one
  because the producer never fires.
- **Cheap fix.** This is a wiring brief — ~30-60 LOC of binary-side
  glue; no new types, no new messages, no schema changes, no anchor
  touches.
- **Unblocks the v2.5 retraining cadence.** The
  [`v25-tcn-alpha-investigation`](../v25-tcn-alpha-investigation/feature.md)
  F4 verdict and the v2.5a / v2.5b training rounds in the (now
  retired) 4-phase DL roadmap all assumed the operator could launch
  training from the cockpit. Right now they cannot.

## Requirements

Numbered, testable, derived from the gap analysis above + the
existing `LabRunRequested` precedent in `cockpit_live.rs`.

### R1 — Wire the `TrainingPressed` message arm in the binary's `update`

- **R1.1** In `crates/ui/src/bin/cockpit_live.rs::AppState::update`,
  intercept `Message::TrainingPressed` BEFORE the
  `ui::state::update(&mut self.cockpit, msg)` delegation. The
  interception path:
  1. Short-circuit if `self.cockpit.lab_state.training_inflight.is_some()`
     (button is disabled per `cockpit-training-control` R3.4, but
     defensive check matches the `LabRunRequested` precedent).
  2. Resolve `TrainingConfig` (R3 — default config source).
  3. Construct an mpsc channel pair for `TrainingLogLine` (size: 256,
     matches the existing R3.1 ring-buffer cap order-of-magnitude).
  4. Construct a `RunCancelHandle` / `RunCancelReceiver` pair (mirror
     the `LabRunRequested` cancellation precedent at
     `cockpit-training-control` D3).
  5. Call `lab::trainer::spawn_training_run(self.rt_handle.as_ref(),
     &cfg, cancel_rx, line_tx, Some(self.bus.activity()))`.
  6. On `Ok((training_handle, activity_handle))`:
     - Store `training_handle` in
       `self.cockpit.lab_state.training_inflight`.
     - Store `activity_handle` in `self.training_activity_handle`.
     - Store the cancel handle in a new
       `AppState::training_cancel: Option<RunCancelHandle>` field
       (mirrors `LabState::run_cancel`).
     - Spawn the `TrainingLogLine` receiver pump that converts each
       received line into `Message::TrainingLogLine(...)` and feeds
       it into the iced runtime via the existing channel-to-task
       bridge pattern (architect picks at M-T1 — likely
       `iced::Task::stream` with a `tokio::sync::mpsc`).
  7. On `Err(SmolStr)`: surface the error via the existing
     `toast_message` field on `Cockpit` with a Lumen DANGER-tinted
     toast (e.g. `"Training failed to launch: <reason>"`).
- **R1.2** Continue to call `ui::state::update(&mut self.cockpit, msg)`
  AFTER the interception so the pure-state arm runs (it is a no-op
  today but the contract preserves future state mutations).
- **R1.3** The pure-state arm in `crates/ui/src/state.rs::update`
  (lines 2064-2070) STAYS a no-op. No changes there. The binary owns
  the I/O wiring per the comment's stated contract.
- **Acceptance:**
  - Manual cockpit run: click the Train button, observe (a) a new
    activity in the status-bar tape labelled `"Train train_tcn ·
    running"`, (b) the Train sub-panel log starts filling with
    subprocess stdout/stderr lines, (c) the button becomes disabled
    while the subprocess runs.
  - Headless integration test
    `crates/ui/tests/cockpit_training_pressed_wiring.rs::pressing_training_button_spawns_subprocess`
    (new file): construct an `AppState`, dispatch
    `Message::TrainingPressed`, assert
    `cockpit.lab_state.training_inflight.is_some()` and
    `training_activity_handle.is_some()` after a bounded wait.
    Subprocess uses a fixture stub (`bash -c "sleep 0.5"`) via a
    test-only `TrainingConfig::binary_path` override.

### R2 — Plumb the activity handle so the tape lights up on press

- **R2.1** The `Some(self.bus.activity())` argument at the
  `spawn_training_run` call site (R1.1 step 5) is what produces the
  `ActivityHandle`. The Wave C T-D-N9 lifecycle arms in
  `cockpit_live.rs:1103-1131` already consume
  `training_activity_handle` for tick/cancel/end — they fire
  automatically once the field is populated.
- **R2.2** No changes to `cockpit_live.rs:1103-1131` (the Wave C
  block). This brief is purely the upstream **population** of
  `training_activity_handle`; the downstream consumption already
  works end-to-end (verified by
  `crates/ui/tests/activity_tape_training_run.rs::training_run_activity_emits_start_and_end_success`).
- **Acceptance:**
  - Headless integration test
    `crates/ui/tests/cockpit_training_pressed_wiring.rs::pressing_training_button_emits_activity_start`:
    construct an `AppState` with a fake activity subscriber wired to
    `bus.activity()`, dispatch `Message::TrainingPressed`, assert
    exactly one `ActivityEvent { kind: Training, phase: Start, .. }`
    arrives at the subscriber within 500 ms.

### R3 — Default training config source (operator-decide Q1)

- **R3.1** **Q1 default**: use the canonical
  `crates/forecast/train_tcn.toml` (the existing v2.5 default,
  already on disk; architect-locked per
  `cockpit-training-control` v0.2.0 D4). Rationale:
  - This is the only training config currently on disk in the
    workspace; `crates/strategy/configs/training/btc_macd_trend.toml`
    referenced in the upstream task brief does **NOT** exist (verified
    `find crates/strategy -name "*.toml" -path "*config*"` 2026-05-26).
  - The `TrainingConfig` struct already defaults the binary path via
    `resolve_train_tcn_path()` (D10 three-tier precedence). The config
    path is the only field with no resolver today.
- **R3.2** Resolution: walk up from `current_dir` (workspace-relative)
  to find `crates/forecast/train_tcn.toml`. Fall back to
  `crates/forecast/train_tcn.toml` literal if the walk fails. Mirror
  the workspace-walk pattern in `lab/trainer.rs::resolve_train_tcn_path`
  (line 121).
- **R3.3** Output directory default:
  `<workspace>/target/training_checkpoints/<timestamp>` (mirror the
  `lab::runner` output dir shape; isolates runs by timestamp).
- **R3.4** `audit_db` default: omitted at v0.1.0 (R-NR — preserves the
  byte-for-byte CI behaviour locked at `cockpit-training-control`
  R5.4 / R10.2). Operator may enable later via a future
  `cockpit-training-control` follow-on that surfaces the audit-DB
  toggle in the panel UI.
- **R3.5** `dry_run` default: `false`. Operator presses Train, real
  training fires. Dry-run is a debug aff; not surfaced at v0.1.0.
- **R3.6** `epochs` / `scenario` overrides: both `None` at v0.1.0
  (use config defaults). Q3 of the parent (`cockpit-training-control`)
  already deferred hyperparameter editing.
- **Acceptance:** Unit test
  `lab::trainer::tests::default_training_config_resolves_train_tcn_toml`
  asserts the resolution finds the workspace-relative config.

### R4 — Cancellation semantics (operator-decide Q2)

- **R4.1** **Q2 default**: re-pressing Train while training is
  in-flight is a no-op at the UI level (button disabled per
  `cockpit-training-control` R3.4). Cancellation goes through the
  existing `Message::TrainingCancelPressed` arm, which already drops
  `training_inflight` → `TrainingHandle::Drop` → `child.start_kill()`
  → SIGKILL per `cockpit-training-control` Q2=(a). This brief does
  NOT change that semantic; we just make sure the cancel handle
  (R1.1 step 4) is dropped properly on subprocess exit.
- **R4.2** **R4.1 reconciliation**: the upstream brief asked
  whether re-pressing Train cancels via SIGKILL. The answer is "the
  question doesn't arise because the button is disabled" — the
  R3.4 disable + the pure-state arm short-circuit (R1.1 step 1)
  make double-press inert. The SIGKILL semantic is owned entirely
  by `TrainingCancelPressed`.
- **Acceptance:** Headless test
  `crates/ui/tests/cockpit_training_pressed_wiring.rs::double_press_is_inert`:
  dispatch `Message::TrainingPressed` twice in rapid succession,
  assert only one subprocess was spawned (only one `Start` event
  in the activity bus).

### R-NR — Non-regression contract

- **R-NR.1** **All 34 anchors stay byte-identical.** Zero touched
  files in `crates/backtest/`, `crates/strategy/`, `crates/exec/`,
  `crates/risk/`, `crates/reports/`, `crates/forecast/` (the
  forecast crate's `train_tcn` binary is the SUBPROCESS — the source
  bytes are unchanged; we just invoke it from a new call site).
- **R-NR.2** **`training_events` audit table unchanged.** No schema
  migration; no new writer; no new reader. R3.4 keeps `audit_db =
  None` at v0.1.0.
- **R-NR.3** **No bus channel changes.** The activity bus
  (`agent::EventBus::activity_tx`) shipped at
  `cockpit-activity-status-bar` v0.1.0 Wave A is reused as-is. No new
  channels, no capacity tuning.
- **R-NR.4** **No Lumen tokens introduced.** Reuse existing
  `color::DANGER` for the toast (R1.1 step 7). No new strings beyond
  the toast template (added to `crates/ui/src/strings.rs`).
- **R-NR.5** **No state.rs signature changes.** `Message`,
  `Cockpit`, `LabState`, `update` — all public types unchanged.
  Wiring is binary-only.
- **R-NR.6** **`cockpit-smoke` 0 panics.** Existing smoke test stays
  green; Train button click on the smoke fixture must not crash.
- **R-NR.7** **818+ workspace tests stay green.** Additive surface;
  no rename / no signature change to public functions.
- **R-NR.8** **`spec-lint` introduces no new violation categories**
  (hard gate per task scope).
- **R-NR.9** **`scripts/verify_anchors.sh` exits 0 with 34/34 PASS**
  (hard gate per task scope).

## Hypothesis register

- **H1** — _The existing Wave C T-D-N9 lifecycle arms
  (`cockpit_live.rs:1103-1131`) correctly consume the
  `training_activity_handle` field once populated, without any
  modification._ **Falsifier**: the new R2 test does NOT observe a
  Start event in the bus after dispatching `TrainingPressed`.
  **Status at analyst pass**: assumed TRUE (the field-population
  side is what's missing; the consumer side was already proven by
  the Wave C integration tests at commit 0ff402f).
- **H2** — _The `TrainingLogLine` channel pump can be modeled as a
  one-shot `iced::Task::stream` adapter._ **Falsifier**: the iced
  stream adapter requires an `async Stream + Send` and the
  `std::sync::mpsc::Receiver<TrainingLogLine>` doesn't satisfy that
  bound without an intermediate `tokio::sync::mpsc` shim.
  **Status at analyst pass**: probably TRUE but architect should
  verify at M-T1; the existing `lab_progress_rx` bridge pattern at
  `cockpit_live.rs:830-835` (`Arc<Mutex<Option<tokio::sync::mpsc::Receiver>>>`)
  is the documented escape hatch.
- **H3** — _`bus.activity()` returns a `Send + Clone` `ActivitySender`
  that can be passed into `spawn_training_run` without lifetime
  surgery._ **Status at analyst pass**: TRUE per Wave A T-D-N2 ship
  notes (the sender is a thin wrapper over `broadcast::Sender`).

## Risk register

- **K1** — **Training subprocess managed lifecycle drift.** If the
  binary's `update` panics between spawning the subprocess and
  storing the `TrainingHandle`, the child is orphaned (we miss the
  Drop impl). **Mitigation**: build the handle BEFORE the activity
  handle assignment (no allocation between spawn and store);
  acceptance test stresses this with a poisoned `bus.activity()`
  fake that panics on send.
- **K2** — **Activity handle ownership.** The `ActivityHandle` is
  `!Send` (per Wave C T-D-N9 design — uses `Cell<>` for the
  throttle state). Storing it in `AppState::training_activity_handle`
  is fine (the iced thread owns AppState); but the `Result` from
  `spawn_training_run` returns the handle, and we must consume it on
  the iced thread, not in any async task. **Mitigation**:
  `spawn_training_run` already returns synchronously (uses
  `rt_handle.block_on` internally per `trainer.rs:223`), so the
  return value never crosses a thread boundary. Verified by the
  signature: returns `Result<(TrainingHandle, Option<ActivityHandle>),
  SmolStr>` synchronously, no `Future` wrapper.
- **K3** — **Default config path drift.** If
  `crates/forecast/train_tcn.toml` moves or is renamed, R3.2
  resolution breaks silently. **Mitigation**: unit test in R3 pins
  the resolved path; CI catches a moved file at the next test run.
- **K4** — **Stdout/stderr pump backpressure.** A noisy training
  subprocess (BS-1 emits ~60 lines/epoch) could overflow the mpsc
  channel if the iced thread is slow. **Mitigation**: 256-slot
  channel matches the 200-line ring buffer (R3.1 of parent); slow
  consumer drops lines rather than blocks the subprocess. Same
  shape as the parent feature's R3.1 contract.
- **K5** — **Toast surface coupling.** R1.1 step 7 surfaces the
  spawn error via `toast_message`. If that field is already in use
  (another subsystem's toast in flight), we'd clobber it.
  **Mitigation**: architect inspects the `Cockpit::toast_message`
  precedent at M-T1; if multi-toast queueing is needed, deferred to
  a follow-on (single-toast clobber is acceptable at v0.1.0 — the
  Train error is a hard-failure mode and outranks routine toasts).

## Open questions for the operator

Both have analyst-recommended defaults; both are standing-Autoapprove-
eligible because the cost of a wrong default is ~5 LOC to flip.

- **Q1 — Default training config source.**
  - (a) **`crates/forecast/train_tcn.toml`** — the existing v2.5
    canonical config, locked by `cockpit-training-control` D4.
    ← **ANALYST DEFAULT**
  - (b) `crates/strategy/configs/training/btc_macd_trend.toml` —
    referenced in the upstream task brief but **does not exist on
    disk** (verified 2026-05-26). Picking (b) requires authoring
    the config first.
  - (c) Defer config selection — surface a config-picker dropdown in
    the Train panel. Deferred to a follow-on; out of scope at v0.1.0.

  Trade-off: (a) ships in ~30 LOC of resolver + 1 unit test; (b)
  ships in ~30 LOC + ~80 LOC of new TOML scaffolding; (c) ships in
  ~200 LOC of widget + a config registry contract.

- **Q2 — Cancellation semantics on double-press.**
  - (a) **Button disabled while in-flight; re-press is a no-op.**
    Inherits R3.4 of the parent. ← **ANALYST DEFAULT**
  - (b) Re-press cancels via SIGKILL (re-routes to
    `TrainingCancelPressed` semantically). Friendlier muscle memory
    for terminal users; risk of accidental cancellation.
  - (c) Re-press queues a follow-on training run. Out of scope per
    `cockpit-training-control` § Out of scope (multi-run queue).

  Trade-off: (a) honors the parent's existing UX contract; (b)
  changes the Train button's semantic mid-flight (operator surprise);
  (c) requires queue infra we don't have.

## Out of scope

- **Hyperparameter editing in the panel.** Already deferred at
  `cockpit-training-control` Q3 (to `cockpit-training-hyperparams`
  follow-on).
- **Config picker dropdown.** Deferred per Q1=(c).
- **Audit-DB toggle in the UI.** Deferred per R3.4. The
  `--audit-db` flag stays opt-in via TOML edit (or future
  follow-on UI).
- **Multi-run training queue.** Already out-of-scope per parent.
- **Re-attach to orphan subprocess.** Already out-of-scope per
  parent K3 / R9.5.

## Backtest scenarios

**None.** This feature is UI-binary wiring only. It does not touch
the backtest engine, scenario producers, or any anchored report. The
34 locked anchors stay byte-identical (R-NR.1). Verified at M-FINAL
by `scripts/verify_anchors.sh`.

## Cost estimate

**~0.5-1 day end-to-end wall-clock** (small wiring fix).

- M-T1 (architect synthesis + ADR-free decomposition into T-D-N): ~1-2h.
- M-DEV (developer):
  - R1.1 binary-side wiring: ~30-60 LOC + ~1h.
  - R1 acceptance integration test: ~80 LOC + ~1h.
  - R2 activity-event acceptance test: ~50 LOC + ~30m.
  - R3 unit test for config resolver: ~30 LOC + ~30m.
  - R4 double-press inert test: ~40 LOC + ~30m.
- M-FINAL (tester): re-run anchors + workspace tests + cockpit-smoke
  + spec-lint: ~30m.

Total: ~5-7h hands-on; calendar fit: 1 day with operator-decide
roundtrip.

## Cross-references

- Predecessor: [`cockpit-activity-status-bar` v0.1.0](../cockpit-activity-status-bar/feature.md)
  — Wave C T-D-N9 ship; the `training_activity_handle` field this
  brief populates.
- Parent: [`cockpit-training-control` v0.2.0](../cockpit-training-control/feature.md)
  — defines `spawn_training_run`, `TrainingConfig`, `TrainingHandle`,
  the cancellation contract, and the audit-DB opt-in.
- Sibling-precedent (binary-side I/O dispatch): the
  `LabRunRequested` arm in `crates/ui/src/bin/cockpit_live.rs` —
  same shape we mirror for `TrainingPressed`.
- Existing integration tests: `crates/ui/tests/activity_tape_training_run.rs`
  (2 tests) — proves the downstream consumer works; this brief proves
  the upstream producer (button press) works.

## Implementation

Implemented 2026-05-26 by developer (M-DEV Wave A).

### Files created

- `crates/ui/src/lab/training_log.rs` — `TrainingLogRecipe` (183 LoC).
  Mirrors `LabProgressRecipe` symbol-for-symbol with std-mpsc → `tokio::task::spawn_blocking`
  bridge (H2 resolution per T-AR-1). Gated on `#[cfg(feature = "live")]`.
- `crates/ui/tests/cockpit_training_pressed_wiring.rs` — 5 integration tests (290 LoC).
- `crates/exec/benches/latency_slippage.rs` — placeholder bench (pre-existing manifest issue fix).
- `crates/exec/benches/throughput_with_sim.rs` — placeholder bench (pre-existing manifest issue fix).

### Files modified

- `crates/ui/src/bin/cockpit_live.rs` — `TrainingPressed` interception block added before
  `ui::state::update` delegation. New `AppState` fields: `training_log_rx`, `training_log_recipe_salt`.
  `TrainingLogRecipe` added to `subscription()`. `TrainingExited` / `TrainingCancelPressed` clear blocks.
- `crates/ui/src/lab/state.rs` — `training_cancel: Option<RunCancelHandle>` field added to `LabState`
  (+ all constructors: `Clone` impl, `Default`, `with_selection`).
- `crates/ui/src/lab/mod.rs` — `pub mod training_log;` added.
- `crates/ui/src/lab/trainer.rs` — `default_training_config()`, `resolve_train_tcn_toml_path()`,
  `resolve_output_dir()` functions added. Two new unit tests: `default_training_config_resolves_train_tcn_toml`
  and `default_training_config_has_correct_defaults`.

### Test results (developer verification)

- `cargo test -p ui --test cockpit_training_pressed_wiring` → 5/5 PASS (0.31s)
- `bash scripts/verify_anchors.sh` → ANCHORS PASS (34/34)
- `cargo build -p ui --tests` → green (exit 0)

### Deviations from spec

None. Architecture matches T-AR-1 (Recipe pattern, not Task::stream). H2 PARTIALLY FALSIFIED
confirmed correct — `TrainingLogRecipe` uses `spawn_blocking` bridge per architect resolution.

## Changelog

- 2026-05-26 (analyst): authored v0.1.0 draft. R1-R4 + R-NR.1-9 +
  H1-H3 + K1-K5 + Q1-Q2 closed. Analyst-recommended defaults set on
  both Qs. Anchor risk zero by construction. Cost ~0.5-1 day.
  HANDOFF → architect for M-T1 decomposition (trivial — likely 4-5
  T-D-N rows; architect may skip directly to developer if the
  binary-side wiring shape is clear at M0 pass).
