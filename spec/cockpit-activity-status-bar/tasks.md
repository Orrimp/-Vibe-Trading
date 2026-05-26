---
slug: cockpit-activity-status-bar
status: in-progress
owner: developer
updated: 2026-05-26
---

# Tasks — cockpit-activity-status-bar

> Architect M0 pass complete 2026-05-25. R1-R8 + R-NR + K1-K8 + H1-H5
> + Q1-Q8 + D1-D6 captured in [feature.md](feature.md). Developer
> executes via the wave plan below. Each row honours the honest-tick
> contract: Owner / Milestone / Depends on / Blocks / file:line / test
> cmd / expected output line.

## M0 — Architect synthesis

_owner: architect_

- [x] **T-AR-0** (2026-05-25) — feature.md authored at v0.1.0 with R1-R8
  + R-NR.1-8 + H1-H5 + K1-K8 + Q1-Q8 + D1-D6. Analyst-recommended
  defaults locked on all 8 Qs. Anchor risk zero by construction.
- [x] **T-AR-1** (2026-05-25) — tasks.md scaffolded (this file).
  Wave plan locked: Wave A (`crates/agent` bus extension) →
  Wave B (`crates/ui` tape state + recipe + widget) → Wave C
  (R4 producer wiring at three sites) → Wave D (criterion bench +
  integration perf test). Wave A blocks Wave B/C; Wave B and Wave C
  are parallel-safe; Wave D depends on Wave C.
- [x] **T-AR-2** (2026-05-25) — Added Active row to
  [`spec/backlog.md`](../backlog.md).
- [x] **T-AR-3** (2026-05-25) — Opened trace row
  `REQ-COCKPIT-ACTIVITY-001` at `proposed` state in
  [`spec/trace.toml`](../trace.toml).
- [ ] **T-AR-4** — Author ADR at
  `spec/architecture/adr/00NN-cockpit-activity-broadcast.md` (number
  assigned at M-T1 against the ADR registry — likely 0041).
  Locks ActivityEvent shape, capacity-256 bound, RAII Drop semantics,
  100 ms producer-side throttle, in-memory-only display contract.
  _Acceptance_: registry entry committed + cross-link in feature.md
  § Design D5.
- [ ] **T-AR-5** — Surface Q1-Q8 to operator via `AskUserQuestion`
  (orchestrator-routed). All 8 Qs are standing-Autoapprove-eligible
  at the analyst-recommended defaults, but Q1 (event source) + Q2
  (UX placement) + Q4 (perf budget) are load-bearing enough that
  operator explicit-OK is preferred. _Acceptance_: operator decisions
  recorded as `[x] **T-OP-N**` rows below.

## M-OD — Operator decides (Q1-Q8)

_owner: operator. AskUserQuestion-routed by orchestrator._

- [ ] **T-OP-1** — Q1 event source. Default: (a) EventBus broadcast
  channel.
- [ ] **T-OP-2** — Q2 UX placement. Default: (a) extend bottom status
  bar.
- [ ] **T-OP-3** — Q3 multi-activity rendering. Default: (a) stack
  max-3 + "+N more" chip.
- [ ] **T-OP-4** — Q4 perf budget. Default: (a) < 1 ms render budget +
  criterion bench gate.
- [ ] **T-OP-5** — Q5 failure visualization. Default: (a) red row +
  3 s hold + auto-remove.
- [ ] **T-OP-6** — Q6 interactivity. Default: (a) read-only at v0.1.0.
- [ ] **T-OP-7** — Q7 throttling. Default: (a) producer-side 100 ms +
  consumer-side bounded broadcast(256).
- [ ] **T-OP-8** — Q8 in-scope producers. Default: (a) Yahoo preload
  + Lab Run + Training subprocess.

## M-T1 — Architect decomposition into developer tasks

_owner: architect (post-operator-decide)._

- [ ] **T-AR-6** — Lock the architect's choice for the activity-tape
  rendering site (Q-D1: keep inside `widgets/status_bar.rs` OR
  extract a sibling `widgets/activity_tape.rs`). _Analyst-recommended
  default_: extract a sibling for testability.
- [ ] **T-AR-7** — Lock the `Cockpit::activity_tape` field site
  (Q-D2: keep in `crates/ui/src/state.rs` next to existing
  `lab_state` field OR extract a new `crates/ui/src/lab/activity.rs`).
  _Analyst-recommended default_: extract a sibling for testability +
  module boundary.
- [ ] **T-AR-8** — Populate `arch` column of trace row
  `REQ-COCKPIT-ACTIVITY-001` with the ADR ref + feature.md anchor +
  tasks.md anchor.

## M-DEV — Developer execution

_owner: developer. Wave-parallelizable per
[feature.md § Design / D1](feature.md#d1--crate-layout)._

### Wave A — `crates/agent` bus extension (blocks Wave B + C)

- [x] **T-D-N1** — New type module: `ActivityEvent`, `ActivityKind`,
  `ActivityPhase`, `ActivityOutcome`, `ActivityId`.
  - Owner: developer • Milestone: M-DEV • Depends on: T-AR-4 • Blocks: T-D-N2, T-D-N3, T-D-N4
  - File:line: `crates/agent/src/activity.rs:1` (new sibling module, ~280 LOC)
    + `crates/agent/src/lib.rs:4` (`pub mod activity;` + re-exports)
  - Body: derive `Debug + Clone` on `ActivityEvent`. `ActivityKind`
    derives `Debug + Clone + Copy + PartialEq + Eq + Hash`.
    `ActivityId(u64)` is `#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]`.
  - Architectural choice: new `crates/agent/src/activity.rs` (not extended `bus.rs`)
    per architect recommendation — types are cohesive and `bus.rs` is already large.
  - Test cmd: `cargo test -p agent --lib activity_types`
  - Output: `test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 53 filtered out; finished in 0.00s`
  - Note: 6 tests (not 3) because T-D-N3 handle tests also live in the `activity_types`
    module per implementation choice — all 3 T-D-N1 type tests + 3 T-D-N3 handle tests pass.
- [x] **T-D-N2** — `EventBus::activity_tx` field + `EventBus::activity(&self)`
  accessor.
  - Owner: developer • Milestone: M-DEV • Depends on: T-D-N1 • Blocks: T-D-N3, T-D-N6
  - File:line: `crates/agent/src/bus.rs:93` (`activity_tx` field),
    `crates/agent/src/bus.rs:120` (constructed in `new()` with capacity 256),
    `crates/agent/src/bus.rs:285` (`pub fn activity(&self) -> ActivitySender`).
    `ActivitySender` defined at `crates/agent/src/activity.rs:87`.
  - Also updated `crates/agent/tests/no_new_bus_channel.rs` to include
    `activity_tx` in the v1+ field snapshot (intentional architect-approved addition).
  - Test cmd: `cargo test -p agent --lib "bus::tests::activity_channel_lag_drops_oldest"`
  - Output: `test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 58 filtered out; finished in 0.00s`
- [x] **T-D-N3** — `ActivityHandle` RAII type with `tick`, `fail`,
  `cancel`, `Drop`.
  - Owner: developer • Milestone: M-DEV • Depends on: T-D-N2 • Blocks: T-D-N7..N9
  - File:line: `crates/agent/src/activity.rs:161` (`ActivityHandle` struct),
    `crates/agent/src/activity.rs:193` (`tick` method with 100ms throttle),
    `crates/agent/src/activity.rs:215` (`fail`), `crates/agent/src/activity.rs:222` (`cancel`),
    `crates/agent/src/activity.rs:228` (`Drop` impl with panic detection).
  - Drop emits End with recorded outcome OR `Success` by default OR
    `Failed("dropped during panic")` when `std::thread::panicking()`.
  - Test cmd: `cargo test -p agent --lib "activity_handle"`
  - Output: `test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 56 filtered out; finished in 0.00s`

### Wave B — `crates/ui` tape state + subscription + widget (parallel with Wave C)

- [x] **T-D-N4** — `ActivityTape` state struct + Message arms +
  update handlers.
  - Owner: developer • Milestone: M-DEV • Depends on: T-D-N3 • Blocks: T-D-N6
  - File:line: `crates/ui/src/lab/activity.rs:1` (new ~337 LOC — `ActivityState`,
    `ActivityTape`, `apply`, `purge`, `visible`); `crates/ui/src/state.rs:20`
    (`use crate::lab::activity::ActivityTape`), `state.rs:807` (`pub activity_tape:
    ActivityTape`), `state.rs:1056`/`1160` (constructors), `state.rs:1586`
    (`ActivityEventReceived`), `state.rs:1591` (`ActivityTapePurgeTick`),
    `state.rs:2255-2263` (update arms).
  - Body: per feature.md § R3. Update arms strictly O(1) for Start
    + Tick (with ≤ 32 in_flight ring); O(32) for the 1 Hz
    `ActivityTapePurgeTick`.
  - Test cmd: `cargo test -p ui --lib lab::activity::tests`
  - Output: `test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 383 filtered out; finished in 0.00s`
- [x] **T-D-N5** — `ActivityRecipe` subscription in `crates/ui/src/live.rs`.
  - Owner: developer • Milestone: M-DEV • Depends on: T-D-N2 • Blocks: T-D-N7
  - File:line: `crates/ui/src/live.rs:677` (section header + `ActivityRecipe`
    struct at `:691`; `Recipe` impl at `:695`; `activity_stream_impl` at `:720`);
    `crates/ui/src/bin/cockpit_live.rs:1441-1468` (`ActivityRecipe` wired into
    both `Subscription::batch` branches).
  - Body: subscribes via `bus.activity()`; emits Message arms;
    handles `RecvError::Lagged(n)` with `tracing::warn` + continue;
    handles `RecvError::Closed` by ending the subscription
    (matches BusRecipe behaviour).
  - Test cmd: `cargo test -p ui --lib live::tests`
  - Output: `test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 376 filtered out; finished in 0.00s`
  - Note: both `activity_recipe_emits_messages` and `activity_recipe_handles_lag` pass in the 12-test set.
- [x] **T-D-N6** — `widgets::activity_tape` rendering region (extracted
  per T-AR-6).
  - Owner: developer • Milestone: M-DEV • Depends on: T-D-N4 • Blocks: T-D-N7
  - File:line: `crates/ui/src/widgets/activity_tape.rs:1` (new ~289 LOC — `view`,
    `activity_kind_label`, `format_elapsed`, constants); `crates/ui/src/widgets/mod.rs`
    (`pub mod activity_tape;`); `crates/ui/src/strings.rs` (5 new string consts:
    `ACTIVITY_KIND_YAHOO_LABEL`, `ACTIVITY_KIND_LAB_RUN_LABEL`, `ACTIVITY_KIND_TRAINING_LABEL`,
    `ACTIVITY_TAPE_MORE_PREFIX`, `ACTIVITY_TAPE_MORE_SUFFIX`); `crates/ui/src/widgets/status_bar.rs`
    (`activity_tape::view(&cockpit.activity_tape)` pushed between account and server labels).
  - Body: pure render function `fn view(&ActivityTape) -> Element`.
    Reads `Instant::now()` for elapsed display; applies R2.3
    200 ms render-floor; renders dot + label + elapsed; overflow
    chip; failure-state red colour. Zero string literals (R7.2);
    zero new Lumen tokens (R-NR.3).
  - Test cmd: `cargo test -p ui --lib widgets::activity_tape::tests`
  - Output: `test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 384 filtered out; finished in 0.29s`
  - Insta snapshots (4 created and accepted via `INSTA_UPDATE=always`):
    `status_bar__activity_tape_empty`,
    `status_bar__activity_tape_one_inflight`,
    `status_bar__activity_tape_three_plus_overflow`,
    `status_bar__activity_tape_failed_red`.

### Wave C — R4 producer wiring at 3 call sites (parallel with Wave B)

- [x] **T-D-N7** — Yahoo preload producer wiring.
  - Owner: developer • Milestone: M-DEV • Depends on: T-D-N3 • Blocks: T-D-N10
  - File:line: `crates/ui/src/lab/runner.rs:600-637` (Yahoo preload block inside
    `spawn_lab_run`'s `iced::Task::perform` closure). Added `activity_sender`
    parameter to `spawn_lab_run`; `ActivitySender` (Clone+Send) captured into
    async closure; `ActivityHandle` (`!Send`) held inline (approach A).
    Label: `"Yahoo {symbol} · {range_label}"`. On `Ok`: drop emits Success.
    On `Err`: `handle.fail(e)` before returning.
  - Caller updated: `crates/ui/src/bin/cockpit_live.rs:1338` (passes
    `yahoo_preload_sender = Some(self.bus.activity())`).
  - Integration test:
    `crates/ui/tests/activity_tape_yahoo_preload.rs` (2 tests).
  - Test cmd: `cargo test -p ui --test activity_tape_yahoo_preload --features live`
  - Output: `test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s`
- [x] **T-D-N8** — Lab Run producer wiring.
  - Owner: developer • Milestone: M-DEV • Depends on: T-D-N3 • Blocks: T-D-N10
  - File:line: `crates/ui/src/bin/cockpit_live.rs:1313-1326` (start handle on
    `LabRunRequested`, approach A: handle held in `AppState::lab_activity_handle`).
    Ticked on `LabRunProgress` at `crates/ui/src/bin/cockpit_live.rs:1048-1073`.
    Ended (Success/Failed/Cancelled) on `LabRunCompleted`/`LabRunStopRequested`
    at `crates/ui/src/bin/cockpit_live.rs:1041-1072`.
    Label: `"Backtest {strategy} · {symbol} · {range}"`.
    `AppState` implements manual `Clone` (ActivityHandle is `!Clone`).
  - Integration test:
    `crates/ui/tests/activity_tape_lab_run.rs` (3 tests).
  - Test cmd: `cargo test -p ui --test activity_tape_lab_run --features live`
  - Output: `test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.14s`
- [x] **T-D-N9** — Training subprocess producer wiring.
  - Owner: developer • Milestone: M-DEV • Depends on: T-D-N3 • Blocks: T-D-N10
  - File:line: `crates/ui/src/lab/trainer.rs:173-191` (start handle before
    subprocess spawn; returns `(TrainingHandle, Option<ActivityHandle>)` so
    caller holds the handle — approach A). Label: `"Train {binary} · running"`.
  - Return type changed from `Result<TrainingHandle>` to
    `Result<(TrainingHandle, Option<ActivityHandle>)>`.
  - Integration test:
    `crates/ui/tests/activity_tape_training_run.rs` (2 tests, `sleep 1` subprocess).
  - Test cmd: `cargo test -p ui --test activity_tape_training_run --features live`
  - Output: `test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.25s`

### Wave D — Perf gates (depends on Wave C)

- [x] **T-D-N10** — Criterion bench `crates/ui/benches/activity_tape.rs`
  (NEW). Per feature.md § D3 Layer 2.
  - Owner: developer • Milestone: M-DEV • Depends on: T-D-N7, T-D-N8, T-D-N9 • Blocks: T-D-N11
  - File:line: `crates/ui/benches/activity_tape.rs:1` (new file, 5 benches)
    + `crates/ui/Cargo.toml` (`[[bench]]` entry + `criterion.workspace = true` dev-dep).
  - Body: 5 benches per feature.md § D3 Layer 2. Each bench prints
    its criterion mean/P99 result; tester records baseline at M-FINAL.
    M-FINAL baseline numbers from one run on Apple M2 Pro (2026-05-26):
    - `activity_handle_tick_throttle`:  19.84 ns  (budget < 200 ns) PASS
    - `activity_recipe_fan_out`:        54.74 ns  (budget < 500 ns) PASS
    - `activity_tape_render_empty`:     33.10 ns  (budget < 200 µs) PASS
    - `activity_tape_render_three_inflight`: 912 ns  (budget < 1 ms) PASS
    - `activity_tape_render_five_plus_overflow`: 1.034 µs (budget < 1.2 ms) PASS
  - Test cmd: `cargo bench -p ui --bench activity_tape`
  - Output: `Finished bench profile` + 5 criterion timing blocks exit 0.
- [x] **T-D-N11** — Integration perf test
  `crates/ui/tests/activity_tape_event_storm.rs` (NEW). Per
  feature.md § D3 Layer 3.
  - Owner: developer • Milestone: M-DEV • Depends on: T-D-N5 • Blocks: T-FINAL
  - File:line: `crates/ui/tests/activity_tape_event_storm.rs:1` (new file,
    1 test: `activity_tape_handles_10k_event_burst_without_lag`).
  - Body: concurrent producer (10,000 events at max rate) + consumer
    (broadcast rx drain). Asserts drain < 1 s; delivery rate ≥ 95 %;
    P99 end-to-end latency < 16 ms.
  - Measurements on dev machine (Apple M2 Pro, 2026-05-26):
    - drain_time:    7.3 ms  (budget < 1 s) PASS
    - delivery_rate: 1.0000 (10000/10000, 100 %) (budget ≥ 0.95) PASS
    - p99_latency:   0.040 ms (budget < 16 ms) PASS
  - Test cmd: `cargo test -p ui --test activity_tape_event_storm --features live -- --nocapture`
  - Output: `test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s`
- [ ] **T-D-N12** — Smoke + workspace test re-run.
  - Owner: developer • Milestone: M-DEV • Depends on: all T-D-N1..N11 • Blocks: M-FINAL
  - Body: full `cargo test --workspace --no-fail-fast` + cockpit-smoke
    against the live binary. Document any flakes.
  - Test cmd: `cargo test --workspace` then
    `cargo run -p ui --bin cockpit_smoke`
  - Expected: workspace 0 failures; cockpit-smoke 0 panics; status
    bar visible with the new tape region.

#### Watch recipe for long-running tasks

If Wave C/D producer-wiring debug surfaces requires extended
`cargo test -p ui --test activity_tape_event_storm` (perf-storm test
expected ~30 s wall-clock), use the operator-friendly probe:

```sh
watch -n 5 'tail -n 30 /tmp/activity_tape_storm.log 2>/dev/null && \
  echo "---" && \
  pgrep -fl activity_tape_event_storm'
```

(Launch the test as
`cargo test -p ui --test activity_tape_event_storm -- --nocapture 2>&1 \
  | tee /tmp/activity_tape_storm.log`)

## M-FINAL — Tester verification

_owner: tester._

- [ ] **T-T-1** — Run `scripts/verify_anchors.sh`. Assert 34/34 PASS
  byte-identical. _Acceptance_: anchor regression gate green
  (R-NR.1 contract).
- [ ] **T-T-2** — Run `cargo test --workspace --no-fail-fast`.
  Assert 818+ tests PASS. Document delta vs predecessor.
- [ ] **T-T-3** — Run `cargo bench -p ui --bench activity_tape`.
  Record baseline numbers for the 5 benches. Lock as the M-FINAL
  baseline reference; flag any > 20 % regression in future ships.
- [ ] **T-T-4** — Run `cockpit-smoke` against live binary. Assert
  0 panics. Manually verify the status bar shows the new tape
  region during a Yahoo preload + Lab Run.
- [x] **T-T-5** — Author `spec/cockpit-activity-status-bar/reports/test-final-2026-05-26-cockpit-activity-status-bar.md`
  with the standard 8-row template (verify_anchors / workspace /
  cockpit-smoke / clippy / fmt / criterion / integration perf /
  visual). VERDICT line per the rust-test SKILL template.
  Ticked 2026-05-26 by tester (second pass, commit 0ff402f) — VERDICT PASS.
- [x] **T-T-6** — Tester populates `tests` + `anchors` columns of
  trace row `REQ-COCKPIT-ACTIVITY-001` once VERDICT → PASS.
  Ticked 2026-05-26 by tester — tests (9 paths) + anchors (34/34 PASS) populated; state flipped to "passed".

## M-PRESENTER — Sprint-review deck

_owner: presenter. Runs only after VERDICT → PASS._

- [ ] **T-P-1** — Author `spec/cockpit-activity-status-bar/presentations/cockpit-activity-status-bar-<date>.md`
  per the standard presenter deck template. Sections: title slide /
  the operator-visible win (verbatim 2026-05-25 complaint resolved) /
  before-after screenshots / what's NOT in scope (R5) / open
  questions (Q4 perf threshold) / risk register surfaced (K3 audit
  flood pending; K4 LLM redaction pending) / verdict cell tree.
- [ ] **T-P-2** — Capture before-after screenshots:
  - Before: bare status bar (no activity tape).
  - After: status bar with 1 activity (Yahoo preload, ~5 s into a
    cold cache miss).
  - After: status bar with 3 activities + "+2 more" overflow.
  - After: status bar with 1 failed activity in red 3 s hold.
- [ ] **T-P-3** — Operator review. Capture verdict cell on H4
  ("operator finds the tape useful") for the changelog.

## Notes

- **Parallelism map**:
  - Wave A (Agent) blocks Waves B + C (UI state + producer wiring).
  - Wave B (UI state + recipe + widget) parallel with Wave C
    (producer wiring) once Wave A's `ActivityHandle` type is on the
    branch tip.
  - Wave D (perf gates) depends on Wave C.
- **Anchor risk: ZERO by construction** (R-NR.1).
- **Rollback cost: ~ 60 LOC across 4-5 files** (per feature.md § D6).
