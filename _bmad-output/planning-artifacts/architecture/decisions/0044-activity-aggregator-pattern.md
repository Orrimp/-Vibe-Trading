---
adr: 0044
title: Activity-aggregator producer pattern — broadcast-receiver + AtomicU32 + tokio interval + long-lived ActivityHandle with idle-end semantics, for high-frequency event sources on the cockpit activity tape
status: accepted
date: 2026-05-26
deciders: analyst (M0 2026-05-26) → architect (M-T1 2026-05-26)
supersedes: none
superseded-by: none
extends: 0042
related: ["ADR-0042 cockpit-activity-broadcast", "ADR-0031 audit-tick-consumer-envelope"]
---

# ADR-0044 — Activity-aggregator producer pattern (audit-ledger-writes producer with 100 ms aggregation envelope)

## Context

Parent ADR-0042 (cockpit-activity-broadcast) shipped the 10th
`EventBus` channel + RAII `ActivityHandle` for in-flight-work tape at
v0.1.0 (2026-05-26). The activity tape's producer contract is:

1. Producer obtains a handle via `bus.activity().start(kind, label)`.
2. Producer ticks the handle ≤ once per 100 ms via the built-in
   throttle (ADR-0042 § D1.4).
3. Producer drops the handle when work is done; `Drop` emits `End`.

That contract works perfectly for the v0.1.0 producers (Yahoo preload,
Lab Run, Training subprocess) which are **single long-running units of
work** that the operator wants to track. It breaks for **high-frequency
event sources** — specifically the audit-ledger-writes producer
explicitly deferred at parent ADR-0042 § "What costs this incurs":

> `AuditLedgerWrite` producer cannot be wired naively. A fast backtest
> emits thousands of audit writes per second; per-event 100 ms throttle
> (D1.4) is the wrong place to enforce aggregation. R5.2 explicitly
> defers this to v0.1.1 plus a new "audit writer: aggregate to one
> 'flushing' activity per second" design.

The naive wiring — one `bus.activity().start(AuditLedgerWrite, "Audit
Fill")` call per `tick::emit` site — would generate 7-8 events per
fill/signal pair (one per journal-writer in the dual-write chain) and
**overwhelm the 256-slot `activity_tx` ring buffer within milliseconds**
of a moderately-active backtest. The per-handle throttle caps a single
handle to 10 events/sec, but does NOT cap **handle creation**, and the
audit writers are short-lived (`start → commit → drop`), each spawning
a fresh handle.

The feature brief at
[`docs/archive/pre-bmad-spec/v1/cockpit-activity-audit-ledger-producer/feature.md`](../../../../docs/archive/pre-bmad-spec/v1/cockpit-activity-audit-ledger-producer/feature.md)
introduces a new producer pattern — an **aggregator** between the
existing `crates/audit::Ledger::tick_bus` broadcast (ADR-0031) and the
`EventBus::activity()` channel (ADR-0042). The aggregator absorbs
arbitrary-rate `AuditTick<AuditEvent>` events and emits at most
10 events/sec onto the activity channel, preserving the parent
ADR-0042 § D1.2 lossiness contract.

This ADR codifies the aggregator pattern so it can be reused for
future high-frequency event sources (forecast cache-hit storms,
multi-venue order-book updates if they ever surface on the activity
tape, etc.) without re-deriving the design.

## Decision

### D1 — Aggregator producer placement: `crates/agent`, sibling of `activity.rs`

The aggregator lives at NEW
[`crates/agent/src/activity_audit_aggregator.rs`](../../../../crates/agent/src/activity_audit_aggregator.rs)
— a sibling of the existing
[`crates/agent/src/activity.rs`](../../../../crates/agent/src/activity.rs)
that owns the `EventBus::activity()` channel + `ActivityHandle`. The
aggregator IS a producer on that channel (it calls `bus.activity()
.start(AuditLedgerWrite, ...).tick(N)` on every non-empty 100 ms
window); cohesion is with the producer-side types, not with the UI
subscriber.

**Rejected**: `crates/ui/src/audit_activity_bridge.rs`. The UI is a
subscriber of the activity channel via `ActivityRecipe` (ADR-0042
§ "Subscriber-side reuses Recipe"); a UI-side bridge would invert the
dataflow direction and put producer code in a crate that has no other
producer responsibility.

**Rejected**: `crates/audit/src/activity_aggregator.rs`. Zero changes
to `crates/audit/` is a load-bearing R-NR.1 contract on the feature
brief — anchor-additive by construction. The audit crate must stay
unaware of the activity tape so the audit ledger remains the source
of truth for compliance / forensics (ADR-0042 § D1.5).

### D2 — Aggregator internal shape

```rust
// crates/agent/src/activity_audit_aggregator.rs
pub struct Aggregator {
    rx: broadcast::Receiver<AuditTick<AuditEvent>>,  // ledger.tick_bus.subscribe()
    bus: ActivitySender,                              // bus.activity()
    counter: AtomicU32,                               // fetch_add(1, Relaxed) per recv
    handle: Option<ActivityHandle>,                   // long-lived; idle-end on empty window
    interval: tokio::time::Interval,                  // 100 ms cadence
}

pub fn spawn_aggregator(
    ledger: &Arc<Ledger>,
    bus: &EventBus,
) -> tokio::task::JoinHandle<()> {
    let agg = Aggregator::new(ledger.tick_bus.subscribe(), bus.activity());
    tokio::spawn(async move { agg.run().await })
}
```

Hot-path inside `run()`:

```rust
loop {
    tokio::select! {
        recv = self.rx.recv() => match recv {
            Ok(_tick) => { self.counter.fetch_add(1, Ordering::Relaxed); }
            Err(broadcast::error::RecvError::Lagged(n)) => {
                tracing::warn!(consumer = "activity_audit_aggregator", lagged = n, "audit tick stream lagged");
                self.counter.fetch_add(n as u32, Ordering::Relaxed);
            }
            Err(broadcast::error::RecvError::Closed) => break,
        },
        _ = self.interval.tick() => {
            let n = self.counter.swap(0, Ordering::Relaxed);
            match (n, self.handle.as_ref()) {
                (0, Some(_)) => { self.handle = None; }            // idle-end → Drop emits End{Success}
                (0, None)    => {}                                 // still idle, no-op
                (n, None)    => {                                  // first non-empty window
                    let label = format_label(n);
                    let h = self.bus.start(ActivityKind::AuditLedgerWrite, label);
                    h.tick(n as u64);
                    self.handle = Some(h);
                }
                (n, Some(h)) => { h.tick(n as u64); }               // continuing burst
            }
        }
    }
}
```

The hot path on each tick is **one `AtomicU32::fetch_add(1, Relaxed)`**
— sub-50 ns on Apple Silicon (criterion-measured at Wave C T-D-N6).
The broadcast::send is already on the audit writer's hot path today
(cost amortised in v0.1.0 of `audit-tick-consumer-envelope`). No new
allocation, no new syscall.

### D3 — Aggregation window: 100 ms aligned with `ActivityHandle::tick` throttle

The aggregator's `tokio::time::interval` is pinned to 100 ms —
**verbatim the same cadence** as the parent `ActivityHandle::tick`
throttle (ADR-0042 § D1.4, measured at 19.84 ns/call P99). The
alignment is intentional:

1. **Status-bar render budget unchanged.** Every other producer on
   the bus (Yahoo preload, Lab Run, Training) emits at most
   10 events/sec; the aggregator does the same. The 256-slot ring
   buffer's 25 s lag-tolerance (ADR-0042 § D1.2) applies uniformly.
2. **Operator UX consistency.** The activity tape updates at a
   uniform ~10 Hz cadence regardless of the underlying producer's
   event rate. No producer "feels different" in render latency.
3. **No new tunable.** Reusing 100 ms means there is no separate knob
   that drifts over time relative to the parent throttle. If a future
   ADR ever changes ADR-0042 § D1.4, this ADR's aggregator inherits
   the change.

**Rejected**: per-batch aggregation (1 event per `journal_transactions`
write). This generates ~10-50 ev/sec on a fast backtest — still too
chatty for the render budget and structurally divergent from the
100 ms cadence the rest of the tape uses.

**Rejected**: per-entity aggregation (one handle per `AuditEvent`
variant). With 9 variants today (`#[non_exhaustive]` per ADR-0031),
the status bar's max-3-visible-rows budget (parent R2.2) cannot
render more than 3 simultaneously without the overflow chip. Defer
to v0.2.0 if operator surfaces a use case.

### D4 — Failure semantics: separate-handle Failed emission (not main-handle `fail()`)

The existing `crates/audit::tick::emit` only fires on
`db_txn.commit().await.is_ok()` (see `crates/audit/src/tick.rs:138`)
— the aggregator sees **only successful commits**. On any post-commit
SQL error path (which today emits nothing on the tick bus), the
aggregator's main long-lived handle stays in `Success` state — it
faithfully reflects the writes that DID land.

For future error-tap producers (v0.1.1+ partial-commit reporter, etc.):

- **Pattern**: emit a separate, transient `ActivityHandle` at the
  failure-observer call site:
  ```rust
  let handle = bus.activity().start(ActivityKind::AuditLedgerWrite,
                                     "Audit: write failed");
  handle.fail(format!("ledger error: {}", err));
  // drop emits End{Failed(reason)} → 3 s red hold per parent R2.5
  ```
- **Why separate-handle**: tainting the aggregator's main handle red
  on a single failure is misleading — the other N writes in the same
  100 ms window succeeded. The two-handle pattern preserves the
  truthfulness of the success-count display while still surfacing
  the error to the operator.

**Rejected**: flip aggregated handle to `Failed` on any inner write
error. Misleading semantics per the above.

### D5 — Idle-end semantics: long-lived handle, end on first empty window

The aggregator holds a single `ActivityHandle` as long as audit ticks
are arriving. On the first 100 ms window that observes **zero** ticks,
the handle is dropped (which emits `End { Success }` via `Drop`). The
next non-empty window starts a fresh handle (new `ActivityId`).

**Why**:

- Gives the operator the "audit is currently active" / "audit is quiet"
  boolean for free, no extra state.
- Respects the parent ADR-0042 § "200 ms render-floor" — single-tick
  bursts get rendered because the handle stays alive ≥ 200 ms whenever
  there's a follow-up tick within the next window.
- Cross-batch correlation (K4 in feature.md) is out-of-scope at
  v0.1.0; the aggregator does NOT thread `bar_id` / `run_id` through.
  Forward-listed to v0.2.0 if operator complains.

**Rejected**: short-lived per-batch handle (start + end per 100 ms
window). 3× chattier than necessary (3 events/sec/window vs. 1
event/sec/window in steady-state).

**Rejected**: strategy-boundary-tied handle (one per `Bar` cycle).
Requires cross-crate coupling between `crates/audit` and
`crates/backtest` that R-NR.1 explicitly forbids.

## Alternatives considered

- **Alt-1: Synchronous flush at the `tick::emit` call site.** Reject —
  injects unbounded latency into the audit hot path (which is
  itself in the strategy commit critical path); violates parent
  K3 < 1 % overhead budget; couples `crates/audit` to `crates/agent`
  in violation of R-NR.1.
- **Alt-2: Sized batch (N writes → 1 event).** Reject — batches of
  varying SQL-write rates produce non-uniform UI update cadence;
  operator perceives the tape as "jumping" rather than smooth.
- **Alt-3: Per-variant aggregator (9 parallel handles).** Reject —
  exceeds the status-bar max-3-rows budget; semantic information
  the operator gets (which subsystem is hot) is better surfaced via
  the existing `audit_tick_emitted_total{variant=...}` metric.
- **Alt-4: Reuse the `ActivityHandle` directly from the
  `tick::emit` call site (no aggregator).** Reject — naive wiring;
  the exact failure mode this ADR was authored to prevent.

## Consequences

**Enforced by:**

- `cargo test -p agent --test activity_audit_aggregator` — 4 unit
  tests (`aggregator_emits_one_tick_per_window`,
  `aggregator_idle_drops_handle`,
  `aggregator_handle_resumes_after_idle`, `aggregator_panic_isolated`).
- `cargo test -p ui --test activity_tape_audit_ledger_event_storm` —
  10k-event burst → counter completeness + rate-cap + zero-Failed
  + K2 truncation assertions.
- `cargo test -p agent --test activity_audit_no_failed_events` —
  D4 invariant gate.
- `cargo bench -p agent --bench activity_audit` — 3 criterion
  micro-benches PASS budget (< 100 ns/tick counter increment,
  < 1 µs interval-tick fan-out, < 100 µs idle-end transition).
- `cargo bench -p agent --bench activity_audit -- aggregator_anchor_replay_parity`
  — R5.2 K3-discharge gate; wall-clock divergence < 1 % at p99 on
  the `top10-2024-fy-momentum-bs1` anchor with vs without aggregator.
  **This is the load-bearing gate** — failure halts ship.
- `bash scripts/verify_anchors.sh` — `ANCHORS PASS (34 / 34)` at
  M-FINAL (R-NR.1 contract: zero scenario-body / strategy / audit /
  report-crate changes).

**What this enables:**

- **Reusable pattern for high-frequency producers.** The aggregator
  shape (broadcast-receiver + `AtomicU32` + `tokio::time::interval` +
  long-lived `ActivityHandle` with idle-end) is the canonical recipe
  any future producer with > 100 ev/sec rate should follow.
  Documented here so v0.2.0+ producers (e.g. forecast cache-hit
  storms; multi-venue order-book updates if they surface on the tape)
  can copy the shape verbatim.
- **Single-line wire-up at cockpit boot.**
  `agent::spawn_aggregator(&ledger, &bus)` in
  `crates/ui/src/bin/cockpit_live.rs` (AFTER the iced::Subscription
  is staged, per K6 ordering mitigation in the feature brief). The
  returned `JoinHandle` is held on `AppState` for graceful abortion.
- **Zero changes to `crates/audit/`.** R-NR.1 contract preserved.
  The audit crate stays unaware of the activity tape; the audit
  ledger remains the source of truth for compliance / forensics
  (ADR-0042 § D1.5).
- **Anchor-additive by construction.** 34/34 anchors stay
  byte-identical at M-FINAL (no scenario body / strategy / audit
  changes; UI + agent additive only).

**What costs this incurs:**

- **One long-lived `tokio::task` per cockpit process.** Memory
  footprint: ~1 KB (task stack frame + Aggregator struct fields).
  CPU footprint at idle: zero (interval.tick().await yields to the
  reactor; broadcast::Receiver::recv yields when the channel is
  empty). CPU footprint under storm: dominated by the `AtomicU32::
  fetch_add` (50 ns/tick measured).
- **K5 panic propagation.** If the aggregator task panics (e.g. a
  hypothetical poison-pill `AuditEvent` variant), the cockpit's
  activity tape silently stops updating from this producer. Mitigation:
  `tokio::spawn` JoinHandle is polled by a future supervisor (out of
  scope at v0.1.0); for now, a `tracing::warn!` on the recv-loop's
  `Err(Lagged)` arm + a `#[ignore]`-marked `aggregator_panic_isolated`
  test document the gap. Forward-listed to a future ADR if operator
  surfaces a real panic-recovery requirement.
- **K6 startup ordering.** The aggregator MUST be spawned **after**
  the iced `Subscription` lifecycle starts — otherwise the first
  burst of audit ticks is fanned out to zero subscribers and silently
  dropped. Mitigation: place `spawn_aggregator` AFTER the iced
  `application(...).subscription(...)` is staged in `cockpit_live.rs`;
  document in tasks.md Wave B T-D-N5 inline.

**Anchor-additive contract:**

- ADR-0038 § D6 anchor-additive contract applies trivially. The
  aggregator feature touches zero anchored report files; the 34
  body-SHA-256 anchors stay byte-identical (R-NR.1 contract;
  `scripts/verify_anchors.sh → ANCHORS PASS (34 / 34)` at M-FINAL).

## Cross-references

- **ADR-0042**
  ([0042-cockpit-activity-broadcast.md](0042-cockpit-activity-broadcast.md))
  — parent feature; 10th `EventBus` channel + RAII
  `ActivityHandle`. This ADR EXTENDS parent ADR-0042 by adding the
  aggregator producer pattern as a reusable shape; does NOT
  supersede.
- **ADR-0031**
  ([0031-audit-tick-consumer-envelope.md](0031-audit-tick-consumer-envelope.md))
  — `AuditTick<Event, Context>` broadcast envelope. The aggregator
  is one new subscriber on the existing `tick_bus` (capacity 1024,
  drop-on-lag). Zero changes to `crates/audit/`.
- **Feature brief**:
  [`docs/archive/pre-bmad-spec/v1/cockpit-activity-audit-ledger-producer/feature.md`](../../../../docs/archive/pre-bmad-spec/v1/cockpit-activity-audit-ledger-producer/feature.md)
  — R1 (aggregation policy) / R2 (label format) / R3 (lifecycle) /
  R4 (failure handling) / R5 (performance budget) / R6 (placement) /
  R-NR (non-regression) / K1-K6 (risks) / H1-H3 (hypotheses) /
  Q1-Q3 (open Qs) / D1-D4 (design sketch).
- **Tasks**:
  [`docs/archive/pre-bmad-spec/v1/cockpit-activity-audit-ledger-producer/tasks.md`](../../../../docs/archive/pre-bmad-spec/v1/cockpit-activity-audit-ledger-producer/tasks.md)
  — M-T1 (this ADR) → M-DEV Waves A-D → M-FINAL (R5.2 K3-discharge
  gate) → M-PRESENTER.
- **Trace row**: `REQ-COCKPIT-ACTIVITY-AUDIT-LEDGER-001` in
  [`_bmad-output/planning-artifacts/trace.toml`](../../../../_bmad-output/planning-artifacts/trace.toml) (state = `proposed` at
  architect M-T1 land; tester flips to `passed` at M-FINAL).
- **Cross-feature precedent**: parent
  `cockpit-activity-status-bar v0.1.0` (shipped 2026-05-26 at commit
  `f728334`).

## Changelog

- 2026-05-26 (architect, M-T1): initial accept. Locks D1 producer
  placement at `crates/agent/src/activity_audit_aggregator.rs`
  (sibling of `activity.rs`) over `crates/ui/` or `crates/audit/`;
  D2 internal shape (broadcast::Receiver + AtomicU32 counter +
  tokio::time::interval + long-lived ActivityHandle); D3 100 ms
  cadence verbatim from parent ADR-0042 § D1.4; D4 separate-handle
  Failed emission (NOT main-handle `fail()`); D5 idle-end semantics
  (long-lived; drop on first empty window). Anchor-additive by
  construction — 34/34 anchors stay byte-identical. Closes T-AR-4
  of `docs/archive/pre-bmad-spec/v1/cockpit-activity-audit-ledger-producer/tasks.md`.
