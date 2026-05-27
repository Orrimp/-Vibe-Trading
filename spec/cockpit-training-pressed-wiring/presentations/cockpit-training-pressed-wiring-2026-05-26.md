---
slug: cockpit-training-pressed-wiring
mode: release
status: draft
audience: human-operator
updated: 2026-05-26
generated: 2026-05-26T00:00:00Z
predecessor: cockpit-activity-status-bar v0.1.0 (Wave C T-D-N9, 2026-05-26)
parent: cockpit-training-control v0.2.0
verdict_source: spec/cockpit-training-pressed-wiring/reports/test-final-2026-05-26-cockpit-training-pressed-wiring.md
---

# cockpit-training-pressed-wiring v0.1.0 — release

## TL;DR

The lab cockpit's **Run Training** button is no longer dead — pressing it now
launches the real `train_tcn` subprocess, fires a `Training` activity on the
status-bar tape, and streams per-line stdout/stderr into the Train sub-panel
log. Tester verdict **PASS**, anchors **34/34**, integration suite **5/5** (live
re-run by presenter at 2026-05-27 confirms the developer-captured `0.31s` PASS).

## What changed

- **New file: [`crates/ui/src/lab/training_log.rs`](../../../crates/ui/src/lab/training_log.rs)
  (183 LoC).** An iced `Recipe` that consumes the synchronous
  `std::sync::mpsc::Receiver<TrainingLogLine>` produced by `spawn_training_run`
  and emits each line into the iced update loop as
  `Message::TrainingLogLine(...)`. The blocking-to-async bridge is
  `tokio::task::spawn_blocking`. Gated on `#[cfg(feature = "live")]`.
- **Binary-side wiring: [`crates/ui/src/bin/cockpit_live.rs`](../../../crates/ui/src/bin/cockpit_live.rs)
  — `Message::TrainingPressed` intercept block** added before the
  `ui::state::update` delegation (mirrors the existing `LabRunRequested`
  precedent). Builds a `TrainingConfig` from the canonical
  `crates/forecast/train_tcn.toml`, creates the cancel + log channels, calls
  `lab::trainer::spawn_training_run`, stores the four resulting handles on
  `AppState` / `LabState`, and routes spawn errors to a `toast_message`.
- **Default config resolver: [`crates/ui/src/lab/trainer.rs`](../../../crates/ui/src/lab/trainer.rs)
  — new `default_training_config()`** plus
  `resolve_train_tcn_toml_path()` (workspace-walk + `tracing::warn!`
  fallback) and `resolve_output_dir()` (timestamped
  `target/training_checkpoints/`). Two new unit tests pin the resolver.
- **Integration suite: [`crates/ui/tests/cockpit_training_pressed_wiring.rs`](../../../crates/ui/tests/cockpit_training_pressed_wiring.rs)
  (387 LoC; 5 tests)** covering the spawn path, the activity-handle
  lifecycle, double-press inertness, K5 toast non-clobber, and the spawn-error
  toast surface.

Public type surface unchanged. No schema migration. No new bus channel. No
anchored file touched (R-NR.1 — verified by the live anchor gate below).

## Why it matters

The status-bar activity tape shipped at `cockpit-activity-status-bar v0.1.0`
two days ago was already wired to consume a `Training` activity — the operator
just never got one, because the producer side (the actual press-to-spawn
glue) had never been written. The Train button was rendered, the message
existed (`state.rs:1472`), the spawn helper existed (`trainer.rs:166`), the
storage slot existed (`AppState::training_activity_handle`), but the binary's
`update` wrapper had no `TrainingPressed` branch that called any of them.
Pressing Train was a documented no-op:

> _"`TrainingPressed` spawning is NOT yet wired in cockpit_live.rs … the
> activity handle is returned from `spawn_training_run` when the caller is
> ready to wire it."_ — `crates/ui/src/bin/cockpit_live.rs:1020-1025` (pre-ship).

That gap was operator-visible and three days fresh. This ship closes it. The
operator can now launch v2.5 training runs from the live cockpit, see them
on the global status bar, and watch the per-epoch log fill in real time.

## Architecture call-outs

### Recipe pattern (per ADR-0042 D3)

The activity-broadcast ADR locks the channel shape on the agent side
(`broadcast::Sender<ActivityEvent>`); the UI side is producer-by-producer.
For event-streams that need to land **inside** the iced update loop (not
just the broadcast bus), the project's accepted pattern is an
`iced::advanced::subscription::Recipe` holding an
`Arc<Mutex<Option<Receiver<T>>>>` — see
[`crates/ui/src/lab/progress.rs::LabProgressRecipe`](../../../crates/ui/src/lab/progress.rs)
(shipped at `lab-end-to-end-v2`).

`TrainingLogRecipe` is the second instance of that pattern. The only delta
from `LabProgressRecipe`: the underlying receiver is a `std::sync::mpsc`
channel (because `trainer::spawn_training_run` predates the tape and uses
sync channels internally), where `LabProgressRecipe` consumes a native
`tokio::sync::mpsc`. The architect's H2 hypothesis (initially "use
`iced::Task::stream`") was **partially falsified** at M-T1 — `Task::stream`
needs an `async Stream + Send`, which a blocking `std::sync::mpsc::Receiver`
isn't. The resolution is a one-line bridge: each `recv()` runs inside
`tokio::task::spawn_blocking`, which lifts the blocking call onto a
blocking-thread pool and yields a `Future` the async stream can await. No
intermediate shim thread needed.

```rust
// crates/ui/src/lab/training_log.rs:114
let result = tokio::task::spawn_blocking(move || {
    rx_clone.lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .recv()
}).await;
```

### Salt-bump per run

Iced subscriptions de-duplicate by `Hash`. The recipe's `hash()` mixes a
`salt: u64` that increments on every `TrainingPressed`, so iced sees each
run as a distinct subscription and re-invokes `stream()` on the new
receiver. Same pattern as `LabProgressRecipe`.

### K8 mitigation (Send-bound preservation)

`stream()` enters the tokio runtime via `rt_handle.enter()`, **takes** the
receiver out of the `Arc<Mutex<Option<_>>>`, and **drops the EnterGuard
before** `Box::pin(...)`. Without this dance, the `!Send` EnterGuard leaks
into the returned `BoxStream<'static, Message>` and iced refuses to compile.
This is the same K8 fix already applied in `LabProgressRecipe` and
`ServerTimeRecipe`.

## Live demo — fresh run by presenter (2026-05-27)

The tester's M-FINAL was technically blocked on disk-full infra (`/dev/disk3s5`
at 100%); the operator has since freed space (now 226 GB free / 48% used),
so the presenter re-ran the suite live before publishing this deck.

### Integration suite — 5/5 PASS

```
$ cargo test -p ui --test cockpit_training_pressed_wiring
    Finished `test` profile [unoptimized + debuginfo] target(s) in 3.13s
     Running tests/cockpit_training_pressed_wiring.rs

running 5 tests
test k5_toast_non_clobber_run_completed_then_training_completed ... ok
test spawn_failure_surfaces_toast ... ok
test training_pressed_dispatches_spawn ... ok
test double_press_is_inert ... ok
test training_completed_clears_inflight_and_drops_activity ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.31s
```

### Recipe unit tests — 2/2 PASS

```
$ cargo test -p ui --features live --lib lab::training_log
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.32s
     Running unittests src/lib.rs

running 2 tests
test lab::training_log::tests::stream_with_none_yields_nothing ... ok
test lab::training_log::tests::stream_yields_lines_and_terminates ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 391 filtered out; finished in 0.00s
```

### Trainer unit tests (config resolver + sibling) — 5/5 PASS

```
$ cargo test -p ui --features live --lib lab::trainer
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.87s
     Running unittests src/lib.rs

running 5 tests
test lab::trainer::tests::default_training_config_resolves_train_tcn_toml ... ok
test lab::trainer::tests::default_training_config_has_correct_defaults ... ok
test lab::trainer::tests::binary_missing_returns_err_sync ... ok
test lab::trainer::tests::stdout_lines_pipe_to_channel ... ok
test lab::trainer::tests::cancel_handle_drop_kills_child ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 388 filtered out; finished in 0.15s
```

### Anchor gate — 34/34 PASS

```
$ bash scripts/verify_anchors.sh
...
PASS  sharpe-comparison-vol-target-bs1-realbaseline  ff2b934961f8cea87c2e44953a746dba3f3b732c42a997c501bbcc3b989d95e9
---
ANCHORS PASS  (34 / 34)
```

Zero anchored files were touched by this feature (R-NR.1 — wiring is
binary-side only; the `train_tcn` subprocess source bytes are unchanged,
the recipe is new code).

## Verification matrix

| Req  | Status     | Evidence |
|------|------------|----------|
| R1 — `TrainingPressed` intercept in binary `update` | **VERIFIED** | `training_pressed_dispatches_spawn` PASS — asserts all four handles populated. |
| R2 — Activity handle plumbed so tape lights up on press | **VERIFIED** | Wave C lifecycle arms already consumed by Activity tape; `double_press_is_inert` proves exactly one `Start` event lands per press. |
| R3 — Default training config from `crates/forecast/train_tcn.toml` | **VERIFIED** | `default_training_config_resolves_train_tcn_toml` + `_has_correct_defaults` PASS. K3 config 1136 bytes on disk. |
| R4 — Cancellation semantics — button disabled, re-press inert | **VERIFIED** | `double_press_is_inert` PASS — second press short-circuits; bus sees exactly one `Start`. |
| R-NR.1 — 34 anchors byte-identical | **VERIFIED** | `ANCHORS PASS (34/34)` (live run above). |
| R-NR.2 — `training_events` audit schema unchanged | **VERIFIED** | `audit_db = None` default per R3.4; no migration, no new writer. |
| R-NR.3 — No bus channel changes | **VERIFIED** | Re-uses `bus.activity()` shipped at `cockpit-activity-status-bar` Wave A. |
| R-NR.4 — No new Lumen tokens | **VERIFIED** | Error toast re-uses existing `color::DANGER` styling. |
| R-NR.5 — No `state.rs` signature changes | **VERIFIED** | All glue lives in the binary (`cockpit_live.rs`) + new module (`lab/training_log.rs`) + new `LabState::training_cancel` field (private, additive). |
| R-NR.6 — `cockpit-smoke` 0 panics | **NOT RE-RUN** | All error paths route to `toast_message`; no panic path introduced. Smoke script not invoked by presenter (binary builds cleanly; no signal-of-regression). |
| R-NR.7 — 818+ workspace tests stay green | **PARTIALLY VERIFIED** | New tests are additive; `cargo test -p ui --lib` shows 391 filtered + 2 passing in the training_log suite. Full `cargo test --workspace` not re-run by presenter (deferred to next CI tick). |
| R-NR.8 — `spec-lint` introduces no new violation categories | **VERIFIED** | Live `spec-lint`: 70 violations / 3 categories. The previously-introduced `missing-frontmatter` for this feature's `tasks.md` was corrected by the tester (`implementation-complete` → `shipped`). The remaining `missing-frontmatter` on `lab-polish-round-2/tasks.md` is pre-existing and not attributable to this feature. |
| R-NR.9 — `verify_anchors.sh` exits 0 with 34/34 | **VERIFIED** | See live run above. |

## Numbers that matter

| Metric | Value |
|---|---:|
| New production LoC (recipe) | **183** |
| New test LoC (integration) | **387** |
| Modified files (production) | **4** (`cockpit_live.rs`, `lab/mod.rs`, `lab/state.rs`, `lab/trainer.rs`) |
| New files (production) | **1** (`lab/training_log.rs`) |
| New files (tests) | **1** (`cockpit_training_pressed_wiring.rs`) |
| Integration tests added | **5** (all PASS) |
| Recipe unit tests added | **2** (all PASS) |
| Trainer unit tests added | **2** (config resolver + defaults) |
| Anchored files touched | **0** |
| Workspace anchors PASS | **34 / 34** |
| Integration test wall-clock | **0.31 s** |
| `spec-lint` categories vs audit-2026-05-25 baseline (61/1) | **70/3** — Δ pre-existing from sibling commits; tester corrected the one feature-attributable violation |
| Predecessor → this gap | **3 days** (Wave C T-D-N9 landed 2026-05-26; this brief authored same-day) |

## What's next

- **`cockpit-activity-audit-ledger-producer`** — `status: draft` today; once
  it lands the activity tape will have the full producer trio of LLM-call /
  Training / AuditLedger events surfacing through one shared status-bar
  surface. This is the third and final producer specified at ADR-0042 R5.
- **`cockpit-toast-queue`** (follow-on) — surfaces today's deferred K5
  multi-toast question. Today, a Training spawn error overwrites any
  in-flight Backtest-completion toast; the queue would FIFO them. Not
  blocking ship; tracked at parent feature.md K5.
- **Optional manual smoke** — the operator can verify end-to-end if desired:
  ```bash
  cargo run -p ui --bin cockpit_live --features live
  # Click "Run Training" → observe (a) status-bar tape lights up with
  # "Train train_tcn · running", (b) per-epoch log lines fill the panel,
  # (c) the button greys out for the duration of the run.
  ```

## Open decisions

_n/a — both Q1 (default config = `crates/forecast/train_tcn.toml`) and Q2
(button disabled on re-press) were standing-Autoapprove defaults locked at
architect M-T1. The K5 multi-toast question is intentionally deferred to a
follow-on `cockpit-toast-queue` brief, not blocking ship._

## Approval

- [x] Approved — ship  _(2026-05-27, operator)_
- [ ] Approve with notes (notes below)
- [ ] Reject — _add reason below_

### Notes

_operator fills in here on approval-with-notes / reject_

## Verdict source

Tester report:
[`spec/cockpit-training-pressed-wiring/reports/test-final-2026-05-26-cockpit-training-pressed-wiring.md`](../reports/test-final-2026-05-26-cockpit-training-pressed-wiring.md)
— `VERDICT → PASS`, run 2026-05-27 10:15 UTC, commit `910fa0f`.
