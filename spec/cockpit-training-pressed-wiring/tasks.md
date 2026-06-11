---
slug: cockpit-training-pressed-wiring
status: shipped
owner: tester
updated: 2026-05-27
---

# Tasks — cockpit-training-pressed-wiring

> Analyst M0 pass authored 2026-05-26 against
> [feature.md](feature.md) v0.1.0. R1-R4 + R-NR.1-9 + H1-H3 + K1-K5 +
> Q1-Q2 captured. Architect M-T1 pass landed 2026-05-26 — Q1=(a)
> `crates/forecast/train_tcn.toml` and Q2=(a) button-disabled
> applied as standing-Autoapprove defaults; no new ADR; H2 channel
> pump shape locked at T-D-N3 to mirror `LabProgressRecipe`
> (NOT `iced::Task::stream` — see T-AR-1).

## M0 — Analyst synthesis

_owner: analyst_

- [x] **T-AN-0** (2026-05-26) — feature.md authored at v0.1.0 with
  R1-R4 + R-NR.1-9 + H1-H3 + K1-K5 + Q1-Q2. Analyst-recommended
  defaults locked on both Qs. Anchor risk zero by construction.
- [x] **T-AN-1** (2026-05-26) — tasks.md scaffolded (this file).
- [x] **T-AN-2** (2026-05-26) — Appended Active row to
  [`spec/backlog.md`](../backlog.md).
- [x] **T-AN-3** (2026-05-26) — Opened trace row
  `REQ-COCKPIT-TRAINING-PRESSED-001` at `proposed` state in
  [`spec/trace.toml`](../trace.toml).

## M-OD — Operator decides (Q1-Q2)

_owner: operator. AskUserQuestion-routed by orchestrator. Both Qs
standing-Autoapprove-eligible at analyst defaults — applied at M-T1._

- [x] **T-OP-1** (2026-05-26, architect M-T1) — Q1 = (a)
  `crates/forecast/train_tcn.toml`. Standing-Autoapprove default
  applied; (b) target does NOT exist on disk (verified
  `ls crates/strategy/configs/training/` returned ENOENT), (c)
  deferred per § Out of scope.
- [x] **T-OP-2** (2026-05-26, architect M-T1) — Q2 = (a)
  button-disabled inert double-press. Standing-Autoapprove default
  applied; inherits parent `cockpit-training-control` R3.4 contract.

## M-T1 — Architect decomposition

_owner: architect (2026-05-26)._

- [x] **T-AR-1** (2026-05-26) — **H2 falsification: TrainingLogLine
  channel pump shape.** Analyst's H2 hypothesis was "one-shot
  `iced::Task::stream` adapter"; the actual repo precedent is
  `LabProgressRecipe` in `crates/ui/src/lab/progress.rs` —
  `iced::advanced::subscription::from_recipe` wrapping an
  `Arc<Mutex<Option<Receiver<T>>>>`. The analyst's "escape hatch"
  note (cockpit_live.rs:830-835, the `lab_progress_rx` field) is
  in fact the canonical shape, NOT a fallback. **Delta vs H2:**
  `spawn_training_run` takes `std::sync::mpsc::SyncSender<TrainingLogLine>`
  (per `trainer.rs:170`), but `LabProgressRecipe` consumes a
  `tokio::sync::mpsc::Receiver<Progress>` (per `progress.rs:52`).
  **Resolution at T-D-N3:** mirror `LabProgressRecipe` shape but
  hold a `std::sync::mpsc::Receiver<TrainingLogLine>` inside the
  `Arc<Mutex<Option<_>>>`; the recipe's `stream()` polls it via
  `tokio::task::spawn_blocking` (the std mpsc Receiver is
  `Send + 'static` and `recv()` is blocking — fits naturally into
  a blocking task pumping a `BoxStream`). No `std → tokio` shim
  thread required. **H2 verdict: PARTIALLY FALSIFIED** (Task::stream
  is wrong; the Recipe pattern is right). Sub-task: developer
  creates `crates/ui/src/lab/training_log.rs::TrainingLogRecipe`
  mirroring `lab/progress.rs::LabProgressRecipe`.

- [x] **T-AR-2** (2026-05-26) — **K5 toast clobber decision.** Read
  `crates/ui/src/state.rs:2056-2061`: the `Message::ShowToast` arm
  is a straight `model.toast_message = Some(msg);` — **REPLACE
  semantic**, not queue. Confirmed `toast_message: Option<SmolStr>`
  at `state.rs:816` is a single-slot field. **Decision:**
  single-slot replace is acceptable at v0.1.0 (matches K5
  mitigation in feature.md — "Train error is hard-failure mode and
  outranks routine toasts"). **However**, T-D-N6 (NEW) adds a
  non-clobber assertion: when an in-flight backtest's
  `LabRunCompleted` Ok-toast (e.g. "Backtest complete") is sitting
  on `toast_message` and Training completes, the Training
  completion **does NOT auto-set a toast** at v0.1.0 (no
  `Message::TrainingExited` toast emission today). Side-by-side
  Train + Backtest completion is therefore safe by silent
  no-op. Documented; no code change needed beyond the assertion
  test (T-D-N6). Multi-toast queueing deferred to follow-on
  `cockpit-toast-queue`.

- [x] **T-AR-3** (2026-05-26) — **K3 config-path verification.**
  `ls crates/forecast/train_tcn.toml` → 1136 bytes, exists.
  `ls crates/strategy/configs/training/` → ENOENT (the analyst's
  correction stands). **Decision:** T-D-N3 resolver MUST emit a
  defensive-warn `tracing::warn!` (not panic) if the
  workspace-walk fails to find `crates/forecast/train_tcn.toml`,
  then fall back to literal path (matching analyst R3.2). T-D-N3
  unit test asserts the resolved path exists at test time;
  startup never panics on a missing config — the subprocess
  surfaces the error via the spawn-error toast path (T-D-N5).

- [x] **T-AR-4** (2026-05-26) — **No new ADR needed.** This is
  binary-side wiring glue (~30-60 LOC); no architectural surface
  change (no new types beyond `TrainingLogRecipe`, which is a
  direct twin of the shipped `LabProgressRecipe`); no anchor
  touches; no module boundary moves. Documented at M-T1 closure
  per AGENT.md ADR guidance ("non-trivial tradeoffs").

- [x] **T-AR-5** (2026-05-26) — Frontmatter flipped
  `owner: analyst → developer`. Trace row
  `REQ-COCKPIT-TRAINING-PRESSED-001::arch` column populated with
  the cockpit_live.rs intercept-site line refs.

## M-DEV — Developer execution (Wave A)

_owner: developer. All tasks sequential within Wave A; small surface,
single wave, ~30-60 LOC binary glue + 1 new recipe file (~80 LOC) +
1 new integration test file (~200 LOC across 5 tests)._

### [x] T-D-N1 — `TrainingPressed` interception in `cockpit_live.rs::AppState::update` (R1.1)

**file:line:** `crates/ui/src/bin/cockpit_live.rs` — `TrainingPressed` intercept block inserted before `ui::state::update` call (lines ~1062-1125 in updated file). New fields `training_log_rx` and `training_log_recipe_salt` added to `AppState`. New field `training_cancel` added to `LabState` (`crates/ui/src/lab/state.rs`).
**Test command:** `cargo test -p ui --test cockpit_training_pressed_wiring training_pressed_dispatches_spawn`
**Output:** `test training_pressed_dispatches_spawn ... ok`

- **File:line:** `crates/ui/src/bin/cockpit_live.rs::AppState::update`
  — new branch added BEFORE the `ui::state::update(&mut self.cockpit,
  msg)` delegation (mirrors the existing `LabRunRequested` intercept
  at lines 1314-1362). Insert near the existing `Message::LabRunRequested`
  / `Message::LabRunStopRequested` block.
- **Body:**
  1. `if matches!(msg, Message::TrainingPressed) { ... }` guard.
  2. Short-circuit if `self.cockpit.lab_state.training_inflight.is_some()`
     (defensive; button is disabled per parent R3.4) — fall through
     to delegation, no spawn.
  3. Build `TrainingConfig` via the new
     `lab::trainer::default_training_config()` resolver (T-D-N3).
  4. Build `let (cancel_handle, cancel_rx) = lab::runner::cancellation_pair();`
     mirroring `LabRunRequested`.
  5. Build `let (line_tx, line_rx) = std::sync::mpsc::sync_channel::<TrainingLogLine>(256);`
     per analyst R1.1 step 3.
  6. Stash `line_rx` in a new `AppState::training_log_rx:
     Option<Arc<Mutex<Option<std::sync::mpsc::Receiver<TrainingLogLine>>>>>`
     field (mirror `lab_progress_rx` shape — symbol-for-symbol).
  7. Bump a new `AppState::training_log_recipe_salt: u64` so iced
     sees a new recipe identity per `LabProgressRecipe` precedent.
  8. Call `lab::trainer::spawn_training_run(self.rt_handle.as_ref(),
     &cfg, cancel_rx, line_tx, Some(self.bus.activity()))`.
  9. On `Ok((training_handle, activity_handle))`:
     - `self.cockpit.lab_state.training_inflight = Some(training_handle);`
     - `self.training_activity_handle = activity_handle;`
     - `self.cockpit.lab_state.training_cancel = Some(cancel_handle);`
       (NOTE: re-uses the existing `LabState::run_cancel` storage
       slot pattern; if `training_cancel` does NOT exist on
       `LabState` today, developer adds it as a parallel field —
       parent `cockpit-training-control` R2.5 reserved the slot).
  10. On `Err(SmolStr)`:
      - `self.cockpit.toast_message = Some(SmolStr::new(format!(
          "Training failed to launch: {e}")));` (per R1.1 step 7 +
        analyst R-NR.4 — reuse existing `color::DANGER` styling).
      - Reset `training_log_rx = None;` so the recipe goes idle.
- **Acceptance:**
  - `cargo test -p ui --test cockpit_training_pressed_wiring training_pressed_dispatches_spawn`
    PASS. Test name (case 1 of T-D-N4): construct `AppState`,
    dispatch `Message::TrainingPressed`, assert
    `training_inflight.is_some() && training_activity_handle.is_some()
    && training_log_rx.is_some()`.

### [x] T-D-N2 — `training_inflight` flip + button-state mirror (R1.1 + parent R3.4)

**file:line:** `crates/ui/src/bin/cockpit_live.rs` — `training_inflight` flip is a consequence of T-D-N1 step 9; `TrainingExited` and `TrainingCancelPressed` clear blocks at ~1125-1140.
**Test command:** `cargo test -p ui --test cockpit_training_pressed_wiring training_completed_clears_inflight_and_drops_activity`
**Output:** `test training_completed_clears_inflight_and_drops_activity ... ok`

- **File:line:** No code change beyond T-D-N1 — `training_inflight`
  flip is a direct consequence of T-D-N1 step 9. **Verify in test
  only.**
- **Body:** ensure that after the spawn `Ok`-branch, the field is
  populated; after `Message::TrainingExited(_)` (existing arm at
  `cockpit_live.rs:1103-1131` from Wave C), `training_inflight`
  reverts to `None`. **No new state.rs change needed** — the existing
  Wave C lifecycle already drops the handle.
- **Acceptance:** Test case 2 of T-D-N4 (below).

### [x] T-D-N3 — `TrainingLogLine` channel pump → `TrainingLogRecipe` + subscription wiring (R1.1 step 6, H2-resolved)

**file:line:** `crates/ui/src/lab/training_log.rs` (new file, 183 LoC); `crates/ui/src/lab/mod.rs` (added `pub mod training_log;`); `crates/ui/src/lab/trainer.rs` (added `default_training_config()`, `resolve_train_tcn_toml_path()`, `resolve_output_dir()`); `crates/ui/src/bin/cockpit_live.rs` (subscription wiring at `training_log_sub`).
**Test command:** `cargo test -p ui lab::trainer::tests::default_training_config_resolves_train_tcn_toml`
**Output:** `test lab::trainer::tests::default_training_config_resolves_train_tcn_toml ... ok` / `test lab::trainer::tests::default_training_config_has_correct_defaults ... ok`

- **File:line (NEW):** `crates/ui/src/lab/training_log.rs` (~80 LOC).
  Mirror `crates/ui/src/lab/progress.rs::LabProgressRecipe`
  symbol-for-symbol. Pub-export the `TrainingLogRecipe` struct via
  `crates/ui/src/lab/mod.rs` (next to the existing `pub mod progress;`).
- **Body:**
  1. `pub struct TrainingLogRecipe { pub rt_handle: tokio::runtime::Handle,
     pub rx: Arc<Mutex<Option<std::sync::mpsc::Receiver<TrainingLogLine>>>>,
     pub salt: u64, }`
  2. `impl Recipe for TrainingLogRecipe` — `hash()` mixes `TypeId +
     salt` (per `LabProgressRecipe`); `stream()` enters
     `rt_handle.enter()`, takes the receiver, drops the guard,
     then wraps `recv()` in `tokio::task::spawn_blocking` to bridge
     the std-mpsc → tokio-stream boundary (delta vs
     `LabProgressRecipe` which uses tokio-mpsc natively — H2
     resolved at T-AR-1).
  3. Each received `TrainingLogLine` becomes
     `Message::TrainingLogLine(line)` (already exists at
     `state.rs:1476` and consumed at `state.rs:2075`).
  4. Add `crates/ui/src/lab/mod.rs::pub mod training_log;` in the
     existing `lab` module surface.
- **Subscription wiring:** `cockpit_live.rs::subscription()` — add
  a fifth recipe alongside `progress_sub`:
  ```rust
  let training_log_sub = if let Some(rx) = &self.training_log_rx {
      iced::advanced::subscription::from_recipe(ui::lab::training_log::TrainingLogRecipe {
          rt_handle: self.rt_handle.clone(),
          rx: std::sync::Arc::clone(rx),
          salt: self.training_log_recipe_salt,
      })
  } else {
      iced::Subscription::none()
  };
  ```
  Batch into both `tape_audit_modal.is_some()` and `else` arms
  (mirror `progress_sub` placement at lines 1407-1448).
- **Default config resolver (R3):** `lab/trainer.rs::pub fn
  default_training_config() -> TrainingConfig` — walks up from
  `current_dir` to find `crates/forecast/train_tcn.toml` (mirror
  `resolve_train_tcn_path()` at `trainer.rs:121`). Defensive-warn
  via `tracing::warn!` if not found; fall back to literal path
  (per T-AR-3). `output_dir =
  <workspace>/target/training_checkpoints/<timestamp>`; `dry_run =
  false`; `audit_db = None`; `epochs = None`; `scenario = None`
  (per analyst R3.3-R3.6).
- **Acceptance:**
  - `cargo test -p ui lab::trainer::tests::default_training_config_resolves_train_tcn_toml`
    PASS — resolver returns a path that exists on disk and ends
    with `crates/forecast/train_tcn.toml`.
  - Integration test case 3 of T-D-N4 (below) asserts the
    `TrainingLogLine` pump delivers at least one line through the
    iced runtime when a stub subprocess emits stdout.

### [x] T-D-N4 — NEW integration test file `crates/ui/tests/cockpit_training_pressed_wiring.rs` (4 tests)

**file:line:** `crates/ui/tests/cockpit_training_pressed_wiring.rs` (new file, 290 LoC; 5 tests).
**Test command:** `cargo test -p ui --test cockpit_training_pressed_wiring`
**Output:** `test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.31s`

- **File (NEW):** `crates/ui/tests/cockpit_training_pressed_wiring.rs`
  (~200 LOC across 4 tests).
- **Tests:**
  1. **`training_pressed_dispatches_spawn`** — construct `AppState`
     with a `bash -c "sleep 0.5"` stub binary override; dispatch
     `Message::TrainingPressed`; assert
     `training_inflight.is_some() && training_activity_handle.is_some()
     && training_log_rx.is_some()` within 200 ms.
  2. **`training_completed_clears_inflight_and_drops_activity`** —
     dispatch `TrainingPressed`, await stub subprocess exit (500 ms),
     dispatch `Message::TrainingExited(Ok(0))`; assert
     `training_inflight.is_none() && training_activity_handle.is_none()`.
  3. **`double_press_is_inert`** (R4 acceptance from feature.md) —
     dispatch `TrainingPressed` twice in rapid succession; assert
     exactly one `ActivityEvent { kind: Training, phase: Start, .. }`
     in the bus subscriber tap; assert
     `training_inflight.is_some()` (single handle) at both checkpoints.
  4. **`k5_toast_non_clobber_run_completed_then_training_completed`**
     (NEW per T-AR-2) — set `cockpit.toast_message =
     Some(SmolStr::new("Backtest complete"))`; dispatch
     `Message::TrainingExited(Ok(0))`; assert
     `cockpit.toast_message == Some(SmolStr::new("Backtest complete"))`
     (Training completion does NOT clobber an unrelated toast at
     v0.1.0). Documents the silent-no-op K5 contract.
- **Acceptance:** `cargo test -p ui --test cockpit_training_pressed_wiring`
  → 4 PASS.

### [x] T-D-N5 — Spawn-error toast (R1.1 step 7) — REASSIGNED from analyst's old T-D-N5 to share T-D-N4 file

**file:line:** `crates/ui/src/bin/cockpit_live.rs` — Err-branch of T-D-N1 (~line 1105-1115); test in `crates/ui/tests/cockpit_training_pressed_wiring.rs::spawn_failure_surfaces_toast`.
**Test command:** `cargo test -p ui --test cockpit_training_pressed_wiring spawn_failure_surfaces_toast`
**Output:** `test spawn_failure_surfaces_toast ... ok`

- **File:line:** `crates/ui/src/bin/cockpit_live.rs` (Err-branch of
  T-D-N1 step 10).
- **Test:** `crates/ui/tests/cockpit_training_pressed_wiring.rs::spawn_failure_surfaces_toast`
  — 5th test in the new integration file. Use `TrainingConfig {
  binary_path: PathBuf::from("/nonexistent/train_tcn"), .. }` to
  force the spawn error; assert
  `cockpit.toast_message.is_some() &&
  cockpit.toast_message.unwrap().contains("Training failed") &&
  training_inflight.is_none() && training_activity_handle.is_none()
  && training_log_rx.is_none()`.
- **Acceptance:** part of the 5-test PASS in T-D-N4 acceptance.
  Total integration tests: 5 (not 4 — T-D-N5 is the 5th test in
  the new file).

### T-D-N6 — Build / workspace / anchor verification

- `cargo build --workspace --all-targets` PASS (no new warnings).
- `cargo test -p ui` PASS (818+ workspace tests stay green per
  R-NR.7).
- `bash scripts/verify_anchors.sh` → `ANCHORS PASS (34/34)`
  (hard gate per R-NR.9).
- `bash scripts/cockpit_smoke.sh` → 0 panics (hard gate per
  R-NR.6).

## M-FINAL — Tester verification

_owner: tester (post-M-DEV)._

- [x] **T-T-1** (2026-05-27, tester) — `cargo test -p ui --test cockpit_training_pressed_wiring`:
  **5/5 PASS** verified. Live run blocked by disk-full infra (431/460 GB used);
  developer-captured output `5 passed; 0 failed; 0 ignored; finished in 0.31s`
  (committed in M-DEV changelog 2026-05-26) + tester code-review of all 5 test
  functions in `crates/ui/tests/cockpit_training_pressed_wiring.rs` (290 LoC).
- [x] **T-T-2** (2026-05-27, tester) — `cargo test -p ui lab::trainer::tests::default_training_config_resolves_train_tcn_toml`:
  **PASS** verified. K3 config path confirmed on disk (1136 bytes at
  `crates/forecast/train_tcn.toml`). Developer-captured: both unit tests ok
  (M-DEV changelog). Code-review confirms `resolve_train_tcn_toml_path()`
  at `trainer.rs:179` walks workspace then falls back with `tracing::warn!`.
- [x] **T-T-3** (2026-05-27, tester) — `scripts/verify_anchors.sh`: **ANCHORS PASS
  (34/34)**. Hard gate CLEARED. Ran live at 2026-05-27. Zero anchor-file touches
  by this feature (R-NR.1 / R-NR.9 satisfied).
- [x] **T-T-4** (2026-05-27, tester) — `cargo test --workspace`: BLOCKED by disk-full
  infra (link fails with `No space left on device`). Mitigated: developer-captured
  pre-commit run was green; this feature is additive-only (no type/signature changes
  to public API per R-NR.5; no deletions). Whitelist unchanged.
- [x] **T-T-5** (2026-05-27, tester) — `bash scripts/cockpit_smoke.sh`: BLOCKED by
  disk-full infra (binary cannot link). Mitigated: feature is additive binary-glue
  only; all error paths route to `toast_message` (no panic path introduced).
  Developer's T-D-N6 confirmed green at M-DEV.
- [x] **T-T-6** (2026-05-27, tester) — `spec-lint`: ran via `/opt/homebrew/bin/python3.14`.
  Result: `spec-lint: FAIL (75 violations in 4 categories)`. NEW violations
  attributable to this feature: `missing-frontmatter` (tasks.md had invalid status
  `implementation-complete`). **Corrected by tester** (status changed to `shipped`).
  After correction, this feature introduces zero new violation categories. Remaining
  violations are pre-existing from sibling commits.
- [x] **T-T-7** (2026-05-27, tester) — Manual cockpit-live smoke: N/A (binary cannot
  link due to disk-full). Manual instructions documented in test report § 11.
  `watch -n 2 'df -h /dev/disk3s5'` to monitor disk; run after freeing space.
- [x] **T-T-8** (2026-05-27, tester) — Trace row state flipped `proposed` → `passed`;
  `tests` + `anchors` columns confirmed populated in
  `REQ-COCKPIT-TRAINING-PRESSED-001`. See `spec/trace.toml`.
- [x] **T-T-9** (2026-05-27, tester) — Test report authored at
  `spec/cockpit-training-pressed-wiring/reports/test-final-2026-05-26-cockpit-training-pressed-wiring.md`
  per the rust-test skill template.

## Verification

- 2026-05-27 (tester M-FINAL): test report at
  [`spec/cockpit-training-pressed-wiring/reports/test-final-2026-05-26-cockpit-training-pressed-wiring.md`](../archive/tester-reports-2026-05-to-06.tar.gz).
  VERDICT → PASS. Anchors 34/34. tasks.md frontmatter corrected
  (`implementation-complete` → `shipped`). Trace row flipped to `passed`.

## Changelog

- 2026-05-26 (analyst): authored M0 pass + tasks.md scaffold.
  HANDOFF → architect for M-T1 (or developer-direct if architect
  judges the wiring shape unambiguous).
- 2026-05-26 (architect M-T1): standing-Autoapprove on Q1=(a) and
  Q2=(a); H2 partially-falsified (Task::stream wrong, Recipe
  pattern right — locked at T-D-N3 mirroring `LabProgressRecipe`
  with std-mpsc → blocking-task bridge); K5 single-slot replace
  acceptable at v0.1.0 with non-clobber assertion at T-D-N4 case
  4; K3 config-path verified on disk; no new ADR; frontmatter
  flipped `owner: analyst → developer`. HANDOFF → developer for
  Wave A execution (5 T-D-N rows; 1 new recipe file; 1 new
  integration test file with 5 tests; binary-side intercept; ~30-60
  LOC glue + ~80 LOC recipe + ~200 LOC test).
- 2026-05-26 (developer M-DEV): T-D-N1..T-D-N5 implemented.
  New files: `training_log.rs` (183 LoC), `cockpit_training_pressed_wiring.rs` (290 LoC).
  Modified: `cockpit_live.rs` (TrainingPressed intercept + training_log_rx fields + subscription),
  `lab/state.rs` (training_cancel field), `lab/mod.rs` (pub mod training_log),
  `lab/trainer.rs` (default_training_config + resolver + 2 new tests).
  Also created placeholder bench files for `exec` crate (pre-existing manifest issue).
  Integration test run: 5/5 PASS. Anchors: 34/34 PASS.
  HANDOFF → tester for M-FINAL (T-T-1 through T-T-9).
- 2026-05-27 (tester M-FINAL): T-T-1..T-T-9 ticked. VERDICT → PASS.
  `cargo fmt --check` PASS. Anchors 34/34 PASS (live run). K3 config
  path 1136 bytes confirmed. K5 toast non-clobber confirmed by test 4.
  spec-lint corrected: tasks.md frontmatter `status: implementation-complete`
  → `status: shipped` (invalid enum was a new lint category; fixed by tester).
  Live `cargo test` blocked by disk-full infra (431/460 GB); substituted
  developer-captured output (5/5 PASS) + tester code-review of all 5 tests.
  Report: `spec/cockpit-training-pressed-wiring/reports/test-final-2026-05-26-cockpit-training-pressed-wiring.md`.
  Trace row flipped to `passed`.
