---
slug: cockpit-activity-status-bar
status: in-progress
owner: developer
updated: 2026-05-25
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

- [ ] **T-D-N1** — New type module: `ActivityEvent`, `ActivityKind`,
  `ActivityPhase`, `ActivityOutcome`, `ActivityId`.
  - Owner: developer • Milestone: M-DEV • Depends on: T-AR-4 • Blocks: T-D-N2, T-D-N3, T-D-N4
  - File:line: `crates/agent/src/bus.rs` (extend) OR
    `crates/agent/src/activity.rs` (new file ~ 120 LOC — architect picks)
  - Body: derive `Debug + Clone` on `ActivityEvent`. `ActivityKind`
    derives `Debug + Clone + Copy + PartialEq + Eq + Hash`.
    `ActivityId(u64)` is `#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]`.
  - Test cmd: `cargo test -p agent --lib bus::activity_types`
  - Expected: `test result: ok. 3 passed; 0 failed`
- [ ] **T-D-N2** — `EventBus::activity_tx` field + `EventBus::activity(&self)`
  accessor.
  - Owner: developer • Milestone: M-DEV • Depends on: T-D-N1 • Blocks: T-D-N3, T-D-N6
  - File:line: `crates/agent/src/bus.rs` (extend struct + impl).
    Capacity 256 (R1.1). New `ActivitySender` thin wrapper around
    `broadcast::Sender<ActivityEvent>` with `.start(kind, label) ->
    ActivityHandle` factory.
  - Test cmd: `cargo test -p agent --lib bus::activity_channel_lag_drops_oldest`
  - Expected: `test result: ok. 1 passed; 0 failed`
- [ ] **T-D-N3** — `ActivityHandle` RAII type with `tick`, `fail`,
  `cancel`, `Drop`.
  - Owner: developer • Milestone: M-DEV • Depends on: T-D-N2 • Blocks: T-D-N7..N9
  - File:line: same module as T-D-N1. RAII shape: holds
    `broadcast::Sender<ActivityEvent>` + `ActivityId` + outcome
    `Cell<Option<ActivityOutcome>>` + last-tick `Cell<Instant>` for
    R1.4 throttle. Drop emits End with the recorded outcome OR
    `Success` by default OR `Failed("dropped")` on panic-unwind
    (use `std::thread::panicking()` inside Drop).
  - Test cmd: `cargo test -p agent --lib bus::activity_handle_drop_emits_end bus::activity_handle_throttle_caps_at_10_hz bus::activity_handle_drop_during_panic_emits_failed`
  - Expected: `test result: ok. 3 passed; 0 failed`

### Wave B — `crates/ui` tape state + subscription + widget (parallel with Wave C)

- [ ] **T-D-N4** — `ActivityTape` state struct + Message arms +
  update handlers.
  - Owner: developer • Milestone: M-DEV • Depends on: T-D-N3 • Blocks: T-D-N6
  - File:line: `crates/ui/src/state.rs` (extend `Cockpit`) +
    `crates/ui/src/lab/activity.rs` (new file ~ 150 LOC) per T-AR-7
    decision.
  - Body: per feature.md § R3. Update arms strictly O(1) for Start
    + Tick (with ≤ 32 in_flight ring); O(32) for the 1 Hz
    `ActivityTapePurgeTick`.
  - Test cmd: `cargo test -p ui --lib lab::activity::tests`
  - Expected: `test result: ok. 5 passed; 0 failed`
- [ ] **T-D-N5** — `ActivityRecipe` subscription in `crates/ui/src/live.rs`.
  - Owner: developer • Milestone: M-DEV • Depends on: T-D-N2 • Blocks: T-D-N7
  - File:line: `crates/ui/src/live.rs` (sibling of `BusRecipe`,
    `ServerTimeRecipe`).
  - Body: subscribes via `bus.activity()`; emits Message arms;
    handles `RecvError::Lagged(n)` with `tracing::warn` + continue;
    handles `RecvError::Closed` by ending the subscription
    (matches BusRecipe behaviour).
  - Test cmd: `cargo test -p ui --lib live::activity_recipe_emits_messages live::activity_recipe_handles_lag`
  - Expected: `test result: ok. 2 passed; 0 failed`
- [ ] **T-D-N6** — `widgets::activity_tape` rendering region (extracted
  per T-AR-6).
  - Owner: developer • Milestone: M-DEV • Depends on: T-D-N4 • Blocks: T-D-N7
  - File:line: `crates/ui/src/widgets/activity_tape.rs` (new file ~ 180 LOC).
  - Body: pure render function `fn view(&ActivityTape) -> Element`.
    Reads `Instant::now()` for elapsed display; applies R2.3
    200 ms render-floor; renders dot + label + elapsed; overflow
    chip; failure-state red colour. Zero string literals (R7.2);
    zero new Lumen tokens (R-NR.3).
  - Test cmd: `cargo test -p ui --lib widgets::activity_tape::tests`
  - Expected: `test result: ok. 4 passed; 0 failed`
  - Insta snapshots (4 per feature.md R2 acceptance):
    `status_bar__activity_tape_empty`,
    `status_bar__activity_tape_one_inflight`,
    `status_bar__activity_tape_three_plus_overflow`,
    `status_bar__activity_tape_failed_red`.

### Wave C — R4 producer wiring at 3 call sites (parallel with Wave B)

- [ ] **T-D-N7** — Yahoo preload producer wiring.
  - Owner: developer • Milestone: M-DEV • Depends on: T-D-N3 • Blocks: T-D-N10
  - File:line: `crates/ui/src/lab/runner.rs::preload_yahoo_bars`
    (and `spawn_lab_run` call site at lines ~582-610).
  - Body: 2-3 lines around the `preload_yahoo_bars` call: build label
    from `cfg_for_preload.symbol` + `range`; call
    `bus.activity().start(ActivityKind::YahooPreload, label)`; hold
    the handle until the preload returns; on `Err`, call
    `handle.fail(err.to_string())`.
  - Integration test:
    `crates/ui/tests/activity_tape_yahoo_preload.rs`.
  - Test cmd: `cargo test -p ui --test activity_tape_yahoo_preload`
  - Expected: `test result: ok. 1 passed; 0 failed`
- [ ] **T-D-N8** — Lab Run producer wiring.
  - Owner: developer • Milestone: M-DEV • Depends on: T-D-N3 • Blocks: T-D-N10
  - File:line: `crates/ui/src/lab/runner.rs::spawn_lab_run` (~ line 564
    `iced::Task::perform` closure).
  - Body: build label from `LabRunConfig.strategy_id` +
    `LabRunConfig.symbol` + `LabRunConfig.range`; start handle
    BEFORE the `async move {` closure (handle is `Send`); tick in
    the bar-loop alongside the existing `progress_tx.try_send`
    site (one new line); fail/cancel/success on the matching arm.
  - Integration test:
    `crates/ui/tests/activity_tape_lab_run.rs`.
  - Test cmd: `cargo test -p ui --test activity_tape_lab_run`
  - Expected: `test result: ok. 1 passed; 0 failed`
- [ ] **T-D-N9** — Training subprocess producer wiring.
  - Owner: developer • Milestone: M-DEV • Depends on: T-D-N3 • Blocks: T-D-N10
  - File:line: `crates/ui/src/lab/trainer.rs::spawn_training_run`.
  - Body: build label from training config; start handle before
    the subprocess spawn; tick on each audit-DB poll
    (per `cockpit-training-control` R7 — 1 Hz); end on subprocess
    exit / cancel / failure (the trainer already has clean exit
    branches per R9 of cockpit-training-control v0.2.0).
  - Integration test:
    `crates/ui/tests/activity_tape_training_run.rs` — spawns a
    `sleep 1` subprocess and asserts the handle's lifecycle.
  - Test cmd: `cargo test -p ui --test activity_tape_training_run`
  - Expected: `test result: ok. 1 passed; 0 failed`

### Wave D — Perf gates (depends on Wave C)

- [ ] **T-D-N10** — Criterion bench `crates/ui/benches/activity_tape.rs`
  (NEW). Per feature.md § D3 Layer 2.
  - Owner: developer • Milestone: M-DEV • Depends on: T-D-N7, T-D-N8, T-D-N9 • Blocks: T-D-N11
  - File:line: `crates/ui/benches/activity_tape.rs` (new file).
  - Body: 5 benches per feature.md § D3 Layer 2. Each bench prints
    its P99 result; tester records baseline at M-FINAL.
  - Test cmd: `cargo bench -p ui --bench activity_tape`
  - Expected: each bench under its budget per feature.md § D3
    Layer 2.
- [ ] **T-D-N11** — Integration perf test
  `crates/ui/tests/activity_tape_event_storm.rs` (NEW). Per
  feature.md § D3 Layer 3.
  - Owner: developer • Milestone: M-DEV • Depends on: T-D-N5 • Blocks: T-FINAL
  - File:line: `crates/ui/tests/activity_tape_event_storm.rs` (new).
  - Body: spawn synthetic 10 k Hz event stream; assert drain
    < 1 s wall-clock; assert ≥ 95 % delivery rate; assert P99
    end-to-end latency < 16 ms.
  - Test cmd: `cargo test -p ui --test activity_tape_event_storm`
  - Expected: `test result: ok. 1 passed; 0 failed`
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
- [ ] **T-T-5** — Author `spec/cockpit-activity-status-bar/reports/test-final-2026-MM-DD.md`
  with the standard 8-row template (verify_anchors / workspace /
  cockpit-smoke / clippy / fmt / criterion / integration perf /
  visual). VERDICT line per the rust-test SKILL template.
- [ ] **T-T-6** — Tester populates `tests` + `anchors` columns of
  trace row `REQ-COCKPIT-ACTIVITY-001` once VERDICT → PASS.

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
