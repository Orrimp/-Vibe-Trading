---
adr: 0042
title: Cockpit activity broadcast — 10th `EventBus` channel + RAII `ActivityHandle` for in-flight-work tape
status: accepted
date: 2026-05-26
supersedes: none
superseded-by: none
extends: 0012
---

# ADR-0042: Cockpit activity broadcast — 10th `EventBus` channel + RAII `ActivityHandle` for in-flight-work tape

## Context

Operator request 2026-05-25 (verbatim):

> "Status bar should show all the current steps the cockpit is doing —
> downloading data, backtesting, everything else which could be helpful
> for the UI user to understand what's going on in background."

Three "is it stuck?" moments in the two weeks before the request all
traced to the same gap: a background activity (Yahoo cold-cache fetch
30-60 s; Lab Run on a slow universe; training subprocess) had no
operator-facing surface OUTSIDE the screen that triggered it. The bottom
status-bar (`crates/ui/src/widgets/status_bar.rs`, 24 px fixed-height
row) is the project's accepted global UI surface but was previously
silent on every kind of in-flight work. The Lab Run progress bar (shipped
via `lab-end-to-end-v2 v0.1.0` Wave D-4) and the Train sub-panel status
strip (`cockpit-training-control v0.2.0` R3.5) are both screen-local —
an operator on the Live screen got zero feedback that a Run was underway.

The feature brief at
[`spec/cockpit-activity-status-bar/feature.md`](../../v1/cockpit-activity-status-bar/feature.md)
landed v0.1.0 in three Waves (A — agent bus extension; B — UI tape state
+ recipe + widget; C — producer wiring at Yahoo preload / Lab Run /
Training subprocess; D — perf gates). Shipped 2026-05-26. The
implementation locked one cross-cutting decision that this ADR records:
**where do in-flight-work events live, and what shape do producers see?**

The decision space at architect M0:

- **(a) Broadcast bus extension.** Add a 10th channel
  `activity_tx: broadcast::Sender<ActivityEvent>` on the existing
  `agent::bus::EventBus` (sibling of `fills` / `positions` / `bars` /
  `ticks` / `pnl` / `mode` / `strategy_loaded` / `strategy_swapped` /
  `strategy_error` / `funding_obs` / `market_health` / `risk_telemetry`).
  Producers obtain an RAII handle via `bus.activity().start(kind, label)`;
  the status-bar widget subscribes via `bus.activity().subscribe()`. The
  same backpressure contract every other channel uses (bounded ring;
  `RecvError::Lagged` skips events; never blocks producer).
- **(b) `tracing_subscriber::Layer`.** Producers emit `tracing::info_span!`
  spans annotated `activity = true`; a custom `Layer` impl filters and
  routes them to the UI. No new bus channel.
- **(c) Per-source polling.** Each subsystem (Yahoo preload, Lab Run,
  Training) exposes a `Vec<InFlight>` accessor; UI polls each at 1 Hz
  and aggregates.

This ADR exists because **CLAUDE.md non-negotiable** — every new
broadcast channel on `EventBus` requires an ADR-level decision per the
ADR-0012 precedent ("v0.5 strategy broadcast types in trading_core").
ADR-0012 set the pattern for "add a new channel on the bus when the
event space is operator-facing, multi-publisher, single-shape"; this
ADR is the second invocation of that pattern (one channel added per
ADR; ten total at v0.1.0 ship).

## Decision

### D1 — Q1 = (a): broadcast bus extension wins

The activity tape uses a 10th `EventBus` channel — `activity_tx:
broadcast::Sender<ActivityEvent>` at capacity 256. The pattern matches
the existing nine channels verbatim; no new mechanism is introduced.

This locks five sub-decisions that all collapse to the broadcast-bus
choice:

#### D1.1 — Event shape

```rust
// crates/agent/src/activity.rs
pub struct ActivityEvent {
    pub id: ActivityId,       // u64 monotonic per-process counter, NOT a UUID
    pub kind: ActivityKind,   // YahooPreload | LabRun | Training | LlmCall | AuditLedgerWrite
    pub label: String,        // operator-facing copy, ≤ 64 chars recommended
    pub phase: ActivityPhase, // Start { total_units } | Tick { current, elapsed_ms } | End(Outcome)
    pub ts_ms: i64,           // wall-clock ms since Unix epoch (UTC)
}

pub enum ActivityOutcome { Success, Failed(String), Cancelled }
```

`ActivityId` is `u64` (monotonic per-process counter, not UUID — the
activity tape is in-memory only per R-NR.4; IDs do not need to survive
restarts). All fields are `Clone` so the broadcast channel fans out to
multiple subscribers without per-receiver allocation. The two
forward-listed `ActivityKind` variants (`LlmCall`, `AuditLedgerWrite`)
exist in the enum at v0.1.0 but are unwired — they sit out for
v0.1.1+ (R5).

#### D1.2 — Capacity 256 bounded ring-buffer (lossy under producer storm)

`broadcast::channel::<ActivityEvent>(256)` constructed in `EventBus::new`.
Producers never block (`broadcast::Sender::send` returns
`SendError::Closed` only if there are zero receivers — silently
ignored, matching `EventBus::publish_fill` and every other channel).
Slow consumers get `RecvError::Lagged(n)` and skip — same backpressure
contract as every other channel. Verified by the
`activity_channel_lag_drops_oldest` test at
[`crates/agent/src/bus.rs:366`](../../../crates/agent/src/bus.rs)
which fills the 256-slot ring with 257 events and asserts the receiver
gets `TryRecvError::Lagged`.

**Lossiness is OK** because the activity tape is operator-eyeball UX,
not an audit artifact. The audit ledger remains the source of truth
for compliance (per R-NR.4 — in-memory only, no persistence). If an
event drops, the operator sees stale tape until the next event lands;
the worst case is a 25 s gap (256 slots / 10 events/sec steady-state).

#### D1.3 — RAII `ActivityHandle` with `Drop` emits End (R1.3)

Producers do not call `send` directly. They obtain a handle:

```rust
let handle = bus.activity().start(ActivityKind::YahooPreload, "Yahoo BTC-USD 2y · downloading");
// ... do work ...
handle.tick(bars_done);
// handle drops here → emits End { Success } automatically
```

The `Drop` impl emits exactly one `End` event with the recorded outcome:

- Default → `End { Success }`.
- After `handle.fail(reason)` → `End { Failed(reason) }`.
- After `handle.cancel()` → `End { Cancelled }`.
- If `std::thread::panicking()` is true at drop time →
  `End { Failed("dropped during panic") }`.

The last branch is the **bug-hunting affordance** — a `Failed("dropped
during panic")` event in the operator's view is a flag that some
producer panicked mid-task and unwound past the handle. The 3-second
red-hold rendering in the status bar gives the operator time to notice.

Reference impl:
[`crates/agent/src/activity.rs:232`](../../../crates/agent/src/activity.rs)
(Drop), L220 (`fail`), L227 (`cancel`).

#### D1.4 — 100 ms producer-side throttle (R1.4)

`ActivityHandle::tick(current)` is rate-limited to ≤ 10 events/sec per
handle:

```rust
const TICK_THROTTLE: Duration = Duration::from_millis(100);

pub fn tick(&self, current: u64) {
    let now = Instant::now();
    if now.duration_since(self.last_tick.get()) < TICK_THROTTLE {
        return; // throttled — silent no-op
    }
    self.last_tick.set(now);
    // ... send Tick event ...
}
```

The throttle is a single `Instant::now()` + `Cell::get/set` — no
allocation, no syscall on the fast path. The criterion bench
`activity_handle_tick_throttle` measured **19.84 ns / call** P99 at
Wave D (budget < 200 ns; far inside K5 hot-path mitigation).

Throttle SHAPE is per-handle (single `Instant` field on the handle, not
a global rate-limiter). With 3 handles in v0.1.0 the steady-state cap
is 30 events/sec, well below the 256-slot ring's drain capacity
(measured at 10k events drained in 7.3 ms in the storm integration test).

#### D1.5 — In-memory only (R-NR.4)

The `EventBus` is constructed once per agent-process startup; the
`activity_tx` field is created internally with no persistence. No audit
migration. No subprocess. No IPC. The status-bar tape (`Cockpit::activity_tape`
at `crates/ui/src/state.rs`) lives in iced process memory; on cockpit
restart the tape is empty by construction.

Compliance / audit / forensics use the existing audit ledger
(`crates/audit`) — the activity tape is purely operator-eyeball UX.
This is the load-bearing distinction that makes the lossiness in D1.2
acceptable.

## Consequences

**Enforced by:**

- `cargo test -p agent --lib activity_types` — 6 type-level tests
  (`activity_event_clone_round_trips`, `activity_kind_hash_round_trips`,
  `activity_id_atomic_monotonic`, `activity_handle_drop_emits_end`,
  `activity_handle_throttle_caps_at_10_hz`,
  `activity_handle_drop_during_panic_emits_failed`) PASS.
- `cargo test -p agent --lib bus::tests::activity_channel_lag_drops_oldest`
  — the 256-slot ring-buffer Lagged-on-overflow contract.
- `cargo test -p ui --test activity_tape_event_storm` — Layer 3
  integration perf test: 10k-event burst drains in < 1 s, delivery
  rate ≥ 95 %, P99 latency < 16 ms.
- `cargo bench -p ui --bench activity_tape` — 5 criterion benches PASS
  absolute budget (tick-throttle 20 ns / fan-out 55 ns / render-empty
  33 ns / render-3 912 ns / render-overflow 1.03 µs).
- `bash scripts/verify_anchors.sh` — `ANCHORS PASS (34 / 34)` at
  M-FINAL (R-NR.1 contract: zero scenario-body / exec / strategy /
  report-crate changes).
- `crates/agent/tests/no_new_bus_channel.rs` — snapshot test updated
  to include `activity_tx` in the v1+ field list (intentional
  architect-approved addition; future channel adds must update this
  snapshot in a separate ADR).

**What this enables:**

- **Cheap producer wiring.** Each new producer is a 1-line `let handle
  = bus.activity().start(kind, label);` plus optional `handle.tick(n)`
  calls. The Wave C wiring landed at 3 sites with ~ 5-10 LOC of net
  addition per site (see `crates/ui/src/lab/runner.rs:600-637`,
  `crates/ui/src/bin/cockpit_live.rs:1313-1326`, and
  `crates/ui/src/lab/trainer.rs:173-191`).
- **Subscriber-side reuses `Recipe`.** The new `ActivityRecipe` in
  [`crates/ui/src/live.rs`](../../../crates/ui/src/live.rs) at L691 is a
  10-line sibling of the existing `BusRecipe` / `ServerTimeRecipe`. No
  novel subscription mechanism.
- **Lossiness is operator-acceptable.** Activity tape is UI eye candy,
  not compliance. The audit ledger is the source of truth. Operator
  loses 25 s of tape state in the absolute worst case (UI-thread
  stall); the cockpit is itself frozen at that point so the staleness
  is moot.
- **Extension surface for v0.1.1+ is structural.** Both `LlmCall` and
  `AuditLedgerWrite` already exist as `ActivityKind` variants; future
  producers wire by adding `bus.activity().start(...)` calls — no
  channel schema migration.
- **Forward-listed `v3-llm-forecaster` integration.** Once that feature
  reaches M-T1, the M-T1 architect adds `bus.activity().start(
  ActivityKind::LlmCall, label)` around each provider call. K4 risk
  (label-redaction) is deferred to that brief — this ADR does not
  enforce a redaction policy at v0.1.0 because no LLM producer is wired.

**What costs this incurs:**

- **`ActivityHandle` is `!Send` by design.** The handle uses `Cell<_>`
  for the throttle state (single-thread tick contract). Producers
  must hold the handle on the same task that started it OR:
  - Approach A (inline, used at all three Wave C sites): hold the
    handle entirely within one async closure or one `iced::Task`
    callback. Cheapest; preferred default.
  - Approach B (Mutex): wrap in `Arc<Mutex<ActivityHandle>>` if the
    handle MUST cross `await` boundaries to a different task.
  - Approach C (sender-side signal): the producer keeps an
    `ActivitySender` (which IS `Send + Clone`), spawns the work
    elsewhere, and signals completion via an mpsc / oneshot back to
    the holder.
  The Wave C Bug #65 fix in flight (vol-killswitch overlay no-op
  recovery — see `spec/vol-killswitch-overlay-noop-fix/`) hits this
  constraint at a producer site; the mitigation is documented in the
  recovery brief.
- **Future LLM-call producer (v0.1.1) needs a PII redaction policy.**
  A label like `"LLM claude-3-5-sonnet-20241022 · forecast"` exposes
  vendor internals; `"LLM · forecast (reflection: ETH 2023-03-14
  trim drawdown)"` could expose user-positioned strategy intent. K4
  mitigation: when v3-llm-forecaster wires this producer at v0.1.1,
  the brief MUST codify a label-redaction rule (e.g. "no model
  versions; no reflection lesson bodies; activity-kind + symbol +
  state only").
- **`AuditLedgerWrite` producer cannot be wired naively.** A fast
  backtest emits thousands of audit writes per second; per-event
  100 ms throttle (D1.4) is the wrong place to enforce aggregation.
  R5.2 explicitly defers this to v0.1.1 plus a new "audit writer:
  aggregate to one 'flushing' activity per second" design. The
  variant exists in the enum but no producer wires it at v0.1.0.

**Anchor-additive contract:**

- ADR-0038 § D6 anchor-additive contract applies trivially. The
  activity-tape feature touches zero anchored report files; the 34
  body-SHA-256 anchors stay byte-identical (R-NR.1 contract;
  `scripts/verify_anchors.sh → ANCHORS PASS (34 / 34)` at M-FINAL
  commit `0ff402f`).

## Alternatives rejected

### Alt-1: Q1 = (b) tracing-subscriber `Layer`

Producers emit `tracing::info_span!(name = "activity", kind = ..., label = ...)`
spans; a custom `tracing_subscriber::Layer` impl filters spans with the
`activity = true` attribute and routes them to the UI.

**Rejected.** Three reasons:

1. **Fragile.** `tracing::Layer` event-shape is constrained by
   `tracing::field::Visit`; a typed enum like `ActivityKind` becomes
   a string round-trip, defeating the cargo-cult goal of
   compile-checked producers. Wave C's three producer sites all use
   `match handle.kind { … }` patterns the type-checker enforces — a
   string-typed channel would have shipped a flicker bug into v0.1.0.
2. **Slow.** `fmt::Layer` (the typical infrastructure) formats every
   span; subscribers inherit the allocation cost even for the activity
   tape's 30 events/sec steady state. The criterion bench surfaced
   that the broadcast-channel hot path is 19 ns / call; a tracing-
   layer round-trip would have measured in the µs range, eating the
   K5 sub-frame budget.
3. **Couples UI to log filtering.** The cockpit would need to install
   a `Layer` distinct from the existing `tracing-subscriber` config;
   future tracing reconfiguration (verbosity, filter EnvFilter, sink
   rotation) would risk silently breaking the activity tape.

### Alt-2: Q1 = (c) per-source polling

Each subsystem exposes a `Vec<InFlight>` accessor; the UI polls each
at 1 Hz and aggregates client-side.

**Rejected.** Three reasons:

1. **Coupling-heavy.** Each new subsystem must implement an
   `InFlight` snapshot accessor; the UI must update its polling
   manifest. The activity-tape goal is "many publishers, the UI
   subscribes" — `EventBus` already solved this for fills /
   positions / bars / ticks. Inventing a parallel polling DAG is
   redundant.
2. **Doesn't extend.** The forward-listed `LlmCall` (v0.1.1) and
   `AuditLedgerWrite` (v0.1.1) producers are by nature event-shaped
   (a single LLM call fires, completes, and is done; no snapshot
   shape). Polling forces the producers to maintain an in-flight set
   that the broadcast model gets for free.
3. **1 Hz lag is operator-visible.** Yahoo preload, Lab Run, and
   Training all complete in seconds-to-minutes; an event-driven tape
   updates immediately, a 1-Hz poll lags by up to a frame second.
   On a fast Lab Run (synthetic GBM, < 5 s) the operator might never
   see the activity at all.

### Alt-3: New `crates/activity` crate

A separate crate for the activity types + RAII handle, dep'd by
`crates/agent` and `crates/ui`.

**Rejected.** Considered briefly; chose the inline-in-`crates/agent`
approach because:

1. The types are cohesive with the existing `EventBus` channel set.
   The activity broadcast IS a 10th channel on the same bus, not a
   parallel mechanism.
2. A separate crate would force `crates/agent` to dep on `crates/activity`,
   adding a workspace edge for zero functional isolation. The
   `crates/agent/src/activity.rs` sibling module (~280 LOC) is the
   cheapest placement.
3. ADR-0012 set the precedent for "broadcast types live in the
   crate that owns the bus" — this ADR is the second invocation,
   not a divergence.

## Cross-references

- **ADR-0012** ([0012-v05-broadcast-bus-extensions.md](0012-v05-broadcast-bus-extensions.md))
  — broadcast-types-in-trading_core pattern; established the
  "add a channel per ADR" cadence this ADR extends.
- **Feature brief**: [`spec/cockpit-activity-status-bar/feature.md`](../../v1/cockpit-activity-status-bar/feature.md)
  — R1 (event source) / R2 (status-bar widget) / R3 (UI tape state) /
  R4 (producer wiring) / R-NR (non-regression contract) / Q1-Q8 / D1-D6.
- **Tasks**: [`spec/cockpit-activity-status-bar/tasks.md`](../../v1/cockpit-activity-status-bar/tasks.md)
  — Wave A (T-D-N1..N3) + Wave B (T-D-N4..N6) + Wave C (T-D-N7..N9) +
  Wave D (T-D-N10..N11) execution log.
- **Trace row**: `REQ-COCKPIT-ACTIVITY-001` in
  [`spec/trace.toml`](../../trace.toml) at L1273 (state = `passed`,
  anchors = `34/34 PASS`, M-FINAL commit `0ff402f`).
- **Shipping commits**:
  - `4248c00` — Wave A: agent bus extension (`crates/agent/src/activity.rs`
    + `crates/agent/src/bus.rs` `activity_tx` field + `activity()` accessor).
  - `ea52057` — Wave B: UI tape state + recipe + render widget.
  - `49bf342` — Wave C: producer wiring at Yahoo / Lab Run / Training sites.
  - `ef6f018` — Wave D: criterion bench + 10k-event storm integration test.
  - `0ff402f` — M-FINAL blocker resolution; tester verdict PASS.
  - `f728334` — v0.1.0 shipped (operator approved 2026-05-26).
- **Reference implementation**:
  - [`crates/agent/src/activity.rs`](../../../crates/agent/src/activity.rs)
    — type module (`ActivityEvent`, `ActivityKind`, `ActivityPhase`,
    `ActivityOutcome`, `ActivityId`, `ActivitySender`, `ActivityHandle`).
  - [`crates/agent/src/bus.rs:98`](../../../crates/agent/src/bus.rs)
    — `activity_tx: broadcast::Sender<ActivityEvent>` field declaration.
  - [`crates/agent/src/bus.rs:301`](../../../crates/agent/src/bus.rs)
    — `pub fn activity(&self) -> ActivitySender` accessor.
- **Cross-feature precedents**:
  - [`spec/cockpit-training-control/feature.md`](../../v1/cockpit-training-control/feature.md)
    R3.5 — status-strip + training_events poll (R4.3 producer-wiring site).
  - [`spec/lab-end-to-end-v2/feature.md`](../../v1/lab-end-to-end-v2/feature.md)
    Wave D-4 — Progress channel (R4.2 producer-wiring site).

## Changelog

- 2026-05-26 (architect, post-ship): initial accept. Codifies the design
  shipped at `cockpit-activity-status-bar v0.1.0` (commits `4248c00` +
  `ea52057` + `49bf342` + `ef6f018` + `0ff402f` + `f728334`). Locks D1
  Q1=(a) broadcast-bus-extension over (b) tracing-layer / (c) per-source
  polling; D1.1 ActivityEvent shape (id u64 monotonic / kind / label /
  phase / ts_ms); D1.2 capacity-256 bounded ring (lossy under producer
  storm; activity tape is operator-eyeball UX not audit); D1.3 RAII
  ActivityHandle Drop semantics (Default Success / `fail` / `cancel` /
  panic-detection emits `Failed("dropped during panic")`); D1.4 100 ms
  producer-side throttle (per-handle, `Cell<Instant>` sync wall-clock
  check; 19.84 ns/call P99 measured); D1.5 in-memory only (audit ledger
  is source of truth). Deferred to v0.1.1: PII redaction policy for
  `LlmCall` producer; aggregator design for `AuditLedgerWrite` producer.
  34/34 anchors stay byte-identical (zero scenario-body change).
  Closes T-AR-4 of `spec/cockpit-activity-status-bar/tasks.md`.
