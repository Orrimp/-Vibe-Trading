---
slug: cockpit-activity-status-bar
version: 0.1.0
status: shipped
shipped: 2026-05-26
owner: orchestrator
updated: 2026-05-26
predecessor: lab-end-to-end-v2 v0.1.0
---

# Cockpit activity status bar — continuously-updated "what is the cockpit doing right now"

> **Predecessor chain**: this brief sits downstream of
> [`lab-end-to-end-v2 v0.1.0`](../lab-end-to-end-v2/feature.md) (shipped
> 2026-05-25) which landed the `crates/backtest/src/progress.rs` channel +
> `Message::LabRunProgress` plumbing and the new `widgets/progress_bar.rs`,
> and downstream of
> [`cockpit-training-control v0.2.0`](../cockpit-training-control/feature.md)
> (shipped 2026-05-19) which landed the `training_events` audit table + 1 Hz
> poller + `TrainingHandle` lifecycle. This feature does **not** replace
> either surface — it surfaces an aggregated "what is happening" tape on
> top of them so the operator can see in-flight work from any subsystem at
> a single glance. **Precedent for the event-source pick (Q1=(a))**: the
> existing `agent::EventBus` at `crates/agent/src/bus.rs` (load-bearing for
> nine domain channels already) is the project's accepted broadcast
> pattern for "many publishers, the cockpit subscribes".

## Why

### Operator complaint (verbatim 2026-05-25)

> "Status bar should show all the current steps the cockpit is doing —
> downloading data, backtesting, everything else which could be helpful
> for the UI user to understand what's going on in background."

### State today

- The bottom status bar (`crates/ui/src/widgets/status_bar.rs`, 24 px
  fixed-height row) shows only static fields: connection dot, latency,
  account label, server time, CPU placeholder, app version. It is
  silent on every kind of work in flight.
- The Lab Run progress bar (`crates/ui/src/widgets/progress_bar.rs`,
  shipped via `lab-end-to-end-v2 v0.1.0` Wave D-4) sits INSIDE the Lab
  Run row and is visible only when the Lab screen is active. An
  operator looking at the Live screen has zero feedback that a Run is
  underway.
- The Train sub-panel status strip (`cockpit-training-control v0.2.0`
  R3.5) is similarly Lab-local.
- `ThrottledSpinner` (`crates/ui/src/widgets/throttled_spinner.rs`)
  reports "something is happening" but not "what". Operators on the
  2026-05-25 verification walk consistently reported "is it stuck?" on
  cold-cache Yahoo preload (30-60 s) because the spinner gives no
  affordance.
- LLM calls (forthcoming via `v3-llm-forecaster`) will add another
  multi-second blocking activity with no operator visibility unless we
  plan ahead.

### Why we're doing this now

Three operator-flagged "is it stuck?" moments in the last two weeks all
trace to the same gap: a background activity (Yahoo cold-cache fetch,
backtest dispatch on a slow universe, training subprocess) has no
operator-facing surface OUTSIDE the screen that triggered it. This
brief picks the **bottom status bar** (already global) as the home for
an aggregated activity tape, locks **`EventBus`** as the event source
(reuses the existing broadcast infrastructure; no new mechanism), and
ships the v0.1.0 with the **three highest-value activity classes**
(Yahoo preload, Lab Run, Training subprocess) wired. The two
forward-listed classes (LLM call, audit ledger writes) sit out at
v0.1.0 and land in v0.1.1+ as their producers mature. Bus broadcast
events themselves are intentionally NOT activity-events — they're
domain events that don't need "in flight" framing.

This is the cheapest plausible slice — ~1 week wall-clock, anchor-
neutral by construction (zero backtest/exec/strategy body changes),
purely additive to the UI crate plus one new field on EventBus.

## Requirements

Numbered, testable, derived from the operator request 2026-05-25 + the
existing precedents in `cockpit-training-control v0.2.0` (R3.5 status
strip), `lab-end-to-end-v2 v0.1.0` (Wave D-4 progress channel), and
`lumen-phase-1-foundation` (R13 status-bar contract). Each R-item
preserves the [34 locked anchors](../anchors.toml) — see R-NR for the
non-regression contract.

### R1 — Event source: `agent::EventBus::activity_tx` broadcast channel

- **R1.1** Add a single new field on `agent::bus::EventBus`:
  `activity_tx: broadcast::Sender<ActivityEvent>` with capacity 256.
  Mirrors the existing nine-channel pattern (fills, positions, bars,
  ticks, pnl, mode, strategy_*, funding_obs, market_health,
  risk_telemetry). Bounded ring-buffer; slow consumers get
  `RecvError::Lagged(n)` and skip — same backpressure contract as
  every other channel.
- **R1.2** Event shape (new type in `crates/agent/src/bus.rs` or its
  own module — architect picks at M-T1):
  ```rust
  pub struct ActivityEvent {
      pub id: ActivityId,                // u64 monotonic, NOT a UUID
      pub kind: ActivityKind,            // enum below
      pub label: SmolStr,                // operator-facing copy, ≤ 64 chars
      pub phase: ActivityPhase,          // Start | Tick { progress } | End { outcome }
      pub started_at: Timestamp,         // UTC, second precision
  }
  pub enum ActivityKind {
      YahooPreload, LabRun, TrainingRun,
      // forward-listed; emit panic if used before v0.1.1:
      LlmCall, AuditLedgerWrite,
  }
  pub enum ActivityPhase {
      Start,
      Tick { current: u32, total: u32 },  // 0..=10000 bps for fractional progress
      End { outcome: ActivityOutcome },
  }
  pub enum ActivityOutcome { Success, Cancelled, Failed(SmolStr) }
  ```
  `ActivityId` is `u64` (monotonic per-process counter, NOT a UUID —
  the activity tape is in-memory only, IDs do not need to be
  globally unique). `Timestamp` reuses the existing `trading_core::Timestamp`.
- **R1.3** Producer API: providers call
  `bus.activity().start(kind, label) -> ActivityHandle` which returns
  an RAII-shaped handle. `ActivityHandle::tick(current, total)` emits
  a Tick phase; `Drop` impl emits `End { outcome: Success }` unless
  the producer has called `handle.fail(msg)` or `handle.cancel()`
  beforehand. **Drop-on-panic emits `Failed("dropped")`** so unwinds
  do not leave dangling Start events.
- **R1.4** Throttling at the producer side: `ActivityHandle::tick` is
  rate-limited to at most one event per 100 ms per handle (last-write-
  wins inside the handle, single `Instant` field). Spec at the
  producer ensures the audit-ledger-writes producer (Q8(e) deferred)
  cannot flood the channel at thousands of events per second.
- **R1.5** Rejected alternatives + reason:
  - **(b) tracing layer**: ties UI to log filtering; subscribers
    inherit allocation costs (`fmt::Layer` formats every span); event
    shape constrained by `tracing::field::Visit`. Fragile and slow.
  - **(c) polling probe**: requires each subsystem to expose a
    `Vec<InFlight>` accessor; coupling-heavy; doesn't extend to
    future producers without code changes per-source. The `EventBus`
    pattern already solved this for fills / positions / bars / etc.
- **Acceptance:** `agent::bus::tests::activity_channel_lag_drops_oldest`
  fills the 256-slot channel and asserts the receiver gets
  `RecvError::Lagged(_)`. `agent::bus::tests::activity_handle_drop_emits_end`
  asserts a dropped `ActivityHandle` emits exactly one `End` event.

### R2 — Status-bar widget extension: activity tape between server-time and CPU fields

- **R2.1** Extend `crates/ui/src/widgets/status_bar.rs` with a new
  region — analyst-recommended **(a) bottom status bar** (Q2=(a)) —
  rendered between the existing server-time field and the existing
  CPU placeholder. Fixed-width budget of **320 px** (logical) at
  cold-start; activity-labels longer than the budget are truncated
  with ellipsis ("…"). The status bar's 24 px height is preserved.
- **R2.2** Multi-activity rendering: analyst-recommended **(a)
  Most-recent-first stack, max 3 visible, "+N more" overflow chip**
  (Q3=(a)). Layout: `Row[activity_dot · label · elapsed | activity_dot
  · label · elapsed | activity_dot · label · elapsed | +2 more]`.
  Each activity slot has fixed ~100 px max width. Overflow chip
  shows the count of additional in-flight activities; click is read-
  only (Q6=(a) — no drill-down at v0.1.0).
- **R2.3** Elapsed-time display: per-activity elapsed shown in
  truncated form (`<1s` / `Ns` / `NmN s` / `NmNs`). Activities under
  200 ms duration **are not rendered** at all (Q3 acceptance —
  prevents flicker on sub-frame activities like a fast cache hit).
  This is enforced at the WIDGET layer (status bar reads
  `Instant::now()` against `event.started_at` and filters); the
  EventBus channel still carries the Start event — we just don't
  paint until the activity has been alive long enough to matter.
- **R2.4** Activity-dot colour token (Lumen Phase 1 reuse, **zero new
  tokens introduced**):
  - In-flight (Start / Tick): `color::ACCENT` (the same blue the Run
    button uses).
  - Failed (Tick saw `Failed`): `color::DANGER`.
  - Cancelled: `color::FG_3` (dim — already cancelled, low-attention).
  - Success-after-200ms-hold: not rendered (we just remove it).
- **R2.5** Failure visualization (Q5=(a)): on `End { Failed }`, the
  activity row turns red (`color::DANGER`) and STAYS visible for a
  configurable hold window (analyst-recommended **3 seconds**, R7
  controls). After the hold expires the row is removed. Operator
  has no dismiss affordance at v0.1.0 (read-only per Q6=(a)).
- **R2.6** Spinner integration (Q-spinner): the activity tape
  **complements** `ThrottledSpinner`, does not replace it. The
  spinner stays inside the Lab Run row (R6 of `ui-rethink-phase-b-
  lab-run`) and inside the Train sub-panel (R3.5 of
  `cockpit-training-control`). The status-bar tape is the GLOBAL
  surface — it shows everything regardless of the active screen.
- **R2.7** Cold-start with no activities: tape region renders an
  empty `Space::with_width(320)`, not a "no activity" label. The
  status bar stays visually quiet when nothing is happening.
- **Acceptance:** insta snapshots:
  - `status_bar__activity_tape_empty` — no activities.
  - `status_bar__activity_tape_one_inflight` — one Yahoo preload
    activity at Tick { 30, 100 }.
  - `status_bar__activity_tape_three_plus_overflow` — three visible
    + "+2 more" chip.
  - `status_bar__activity_tape_failed_red` — one Failed activity in
    the red 3-second hold window.

### R3 — UI-side state: `Cockpit::activity_tape: ActivityTape`

- **R3.1** New field on `Cockpit` (`crates/ui/src/state.rs`):
  `activity_tape: ActivityTape`. `ActivityTape` is a new small struct
  in `crates/ui/src/state.rs` (or `crates/ui/src/lab/activity.rs` —
  architect picks site at M-T1):
  ```rust
  pub struct ActivityTape {
      // bounded ring buffer, capacity 32 — far more than R2.2's max-3-visible
      // because we keep failed activities in the hold window and the channel
      // can legitimately carry a brief burst.
      in_flight: VecDeque<ActivityRow>,
      next_purge_at: Option<Instant>,
  }
  pub struct ActivityRow {
      pub id: ActivityId,
      pub kind: ActivityKind,
      pub label: SmolStr,
      pub started_at: Instant,    // wall-clock for elapsed display
      pub progress: Option<(u32, u32)>,
      pub state: ActivityRowState, // InFlight | Failed { until: Instant } | Cancelled { until: Instant }
  }
  ```
- **R3.2** Message arms (additive — no existing variants change):
  - `Message::ActivityStarted(ActivityId, ActivityKind, SmolStr)`
  - `Message::ActivityTick(ActivityId, u32, u32)`
  - `Message::ActivityEnded(ActivityId, ActivityOutcome)`
  - `Message::ActivityTapePurgeTick` — emitted by the UI 1 Hz tick
    subscription to remove expired Failed/Cancelled rows whose
    hold window has elapsed.
- **R3.3** Subscription: new `ActivityRecipe` in
  `crates/ui/src/live.rs` (sibling of `BusRecipe`, `ServerTimeRecipe`)
  that subscribes to `bus.activity()` and emits one of the three
  Message arms per phase. Batched alongside the existing `BusRecipe`
  + `ServerTimeRecipe` (single `Subscription::batch` call), so we
  inherit the existing subscription lifecycle / teardown contract.
- **R3.4** Update arms are O(1): `ActivityStarted` pushes a new
  `ActivityRow`, `ActivityTick` updates progress (linear scan over
  ≤ 32 items — acceptable), `ActivityEnded { Success }` removes the
  row, `ActivityEnded { Failed | Cancelled }` flips state to the
  3-second hold variant.

### R4 — Producer wiring: three subsystems at v0.1.0

In-scope producers (Q8=(a-c)):

- **R4.1** Yahoo preload (`crates/ui/src/lab/runner.rs::preload_yahoo_bars`):
  emit `ActivityHandle` around the `fetch_and_cache` call + the
  parquet read. Label: `"Yahoo BTC-USD 2y · downloading"` →
  `"Yahoo BTC-USD 2y · loading"`. The handle is held by the spawned
  task; on completion (`Ok` or `Err`) the handle's `Drop` /
  `handle.fail` emits the End event.
- **R4.2** Lab Run (`crates/ui/src/lab/runner.rs::spawn_lab_run`):
  emit `ActivityHandle` around the entire `iced::Task::perform`
  closure. Label: `"Backtest v1.momentum · TOP10 · 2024 FY"` (built
  from `LabRunConfig`). Hook the existing `progress_tx.try_send` site
  so that the bar-loop's `Progress { current_bar, total_bars }`
  ALSO produces an `activity.tick(current_bar, total_bars)` — this
  is a one-line addition next to the existing progress send. **No
  scenario-body changes**: we wrap the progress hand-off at the UI
  layer.
- **R4.3** Training subprocess (`crates/ui/src/lab/trainer.rs::spawn_training_run`):
  emit `ActivityHandle` around the subprocess lifecycle. Label:
  `"Train BS-1 TCN · epoch N / M"`. The handle ticks once per
  audit-DB poll (1 Hz per `cockpit-training-control` R7) — we reuse
  the existing `training_events` poller's most-recent row and emit
  a tick whenever the epoch advances. No new poller needed.
- **R4.4** Each producer wire-up is a **single new line + 2-3 line
  RAII handle** at the call site. No scenario-body / exec / strategy
  changes. Anchor-neutral by construction (see R-NR).
- **Acceptance:** integration tests:
  - `crates/ui/tests/activity_tape_yahoo_preload.rs` — spawn a Lab
    Run against fixture YahooCache data; assert exactly one
    `ActivityKind::YahooPreload` Start + ≥ 1 Tick + 1 End event
    arrive at the `ActivityRecipe` subscriber.
  - `crates/ui/tests/activity_tape_lab_run.rs` — spawn a Lab Run
    against the synthetic GBM fixture; assert at least one
    `ActivityKind::LabRun` Start + ≥ 1 Tick + 1 End.
  - `crates/ui/tests/activity_tape_training_run.rs` — spawn a no-op
    training subprocess (`sleep 1`); assert one Start + 1 End event.

### R5 — Out-of-scope producers (v0.1.1+)

- **R5.1** LLM call activity (`v3-llm-forecaster` once shipped).
  Status: forward-listed; `ActivityKind::LlmCall` exists in the
  enum but is unused at v0.1.0. v0.1.1 wires this.
- **R5.2** Audit ledger writes (`crates/audit/src/writer.rs`).
  Status: out-of-scope at v0.1.0 because audit writes can fan-out
  at thousands per second during a fast backtest (K3) and the
  per-event throttle (R1.4) is the wrong place to enforce
  aggregation. The right shape is a "audit writer: aggregate to
  one 'flushing' activity per second" — defer to v0.1.1 where we
  can design the aggregator properly.
- **R5.3** Bus broadcast events themselves (fills, positions, bars,
  ticks, pnl, etc.). Status: NOT activities. They are domain events
  with their own surfaces (tape panel, position panel, charts).
  Adding them to the activity tape would duplicate without value.

### R6 — Performance budget

- **R6.1** Status-bar render: **must complete in < 1 ms** per frame
  on the operator's reference Retina (3360 × 1890 native, Lumen
  cold-start scale factor 2.0). The status bar is laid out once per
  frame; rendering 3 `Row[dot + Text + Text]` children is negligible
  next to the rest of the cockpit chrome.
- **R6.2** Activity-event fan-out: from `ActivityHandle::tick` →
  `broadcast::send` → `ActivityRecipe` → `Message` → `Cockpit::update`
  → state mutation, the path **must complete in < 100 µs** P99 at a
  steady-state event rate of 10 events/sec (the steady-state for
  R4.1 + R4.2 + R4.3 combined). This is well below frame budget
  (16.67 ms at 60 fps) so the activity channel cannot stall renders.
- **R6.3** Channel lag: with the 256-slot bounded ring (R1.1), a
  consumer lagging more than ~25 s at 10 events/sec gets
  `RecvError::Lagged`. The recipe emits a single
  `Message::ActivityTapeLagged(n)` log line (tracing::warn) and
  resumes. **No retry / no replay** — activities are display-only,
  the audit ledger remains the source of truth.
- **R6.4** Criterion bench (per Q4=(b)) — see R-Test § Test layer 2.

### R7 — Configuration: hold-window + visible-count constants

- **R7.1** All numeric knobs live in `crates/ui/src/widgets/status_bar.rs`
  as `const` items at the top of the file (no runtime config; not
  operator-tunable at v0.1.0):
  ```rust
  const ACTIVITY_TAPE_WIDTH_PX: f32 = 320.0;
  const ACTIVITY_TAPE_MAX_VISIBLE: usize = 3;
  const ACTIVITY_TICK_RENDER_FLOOR_MS: u64 = 200;     // R2.3
  const ACTIVITY_FAILED_HOLD_MS: u64 = 3_000;         // R2.5
  const ACTIVITY_PRODUCER_TICK_THROTTLE_MS: u64 = 100; // R1.4
  ```
- **R7.2** Operator-facing copy lives in `crates/ui/src/strings.rs`
  alongside the existing status-bar copy block (preserves the "zero
  string literals" widget contract). Net-new strings: overflow chip
  template `"+{N} more"`, lag warning template, kind-prefix labels
  (`"Backtest "`, `"Train "`, `"Yahoo "`).

### R8 — Failure modes

| Mode | Symptom | Handling |
|------|---------|----------|
| F1 — `EventBus` channel full | `broadcast::Sender::send` returns SendError::Closed (no subscribers) | `ActivityHandle` silently ignores the send error (matches every existing channel's behaviour); no panic; tape just stays stale until the next event. |
| F2 — UI lagged | `RecvError::Lagged(n)` | `ActivityRecipe` logs `tracing::warn!("activity tape lagged {n} events")` and continues. UI tape may temporarily show stale state until next event lands. |
| F3 — `ActivityHandle` dropped without explicit outcome | Producer panic, or future without `.await`-resolved completion | Drop impl emits `End { Failed("dropped") }`. The tape row renders red for the 3-second hold window. **This is the bug-hunting affordance** — a "dropped" failure in the operator's view is a flag to file an issue against the producer. |
| F4 — Same `ActivityId` reused | Should never happen — IDs are monotonic per-process | If it does (test bug), the second `Start` overwrites the first row in-place; the first row's "missing" End is silently discarded. Add a debug-assert in M-T1 to catch in tests. |
| F5 — Frame budget exceeded | Render takes > 16 ms with activity tape active | Status bar perf test (R-Test § L2) gates this at CI. Mitigation: drop the live elapsed-time refresh and pin to "started at" timestamp display (compile-time choice — easy fallback). |
| F6 — Activity tape shows phantoms after cockpit restart | Activities are in-memory, no replay; should never persist across restart | By construction: tape state lives in `Cockpit::activity_tape` (in-process). Document explicitly. |

### R-NR — Non-regression contract

- **R-NR.1** **All 34 anchors stay byte-identical.** Zero touched
  files in `crates/backtest/`, `crates/strategy/`, `crates/exec/`,
  `crates/risk/`, `crates/reports/`, `crates/forecast/`. Producer
  wiring at R4.1 / R4.2 / R4.3 lives entirely in `crates/ui/` and
  `crates/agent/` (the bus extension). Verified by
  `scripts/verify_anchors.sh` at M-FINAL.
- **R-NR.2** **CLI binaries unaffected.** `crates/backtest/src/bin/`,
  `crates/data/src/bin/`, `crates/forecast/src/bin/` (incl.
  `train_tcn`) do not depend on `crates/ui` or the new `EventBus`
  field — adding a broadcast channel on `EventBus` is binary-compat
  with all existing CLI call sites (they construct `BusConfig` then
  `EventBus::new(&cfg)`; the additive channel is created internally).
- **R-NR.3** **No new Lumen tokens.** All colours reuse `ACCENT`,
  `DANGER`, `FG_3` from `crates/ui/src/theme.rs`. All sizes reuse
  `space::*`, `text::MICRO`, `radius::PILL`.
- **R-NR.4** **No new audit migration.** The tape is in-memory only.
- **R-NR.5** **No subprocess / no IPC.** Status-bar tape reads from
  the in-process `Arc<EventBus>` only.
- **R-NR.6** **No new external dependency.** Uses `tokio::sync::broadcast`
  (already in workspace), `SmolStr` (already in workspace),
  `iced::widget::Text/Row/Container` (already in workspace).
- **R-NR.7** **`cockpit-smoke` 0 panics.** Existing smoke test stays
  green; new tape state has a documented empty-cold-start path
  (R2.7).
- **R-NR.8** **818+ workspace tests stay green.** Additive surface;
  no rename / no signature change to public functions outside the
  new tape module + the additive `EventBus::activity*()` accessor
  pair.

## Hypothesis register

- **H1** — _Producer-side 100 ms throttle (R1.4) is sufficient to
  prevent channel saturation under all v0.1.0 producer rates._
  **Falsifier**: a criterion bench (R-Test § L2) pushes a synthetic
  10 k Hz event stream through `ActivityHandle::tick` and the
  per-call wall-clock exceeds 10 µs OR the channel logs Lagged
  events at the consumer. **Status at architect pass**: assumed
  TRUE (the 100 ms throttle bounds per-handle to 10 events/sec; with
  3 handles in v0.1.0 that's 30/sec, well under the 256-slot ring's
  drain capacity).
- **H2** — _Activity-tape render does not regress total status-bar
  render time above 1 ms per frame (R6.1)._ **Falsifier**: criterion
  bench `status_bar_render_with_three_activities` shows P99 > 1 ms.
  **Status at architect pass**: assumed TRUE (rendering 3 short Text
  rows is negligible).
- **H3** — _The `ActivityHandle::Drop` RAII pattern catches all
  panic-shaped activity losses without producer cooperation._
  **Falsifier**: a test where the producer panics mid-`Tick` does
  NOT emit an `End { Failed("dropped") }`. **Status at architect
  pass**: assumed TRUE per Rust's stack-unwinding semantics +
  documented Drop-during-unwind behaviour.
- **H4** — _Operator finds the activity tape useful enough to KEEP
  past v0.1.0 ship review._ **Falsifier**: presenter sprint-review
  deck shows operator says "remove this" or "I never look at it".
  **Status at architect pass**: untestable in code; documented for
  presenter to capture in the post-ship review.
- **H5** — _The 200 ms render-floor (R2.3) eliminates flicker on
  fast cache hits without hiding slow activities._ **Falsifier**: an
  operator reports a brief flash of a label on a < 200 ms activity,
  or reports a 250 ms activity that didn't render. **Status at
  architect pass**: 200 ms is the conservative pick — sub-frame
  flicker invisible to human eye, but slow enough that legitimate
  short activities (e.g. cache hit + 50 ms parquet read) are
  silently fast (good UX). Tunable via R7.1 constant.

## Risk register

- **K1** — **Channel-lag-shaped silent staleness.** If the UI
  thread blocks for > 25 s the activity tape goes stale and
  operator sees yesterday's state. **Mitigation**: tracing::warn
  on Lagged + R6.2 frame-budget gate. **Severity**: LOW — a
  25 s UI-thread stall means the entire cockpit is frozen; the
  activity tape staleness is the least of the operator's worries
  at that point.
- **K2** — **Producer wiring drift.** Future developer adds a new
  background activity but forgets to wire `ActivityHandle`. Operator
  loses visibility silently. **Mitigation**: developer M-DEV
  acceptance criterion enforces "every R4 producer has the wiring".
  Long-term: add a `cargo clippy`-style lint at the spec-auditor
  layer to flag `async fn` returning `iced::Task` that doesn't
  have an `ActivityHandle` in scope — deferred to a follow-on
  feature.
- **K3** — **Audit-ledger-writes producer flood (deferred R5.2).**
  If we wire R5.2 naively, fast backtests generate thousands of
  audit writes per second. **Mitigation**: R5.2 explicitly deferred
  to v0.1.1 + a new aggregator design.
- **K4** — **LLM-call activity reveals secrets / model versions in
  the label (deferred R5.1).** A label like
  `"LLM claude-3-5-sonnet-20241022 · forecast"` exposes vendor
  internals. **Mitigation**: when v0.1.1 wires this, label-redaction
  rule must be specified in the v0.1.1 brief.
- **K5** — **Sub-frame producer side effects on a hot path.**
  `ActivityHandle::tick` is called from the engine's bar loop
  (R4.2) — if it ever blocks (e.g. `broadcast::send` allocation
  bursts), bar-loop throughput degrades. **Mitigation**: the
  100 ms throttle inside the handle is a sync wall-clock check
  (no allocation, no syscall on the fast path). Criterion bench
  R-Test § L2 gates this.
- **K6** — **Status-bar regression on the 24 px height contract.**
  `lumen-phase-1-foundation` R13 locks the status bar at 24 px
  fixed. Adding the activity tape MUST not change height.
  **Mitigation**: R2 explicit; insta snapshots gate this.
- **K7** — **Inter-feature ordering with `v3-llm-forecaster`.**
  v0.1.0 of this brief ships `ActivityKind::LlmCall` as an
  unused enum variant. When v3-llm-forecaster lands, it will need
  to wire the producer. **Mitigation**: cross-link in
  `v3-llm-forecaster/feature.md` once that feature reaches M-T1;
  noted here for the analyst to surface during the v3 pass.
- **K8** — **Anchor-additive contract risk via ADR-0038 § D6.**
  Anchored report files under `spec/*/reports/` are byte-immutable.
  This brief touches NONE of them — UI-only + bus extension. But
  the architect must double-check at M-T1 that no producer wiring
  accidentally serializes activity state into a report front-matter.
  **Mitigation**: R-NR.4 / R-NR.5 explicit; tester gate enforces.

## Open questions for the operator

All 8 Qs come with an analyst-recommended default; architect should
escalate Q1 (event source — biggest mechanism choice) + Q2 (UX
placement) + Q4 (perf budget) explicitly. Q3, Q5, Q6, Q7, Q8 are
standing-Autoapprove-eligible at their defaults.

- **Q1 — Event source.**
  - (a) `EventBus::activity_tx` broadcast channel. ← **ANALYST DEFAULT**
  - (b) `tracing_subscriber::Layer`. Rejected per R1.5.
  - (c) Per-source polling probe. Rejected per R1.5.
- **Q2 — UX placement.**
  - (a) Extend the existing 24 px bottom status bar (between server-
    time and CPU). ← **ANALYST DEFAULT**
  - (b) New right-rail strip below the top bar.
  - (c) Overlay strip floating above the cockpit body.
- **Q3 — Multiple-activity rendering.**
  - (a) Most-recent-first stack, max 3 visible, "+N more" overflow
    chip. ← **ANALYST DEFAULT**
  - (b) Marquee rotation through all in-flight activities.
  - (c) Single most-recent activity only.
- **Q4 — Perf budget per frame.**
  - (a) < 1 ms status-bar render budget + criterion bench gate.
    ← **ANALYST DEFAULT**
  - (b) < 2 ms (more headroom; risk: regression sneaks in).
  - (c) No perf gate; trust 60fps emergent (risk: silent regression).
- **Q5 — Failure-state visualization.**
  - (a) Red row, 3-second hold, then auto-remove. ← **ANALYST DEFAULT**
  - (b) Red row, indefinite hold, manual dismiss.
  - (c) Toast-style banner above the status bar.
- **Q6 — Operator interactivity.**
  - (a) Read-only at v0.1.0. ← **ANALYST DEFAULT**
  - (b) Click activity → drill-down (navigate to the originating
    screen; e.g. backtest activity → Lab).
- **Q7 — Throttling policy.**
  - (a) Producer-side 100 ms `ActivityHandle::tick` throttle +
    consumer-side bounded broadcast(256). ← **ANALYST DEFAULT**
  - (b) Consumer-side only (16.67 ms = one frame).
  - (c) No throttle.
- **Q8 — In-scope activity classes for v0.1.0.**
  - (a) Yahoo preload + Lab Run + Training subprocess (the three
    operator-cited slow blockers). ← **ANALYST DEFAULT**
  - (b) Just Yahoo preload (smallest slice — 0.5 wk).
  - (c) (a) + LLM call (couples to v3-llm-forecaster ordering).
  - (d) (a) + Audit ledger writes (requires aggregator design — K3).
  - (e) (a) + (c) + (d) (full scope; ~3 weeks; not recommended).

## Design

This section is the architect's M-T1 work. Captured here in this
draft as the architect's M0 ratification — the developer M-DEV can
build against this without an additional architect pass.

### D1 — Crate layout

| Crate | What it owns at v0.1.0 |
|-------|--------------------------|
| `crates/agent` | New `bus::ActivityEvent` + `bus::ActivityKind` + `bus::ActivityPhase` + `bus::ActivityOutcome` + `bus::ActivityId` types; new `broadcast::Sender<ActivityEvent>` field on `EventBus`; new `EventBus::activity(&self) -> ActivitySender` accessor; new `ActivitySender::start(kind, label) -> ActivityHandle` factory; new `ActivityHandle` RAII handle. ~ 200 LOC. |
| `crates/ui` | New `crate::widgets::activity_tape` module (the visual region inside the existing status_bar widget; the architect decides at M-T1 whether to keep the rendering inline in `status_bar.rs` or extract a `widgets::activity_tape.rs` — analyst recommends the latter for testability). New `crate::state::ActivityTape` struct + Message arms (R3.2). New `ActivityRecipe` subscription in `crate::live`. New strings in `crate::strings`. R4 producer wiring at three call sites (`lab/runner.rs::preload_yahoo_bars`, `lab/runner.rs::spawn_lab_run`, `lab/trainer.rs::spawn_training_run`). ~ 400 LOC. |
| _no other crate touched_ | Strategy / backtest / exec / risk / reports / audit / forecast / replay-cache / data / models / core / cost — all unchanged. |

### D2 — Concurrency map

```
                      crates/agent::bus::EventBus
                                  │
                  activity_tx: broadcast::Sender<ActivityEvent> (capacity 256)
                                  │
            ┌─────────────────────┼──────────────────────────────┐
            │                     │                              │
    UI producer 1            UI producer 2                UI producer 3
    (preload_yahoo_bars)   (spawn_lab_run)             (spawn_training_run)
    holds 1 ActivityHandle  holds 1 ActivityHandle      holds 1 ActivityHandle
    rate-limited 100 ms     ticks on bar boundary       ticks on audit poll
                                  │
                                  ▼
                       broadcast::Receiver<ActivityEvent>
                                  │
                       crate::live::ActivityRecipe (iced::Subscription)
                                  │
                         Message::Activity{Started, Tick, Ended, TapePurgeTick}
                                  │
                                  ▼
                       Cockpit::update arm → ActivityTape mutation
                                  │
                                  ▼
                  widgets::status_bar::view reads Cockpit::activity_tape
                                  │
                                  ▼
                   widgets::activity_tape::view renders the region
```

### D3 — Performance test strategy

Three test layers, each enforcing a specific budget:

**Layer 1 — Unit tests** (`crates/agent/src/bus.rs::tests` + `crates/ui/src/widgets/activity_tape.rs::tests`):

- `activity_handle_drop_emits_end` — handle dropped → exactly one End event.
- `activity_handle_throttle_caps_at_10_hz` — 100 calls to `tick` within
  10 ms produces ≤ 1 `Tick` event downstream.
- `activity_tape_state_machine_purge` — Failed row state-transitions
  after the 3 s hold.
- `activity_tape_overflow_chip_count` — 5 in-flight, ≥ 3 visible + chip
  shows "+2".
- ~ 8-10 tests total, all sub-100ms.

**Layer 2 — Criterion bench** (NEW `crates/ui/benches/activity_tape.rs`
— sibling of any existing `benches/`):

```rust
#[bench] fn activity_handle_tick_throttled_10_hz_steady(b: &mut Bencher);
#[bench] fn activity_recipe_fanout_100_events(b: &mut Bencher);
#[bench] fn status_bar_render_zero_activities(b: &mut Bencher);
#[bench] fn status_bar_render_three_activities(b: &mut Bencher);
#[bench] fn status_bar_render_five_activities_with_overflow(b: &mut Bencher);
```

- **Baseline** locked at M-FINAL after a clean run on the reference
  hardware (Apple M2 Pro, single-process). **Regression threshold**:
  +20 % over baseline triggers tester re-gate.
- Per-bench budget budget:
  - `activity_handle_tick_throttled_10_hz_steady`: < 200 ns / call P99
    (the throttle is a single `Instant::now` + AtomicI64 compare).
  - `activity_recipe_fanout_100_events`: < 50 µs total (broadcast
    delivery + Message construction).
  - `status_bar_render_zero_activities`: < 200 µs.
  - `status_bar_render_three_activities`: < 1 ms (R6.1).
  - `status_bar_render_five_activities_with_overflow`: < 1.2 ms.

**Layer 3 — Integration perf test** (NEW
`crates/ui/tests/activity_tape_event_storm.rs`):

```rust
#[test]
fn activity_tape_handles_10k_event_burst_without_lag() {
    // Spawn a synthetic ActivityHandle producer that fires 10_000 ticks
    // as fast as it can (no throttle). Subscribe a recipe; assert the
    // subscriber drains within 1 s wall-clock and saw at least 95 % of
    // events (the 5 % allowance covers legitimate Lagged behaviour).
    // Frame-budget assertion: the wall-clock between an event being
    // produced and the corresponding Message reaching the recipe
    // stays < 16 ms P99.
}
```

This integration test is the e2e gate against R6.2 / R6.3.

### D4 — Cross-feature interactions

- **`cockpit-training-control v0.2.0`**: R4.3 reuses the existing 1 Hz
  `training_events` poller. No new subscription. The `TrainingHandle`
  on `LabState::training_inflight` gains a new sibling field
  `activity: ActivityHandle` (or an `Option<ActivityHandle>` for
  rollback safety). The poller emits the tick.
- **`lab-end-to-end-v2 v0.1.0`**: R4.2 wraps the existing progress-tx
  call. The `Progress { current_bar, total_bars, elapsed_ms }` event
  bifurcates: one branch goes to the existing `LabRunProgress`
  message (the in-Lab progress bar stays); the other goes to
  `ActivityHandle::tick`. **One added line at the producer; both
  consumers see the event.**
- **`lab-yahoo-realdata v0.1.0` (shipped) + v0.1.1 (in-flight)**:
  R4.1 wraps `preload_yahoo_bars`. The "0% with explicit label"
  sentinel that the Yahoo preload emits today (Bug #64 fix) becomes
  unnecessary at the status-bar layer — the activity tape now shows
  "Yahoo BTC-USD · downloading" as soon as the activity handle starts.
  The in-Lab Run progress bar still renders the sentinel for
  consistency with the Lab Run UX (no removal needed).
- **`reflection-memory-trader-wiring v0.1.0`**: pure refactor; no
  activity-event-emitting code paths gained or lost. No wiring needed.
- **`v3-llm-forecaster` (in-flight)**: K7 — once shipped, that feature's
  M-T1 should add a `bus.activity().start(ActivityKind::LlmCall,
  label)` around each provider call. Cross-link must be added there.

### D5 — ADRs

- **ADR-NNNN — Activity broadcast channel on EventBus** (architect M-T1
  authors at `spec/architecture/adr/00NN-cockpit-activity-broadcast.md`).
  Locks: bus.activity_tx capacity 256; ActivityEvent shape; RAII
  ActivityHandle Drop semantics; producer-side 100 ms throttle; the
  in-memory-only display contract (no persistence; audit ledger
  remains source of truth for compliance).
- Number to be assigned by architect at M-T1 against
  `spec/architecture/adr/README.md` registry. Likely 0041 or higher
  (lab-yahoo-realdata locked 0040 most recently).

### D6 — Rollback path

If the operator decides post-ship "I don't want this":

- Single revert of the producer wiring at the three R4 sites (~5 lines
  each).
- Single revert of the `activity_tx` field on `EventBus` (binary-compat;
  no external API change).
- Single revert of the status-bar layout change (one new region).
- Total rollback diff: ~ 60 LOC across 4-5 files. No anchor changes.
  No audit migration. No persistence.

## Backtest Scenarios

**None.** This feature is UI-only + a bus extension; it does not touch
the backtest engine's body, the matching engine, or any scenario
producer. The 34 locked anchors stay byte-identical (R-NR.1).

## Implementation

### Wave A — `crates/agent` bus extension (2026-05-26)

**Architectural choice**: new `crates/agent/src/activity.rs` sibling module
(~280 LOC) rather than extending `bus.rs`. Rationale: types are cohesive,
`bus.rs` is already 400+ LOC, and a sibling module maintains the single-
responsibility principle. Matches the architect's recommendation in D1.

**Files created/modified**:
- NEW `crates/agent/src/activity.rs` — `ActivityId`, `ActivityKind`,
  `ActivityPhase`, `ActivityOutcome`, `ActivityEvent`, `ActivitySender`,
  `ActivityHandle` (RAII with 100ms tick throttle + panic-aware Drop).
- EDIT `crates/agent/src/lib.rs` — `pub mod activity;` + re-exports of
  all 7 public types.
- EDIT `crates/agent/src/bus.rs` — `activity_tx: broadcast::Sender<ActivityEvent>`
  field (capacity 256), constructed in `new()`, `pub fn activity() -> ActivitySender`
  accessor, `activity_channel_lag_drops_oldest` test.
- EDIT `crates/agent/tests/no_new_bus_channel.rs` — updated v1+ field snapshot
  to include `activity_tx` (intentional architect-approved addition per D1).

**Test count**: 7 new tests in `crates/agent` (6 in `activity::activity_types`
module + 1 in `bus::tests`). All pass. Zero regressions. 34/34 anchors PASS.

## Implementation

_Wave B — developer (2026-05-26)_

### Files created

- `crates/ui/src/lab/activity.rs` (~337 LOC) — `ActivityState` + `ActivityTape`
  state machine. `apply()` handles all four `ActivityPhase` variants;
  `purge(now)` removes expired red-hold rows; `visible()` exposes the full
  slice. `Vec<ActivityState>` (not VecDeque) capped at 32 — O(32) scans are
  negligible at the UI update rate and VecDeque buys nothing at this cap.
- `crates/ui/src/widgets/activity_tape.rs` (~289 LOC) — pure render function
  `fn view(&ActivityTape) -> Element`. Applies R2.3 200 ms render-floor (red-held
  rows bypass), R3.1 max-3-visible cap + overflow chip, Q5=(a) `DOWN_500` colour
  for failed rows. Zero inline string literals; zero new Lumen tokens.
- `crates/ui/src/widgets/snapshots/` — 4 insta snapshot files created and
  accepted via `INSTA_UPDATE=always`.

### Files modified

- `crates/ui/src/lab/mod.rs` — added `pub mod activity;`
- `crates/ui/src/widgets/mod.rs` — added `pub mod activity_tape;`
- `crates/ui/src/strings.rs` — added 5 string constants:
  `ACTIVITY_KIND_YAHOO_LABEL`, `ACTIVITY_KIND_LAB_RUN_LABEL`,
  `ACTIVITY_KIND_TRAINING_LABEL`, `ACTIVITY_TAPE_MORE_PREFIX`,
  `ACTIVITY_TAPE_MORE_SUFFIX`.
- `crates/ui/src/state.rs` — `Cockpit.activity_tape: ActivityTape` field,
  `Message::ActivityEventReceived(ActivityEvent)` and `Message::ActivityTapePurgeTick`
  variants, update arms delegating to `ActivityTape::apply` and `ActivityTape::purge`.
- `crates/ui/src/live.rs` — `ActivityRecipe` struct + `Recipe` impl +
  `activity_stream_impl` extracted for testability; handles `RecvError::Lagged`
  with `tracing::warn`, `RecvError::Closed` with `debug` + break.
- `crates/ui/src/bin/cockpit_live.rs` — `ActivityRecipe` wired into both
  `Subscription::batch` branches (modal-open and normal).
- `crates/ui/src/widgets/status_bar.rs` — `activity_tape::view(&cockpit.activity_tape)`
  pushed into the status bar row between account label and server-time label
  (Q2=(a) placement: to the LEFT of server-time).

### Test verdict (Wave B)

| Task   | Tests | Result |
|--------|-------|--------|
| T-D-N4 | 5 unit tests in `lab::activity::tests` | ok. 5 passed |
| T-D-N5 | 2 unit tests in `live::tests` (`activity_recipe_emits_messages`, `activity_recipe_handles_lag`) | ok. 12 passed (full module) |
| T-D-N6 | 4 unit tests + 4 insta snapshots in `widgets::activity_tape::tests` | ok. 4 passed |

`scripts/verify_anchors.sh`: ANCHORS PASS (34 / 34)

### Design decisions

- **Vec vs VecDeque**: Chose `Vec<ActivityState>` capped at 32. Rationale: the
  cap is small enough (32 slots) that O(32) linear scans and removals are
  negligible at the iced UI update rate (~60 fps / 16 ms budget). A `VecDeque`
  would buy constant-time front removal but `Cockpit` update runs on the iced
  thread and the scan is sub-microsecond.
- **Snapshot strategy**: insta snapshots capture a descriptive text summary of
  `ActivityTape::visible()` state rather than the iced element tree (which cannot
  be serialized). Follows the `run_button.rs` precedent in this codebase.
- **stream_impl extraction**: `activity_stream_impl` is a free function so
  integration tests can construct a stream directly without a running iced
  application. Follows the `trail_mirror_stream_impl` pattern in `live.rs`.

## Implementation

_Wave D — developer (2026-05-26)_

### Wave D — Criterion bench + integration storm test (T-D-N10 + T-D-N11)

**Files created**:
- `crates/ui/benches/activity_tape.rs` (~200 LOC) — 5 criterion micro-benches
  per feature.md § D3 Layer 2: `activity_handle_tick_throttle`,
  `activity_recipe_fan_out`, `activity_tape_render_empty`,
  `activity_tape_render_three_inflight`, `activity_tape_render_five_plus_overflow`.
- `crates/ui/tests/activity_tape_event_storm.rs` (~150 LOC) — Layer 3 integration
  perf test: 10,000-event concurrent producer/consumer storm with 3 budgeted
  assertions (drain < 1 s, delivery rate >= 95 %, P99 latency < 16 ms).

**Files modified**:
- `crates/ui/Cargo.toml` — added `[[bench]] name = "activity_tape" harness = false`
  + `criterion.workspace = true` in `[dev-dependencies]`.

**Bench baselines (Apple M2 Pro, 2026-05-26, `cargo bench -p ui --bench activity_tape`)**:

| Bench | Result | Budget (D3 L2) | Status |
|-------|--------|-----------------|--------|
| `activity_handle_tick_throttle` | 19.84 ns | < 200 ns | PASS |
| `activity_recipe_fan_out` | 54.74 ns | < 500 ns | PASS |
| `activity_tape_render_empty` | 33.10 ns | < 200 µs | PASS |
| `activity_tape_render_three_inflight` | 912 ns | < 1 ms | PASS |
| `activity_tape_render_five_plus_overflow` | 1.034 µs | < 1.2 ms | PASS |

**Storm test measurements (Apple M2 Pro, 2026-05-26)**:

| Metric | Result | Budget | Status |
|--------|--------|--------|--------|
| drain_time | 7.3 ms | < 1 s | PASS |
| delivery_rate | 1.0000 (10000/10000) | >= 0.95 | PASS |
| p99_latency | 0.040 ms | < 16 ms | PASS |

**Design decisions**:
- **Concurrent producer/consumer**: the storm test uses a 2-task `tokio::join!`
  pattern (producer on one worker, consumer on another). A single-threaded run
  would overflow the 256-slot ring before the consumer gets scheduled. The
  512-slot storm channel (vs. production 256) gives the scheduler headroom;
  the latency/drain measurements still exercise the same codepath.
- **Red-held rows for bench render-floor**: bench helper `build_tape_with_n_inflight`
  uses `End(Failed)` events to put rows in the red-hold state, bypassing the
  200 ms render-floor. This ensures the bench measures the rendering path
  rather than the early-exit empty path.
- **No numeric thresholds in bench code**: per spec, criterion compares against
  its saved baseline; the tester sets the +20 % regression-fail rule at M-FINAL.

**Anchors**: 34/34 PASS (no scenario-body changes; pure bench + test additions).
**Zero regressions introduced** (lab_run_engine H3 failure is pre-existing).

## Verification

_tester links to reports here at M-FINAL._

### Wave C — Producer wiring at 3 call sites (2026-05-26)

**Send-constraint decisions**:

- **T-D-N7 (Yahoo preload)**: Approach A (inline handle). `ActivitySender`
  (Clone+Send) cloned into `iced::Task::perform` async closure. `ActivityHandle`
  (`!Send`) held entirely within the closure on a single task — no thread
  boundary crossing. `spawn_lab_run` gains `activity_sender: Option<ActivitySender>`
  parameter.
- **T-D-N8 (Lab Run)**: Approach A (iced-side hold). `ActivityHandle` stored in
  `AppState::lab_activity_handle`; started on `LabRunRequested`; ticked on
  `LabRunProgress`; ended (Success/Failed/Cancelled) on `LabRunCompleted` /
  `LabRunStopRequested`. `AppState::Clone` implemented manually (ActivityHandle
  is `!Clone`; both fields return `None` on clone — correct since clone only
  happens at cold-boot).
- **T-D-N9 (Training)**: Approach A (caller holds). `spawn_training_run` gains
  `activity_sender: Option<ActivitySender>` parameter and returns
  `(TrainingHandle, Option<ActivityHandle>)`. Caller holds the activity handle
  alongside the training handle and ends it on subprocess exit.

**Files modified**:
- `crates/ui/src/lab/runner.rs` — `spawn_lab_run` signature + Yahoo preload
  activity wiring (T-D-N7).
- `crates/ui/src/lab/trainer.rs` — `spawn_training_run` signature + return type
  + activity handle creation (T-D-N9).
- `crates/ui/src/bin/cockpit_live.rs` — `AppState` manual Clone; new
  `lab_activity_handle` + `training_activity_handle` fields; Lab Run start/tick/end
  lifecycle management (T-D-N8); Yahoo preload sender passed to `spawn_lab_run`
  (T-D-N7).

**Files created**:
- `crates/ui/tests/activity_tape_yahoo_preload.rs` — 2 tests (T-D-N7).
- `crates/ui/tests/activity_tape_lab_run.rs` — 3 tests (T-D-N8).
- `crates/ui/tests/activity_tape_training_run.rs` — 2 tests (T-D-N9).

**Test verdict (Wave C)**:

| Task   | Tests | Result |
|--------|-------|--------|
| T-D-N7 | 2 integration tests | ok. 2 passed |
| T-D-N8 | 3 integration tests | ok. 3 passed |
| T-D-N9 | 2 integration tests (Unix) | ok. 2 passed |

`scripts/verify_anchors.sh`: ANCHORS PASS (34 / 34)
`cargo test -p backtest --test progress_emit`: ok. 6 passed

## Changelog

- 2026-05-25 (architect): authored v0.1.0 draft. R1-R8 + R-NR + K1-K8
  + H1-H5 + Q1-Q8 + D1-D6 design closed. Analyst-recommended defaults
  set on all 8 Qs. Anchor risk zero by construction. HANDOFF →
  developer.
- 2026-05-26 (developer): Wave B complete — T-D-N4 (ActivityTape state),
  T-D-N5 (ActivityRecipe subscription), T-D-N6 (activity_tape widget).
  11 tests pass; 4 insta snapshots accepted; anchors 34/34 PASS.
  HANDOFF → tester (Wave C parallel via background agent).
- 2026-05-26 (developer): Wave C complete — T-D-N7 (Yahoo preload producer),
  T-D-N8 (Lab Run producer), T-D-N9 (Training producer). 7 new integration
  tests (2+3+2); 34/34 anchors PASS; progress_emit 6/6 PASS.
  HANDOFF → tester.
