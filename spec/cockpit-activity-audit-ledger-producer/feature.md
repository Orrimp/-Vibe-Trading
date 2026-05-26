---
slug: cockpit-activity-audit-ledger-producer
status: draft
owner: analyst
version: 0.1.0
updated: 2026-05-26
predecessor: cockpit-activity-status-bar v0.1.0
---

# Cockpit activity tape — audit-ledger-writes producer with aggregation envelope

> **Predecessor chain**: this brief sits downstream of
> [`cockpit-activity-status-bar v0.1.0`](../cockpit-activity-status-bar/feature.md)
> (shipped 2026-05-26) which landed the `ActivityKind::AuditLedgerWrite`
> enum variant unused (R5.2) + the `EventBus::activity()` channel +
> `ActivityHandle` RAII + status-bar tape rendering. It also depends on
> the existing `crates/audit/src/tick.rs` `AuditTick<AuditEvent>`
> broadcast (`audit-tick-consumer-envelope v0.1.0`) which already
> tees every post-commit writer (`post_fill`, `post_strategy_signal`,
> `kill_switch_tripped`, `strategy_event`, etc. — 7-8 call sites today).
> **The aggregator pattern in this brief does NOT add new taps in
> `crates/audit/src/journal.rs`** — it subscribes to the existing
> `tick_bus` and aggregates into the cockpit-activity channel.

## Why

### What v0.1.0 deferred (and why)

`cockpit-activity-status-bar v0.1.0` R5.2 + K3 explicitly deferred the
`ActivityKind::AuditLedgerWrite` producer:

> "Audit writes can fan-out at thousands per second during a fast
> backtest and the per-event throttle (R1.4) is the wrong place to
> enforce aggregation. The right shape is an 'audit writer: aggregate
> to one flushing activity per second' — defer to v0.1.1 where we can
> design the aggregator properly."

The naive wiring — one `ActivityHandle::start` per `tick::emit` call —
would generate 7-8 events per fill/signal pair (one per journal-writer
in the dual-write chain) and overwhelm the 256-slot `activity_tx` ring
buffer within milliseconds of a moderately-active backtest. The
producer-side 100 ms `ActivityHandle::tick` throttle (v0.1.0 R1.4)
caps a single handle to 10 events/sec, but does NOT cap **handle
creation** — and the audit writers are short-lived
(`start → commit → drop`), each spawning a fresh handle.

### State today

- The `tick_bus` broadcast in `crates/audit/src/tick.rs` already fans
  out post-commit `AuditTick<AuditEvent>` envelopes to in-memory
  consumers (the reflection journal builder, the cockpit Memory drawer
  reader, the operator-success report). Capacity 1024, drop-on-lag.
- The variants today (`Fill`, `StrategySignal`, `StrategyEvent`,
  `ForecastEmitted`, `KillSwitchTripped`, `FeedReconnect`,
  `UptimeIntervalOpened`, `UptimeIntervalClosed`, `LlmForecastEmitted`)
  cover every writer the operator would want to see "in flight" — they
  are the natural upstream for an aggregator.
- The cockpit activity tape (status bar tape from v0.1.0) renders
  `Yahoo preload`, `Lab Run`, `Training subprocess`. The audit-ledger
  writes are visually absent — operator has no signal that a moderate
  backtest is producing 4000 SQL writes/sec until a slow disk shows up
  as kept-alive p99 latency.

### Why now

v0.1.0 of the parent feature surfaced the K3 risk in writing, and the
v0.1.1 follow-on slot is the natural place to design + ship the
aggregator before the v3-llm-forecaster + v3-volatility-overlay live
trading paths multiply the audit-write rate by another order of
magnitude. We want the visibility BEFORE the rate climbs, not after.

## Requirements

R1-R6 explore the aggregation policy space; K1-K4 enumerate risks;
H1-H3 are the testable hypotheses; Q1-Q3 are operator-decide gates with
analyst-recommended defaults.

### R1 — Aggregation policy: per-time-window (default) vs per-batch vs per-entity

**Three candidate shapes**, each with different "what does the operator
see" trade-offs:

- **R1.a — per-time-window (analyst recommended)**: aggregate every
  `AuditTick` arriving within a 100 ms wall-clock window into a single
  in-flight `ActivityHandle`. Tick events on the activity channel are
  emitted at the window boundary with `current_units = N` where N is
  the number of audit ticks observed. The handle is long-lived —
  re-used across consecutive non-empty windows, ended after the first
  empty window.
  - **Why this is the analyst default**: aligns with the existing
    `ActivityHandle::tick` 100 ms producer-side throttle (parent R1.4)
    — same cadence the rest of the activity tape already uses. The
    aggregator is one timer + one counter + one handle reference.
- **R1.b — per-batch (rejected)**: aggregate by the natural
  `journal_transactions` write boundary — one event per "transaction
  header + N entries" group. This matches the dual-write contract
  semantics (atomic per transaction) but generates 1 event per fill
  (currently ~10-50 per second on a fast backtest) — still too
  chatty for the status-bar render budget, and not aligned with the
  existing 100 ms cadence.
- **R1.c — per-entity (rejected at v0.1.0)**: one in-flight handle per
  `AuditEvent` variant (Fill / StrategySignal / KillSwitchTripped / …),
  ticking the matching handle on every event of that kind. Gives the
  operator a "which subsystem is hot" view but multiplies handle count
  by N-variants (9 today, `#[non_exhaustive]`); the status bar's
  max-3-visible-rows (parent R2.2) can't render more than 3
  simultaneously without overflow chip. Defer to v0.2.0 if operator
  surfaces a use case.

Operator picks the policy at Q1 — analyst default R1.a.

### R2 — Label format

- **R2.1 — redacted (analyst recommended)**: render the aggregated
  handle's label as `"Audit: {N} writes"` where `N` is the count of
  ticks aggregated into the most recent window. Per Q2 default;
  preserves the K1 PII contract — no operator-visible mention of
  strategy ID, symbol, fill price, reason string. Operator drills into
  the actual audit ledger (Memory drawer, reflection builder, or `sqlite`
  cli on the audit DB) if they want the detail.
- **R2.2 — verbose (rejected)**: render label as
  `"Audit: KillSwitchTripped"` or `"Audit: StrategySignal BTCUSDT buy 100"`.
  The label is at most 64 chars (parent R1.2) and a `StrategySignal`
  with a full reason string blows past the budget; truncation surfaces
  inconsistent labels. The verbose label also leaks venue/symbol/
  strategy_id into the status-bar tape — which is visible on every
  cockpit screen — without the operator opting in. **Hard veto on PII
  grounds** (parent K4 generalised: any operator-facing surface that
  shows venue + symbol + side + reason is a screenshot leak vector).
- **R2.3 — kind-mix summary (forward-listed)**: render
  `"Audit: 4 Fill · 1 Signal · 1 KillSwitch in 100ms"`. Higher info
  density but ALSO higher label length + complexity. Forward-list to
  v0.2.0 once operator confirms R2.1 is too coarse.

Operator picks at Q2 — analyst default R2.1.

### R3 — Activity lifecycle: when does the aggregated handle "end"

- **R3.1 — long-lived with idle-end (analyst recommended)**: a single
  `ActivityHandle` for `ActivityKind::AuditLedgerWrite` exists as long
  as audit ticks are arriving. On the first 100 ms window that observes
  **zero** ticks, the aggregator drops the handle (which emits
  `End { Success }` via Drop). The next non-empty window starts a fresh
  handle. This gives the operator the "audit is currently active" /
  "audit is quiet" boolean for free, and respects the v0.1.0
  parent R2.5 200 ms render-floor — single-tick bursts get rendered
  because the handle stays alive ≥ 200 ms whenever there's a follow-up.
- **R3.2 — short-lived per batch (rejected)**: start + end one handle
  per 100 ms window. This emits 30 events/sec to the activity channel
  in steady-state (1 Start + 1 Tick + 1 End per window) which is
  3× more chatty than necessary. R3.1 emits at most 10 events/sec
  (1 Tick per window) in the same scenario.
- **R3.3 — strategy-boundary-tied (rejected)**: one handle per
  strategy `Bar` cycle; ends when the bar's signal+fill chain
  completes. Requires the aggregator to track the bar boundary —
  cross-crate coupling between `crates/audit` and `crates/backtest`
  that v0.1.0 explicitly avoided.

Cross-batch correlation (K4) is out-of-scope at v0.1.0 — the
aggregator does NOT thread a `bar_id` or `run_id` through the activity
channel. The label is purely a count-of-writes.

### R4 — Failure handling

If any single SQL write inside the aggregation window fails, the
existing `AuditTick<AuditEvent>` broadcast does NOT carry the error
(it only fires on `db_txn.commit().await.is_ok()`). The aggregator
sees the post-commit event stream — failures are silent from its
perspective. Two shapes:

- **R4.1 — continue + sibling Failed event (analyst recommended)**:
  the aggregator's long-lived handle stays in `Success` state — it
  reflects the writes that DID land. A separate, transient
  `ActivityHandle::start(AuditLedgerWrite, "Audit: write failed")` is
  spawned by the failure observer at the **caller** site
  (`Result::Err` from `journal::post_*`) and immediately
  `handle.fail(reason)`-then-dropped, producing the red 3-second hold
  on the tape per parent R2.5.
  - **Operator UX**: the steady "Audit: 12 writes" tape entry stays
    green; a separate "Audit: write failed" red row appears for 3 s.
- **R4.2 — flip aggregated handle to Failed (rejected)**: one failure
  taints the entire window's writes red. Misleading — the other 11
  writes in the window succeeded.

Per Q3 default. Q3 forwards to operator.

### R5 — Performance budget

**Hard requirement**: enabling the aggregator MUST NOT slow audit
writes by > 1 % at the existing `tick::emit` call sites (parent K3
verbatim). Path is:

```
journal::post_fill (commits SQL)
  → crate::tick::emit (broadcast::send into tick_bus)
  → aggregator's broadcast::Receiver wakes
  → AtomicU32::fetch_add(1, Relaxed)    [the only sync work]
  → window timer Drop / fresh start
```

The aggregator's hot path is one `fetch_add` per tick — sub-50 ns. The
broadcast::send is already on the audit writer's hot path today (cost
amortised in v0.1.0 of `audit-tick-consumer-envelope`). No new
allocation; no new syscall.

- **R5.1** Criterion bench `audit_aggregator_overhead_per_tick`
  measures the cost added by the aggregator's subscriber compared to a
  no-subscriber baseline. Budget: < 100 ns/tick.
- **R5.2** Anchor-replay parity bench: run the `top10-2024-fy-momentum-bs1`
  anchor end-to-end WITH and WITHOUT the aggregator subscribed. Budget:
  wall-clock divergence < 1 % at p99. **This is the K3-discharge gate**.

### R6 — Aggregator placement: new module + recipe

- **R6.1** New `crates/agent/src/activity_audit.rs` (sibling of the
  v0.1.0 `crates/agent/src/activity.rs`) — owns the aggregator
  worker. Spawns a tokio task; subscribes to `Arc<Ledger>::tick_bus`;
  ticks an `AtomicU32` counter; reads on a `tokio::time::interval(100ms)`
  cadence; promotes counter → `ActivityHandle::tick(N)` via the
  `EventBus::activity()` channel. Worker exits when both the audit
  ledger AND the EventBus are dropped.
- **R6.2** Wire-up site: the cockpit `cockpit_live.rs` binary
  constructs the `Ledger` and the `EventBus` at startup; immediately
  after construction it spawns the aggregator via a single new
  function call: `agent::activity_audit::spawn_aggregator(&ledger, &bus)`.
  The aggregator is opt-in — if the binary skips the call (e.g. an
  isolated test), no events flow. **No production binary
  default-skips**.
- **R6.3** NEW `bus.activity().start(ActivityKind::AuditLedgerWrite, …)`
  call site lives **inside the aggregator** — not in `journal.rs`,
  not in `ledger.rs`. The audit crate stays unaware of the activity
  tape. **Zero changes to `crates/audit/`** — anchor-additive by
  construction (parent R-NR.4 generalised: audit migration count
  unchanged; SQL write path unchanged).
- **R6.4** Acceptance tests:
  - `crates/agent/tests/activity_audit_aggregator.rs` — synthetic
    audit-ledger fixture; fire 500 `AuditEvent::Fill` ticks across
    a 350 ms window; assert exactly 3 `ActivityHandle::tick` emits
    arrive (one per 100 ms boundary) + 1 `End { Success }` after the
    idle window.
  - `crates/agent/tests/activity_audit_aggregator_idle.rs` —
    no audit ticks ever fire; assert zero events on the
    `EventBus::activity()` channel.

### R-NR — Non-regression contract

- **R-NR.1** All 34 anchors stay byte-identical. Zero changes to
  `crates/backtest/`, `crates/strategy/`, `crates/exec/`,
  `crates/risk/`, `crates/reports/`, `crates/forecast/`, `crates/audit/`.
  Producer wiring lives entirely in `crates/agent/src/activity_audit.rs`
  (NEW) + a single `spawn_aggregator()` call in
  `crates/ui/src/bin/cockpit_live.rs`.
- **R-NR.2** No new audit migration. Aggregator subscribes; does not
  write.
- **R-NR.3** No new Lumen tokens. Reuse `ActivityKind::AuditLedgerWrite`
  variant (already added at v0.1.0 R5.2).
- **R-NR.4** No new operator-facing strings beyond
  `ACTIVITY_KIND_AUDIT_LABEL` and the `"{N} writes"` format template
  in `crates/ui/src/strings.rs`. **Zero inline literals.**
- **R-NR.5** No external dependency. Reuses `tokio::sync::broadcast`,
  `tokio::time::interval`, `AtomicU32` — all already in workspace.
- **R-NR.6** `cockpit-smoke` 0 panics. The aggregator is a single
  background tokio task; cold-start state is "no subscribers, no
  events" — same empty-cold-start contract as parent R2.7.
- **R-NR.7** Test count grows by 4-5 new tests (R6.4 + R5 benches +
  hypothesis-falsifier tests).

## Hypothesis register

- **H1 — Aggregator overhead is < 1 % of audit-write wall-clock.**
  Falsifier: R5.2 anchor-replay bench shows > 1 % divergence at p99
  with the aggregator subscribed. **Assumed TRUE** — the aggregator's
  hot path is one `AtomicU32::fetch_add` per tick; this is
  sub-50 ns on Apple Silicon, well under 1 % of even the fastest
  `db_txn.commit()` path (which is multi-µs).
- **H2 — 100 ms aggregation window keeps the activity channel from
  saturating under a fast-backtest write rate.** Falsifier: spawn a
  synthetic 4000 Hz `AuditEvent` storm; observe `RecvError::Lagged`
  on the `EventBus::activity()` channel within < 2 s. **Assumed TRUE**
  — 100 ms window with a 256-slot ring means the channel needs > 25 s
  of UI stall to see Lagged, same calculation as parent R6.3.
- **H3 — Operator finds the "Audit: N writes" label useful and not
  visual noise.** Falsifier: presenter sprint-review captures "remove
  this" or "I ignore it". Untestable in code; documented for the
  presenter to capture.

## Risk register

- **K1 — PII / venue secret leak via the label.** Even the redacted
  "Audit: 12 writes" surface could leak rate-information (e.g. a venue
  outage signature — sudden zero writes). **Mitigation**: at v0.1.0
  the label carries only the count, not the venue or strategy_id.
  Operator-only (no remote screensharing default-on). Forward-listed
  R2.3 kind-mix label is opt-in. **Severity LOW** — the rate is
  already observable via `metrics::counter!("audit_tick_emitted_total")`
  in any case.
- **K2 — Flood mitigation invariance under burstiness.** If 10 000
  audit writes arrive in a single 1 ms span, the AtomicU32 absorbs
  them but the next 100 ms window emits a "10000 writes" label that
  exceeds 64 chars. **Mitigation**: cap the displayed count at 9999
  (label budget); aggregator's internal counter still tracks the
  precise total; truncation copy is `"Audit: 9999+ writes"`.
- **K3 — Deduplication across the dual-write chain.** A single
  `kill_switch_tripped` emits ONE `AuditEvent::KillSwitchTripped` tick
  (the call site emits one event after the transaction commits — see
  `journal.rs:1002`). So no dedup needed at v0.1.0. **But** the
  forward-listed R2.3 kind-mix surface would need to deduplicate the
  memo-row from the strategy-events-row write inside a single
  transaction. Defer to v0.2.0; document the trap.
- **K4 — Cross-batch correlation lost.** R3 picks a long-lived handle
  that re-uses the same `ActivityId` across consecutive non-empty
  windows. The operator sees one "Audit: …" row in the tape even when
  the underlying writes are from two different `bar`s. **Mitigation**:
  v0.1.0 acceptable — the activity tape is a "what is happening NOW"
  surface, not a per-bar drill-down. Forward-list to v0.2.0 if
  operator complains.
- **K5 — Aggregator task panics silently.** The tokio task lives for
  the cockpit process lifetime; a panic during the broadcast `recv()`
  or the `interval.tick().await` would kill the aggregator silently
  unless we wrap with `tokio::task::JoinHandle` polling. **Mitigation**:
  wrap the worker in `tokio::spawn` + `tracing::warn` on the
  `JoinHandle::await` failure path (analogous to the parent
  `ActivityRecipe` Lagged handling). Add `H3` falsifier test.
- **K6 — Order-of-construction race at startup.** If the aggregator
  spawns before subscribers attach to `EventBus::activity()`, the
  first burst of audit ticks is fanned out to zero subscribers and
  silently dropped. The cockpit UI subscribes via `ActivityRecipe`;
  the aggregator is spawned at `cockpit_live.rs` startup. Order matters.
  **Mitigation**: spawn the aggregator AFTER the iced runtime starts
  the `Subscription` lifecycle. Document in tasks.md at M-DEV.

## Open questions for the operator

Three Qs, each with an analyst-recommended default. All three are
**Autoapprove-eligible** at the defaults; Q1 is the load-bearing
mechanism choice and Q2 is the visible UX choice — architect should
escalate at M-T1 if the operator hasn't ACK'd.

- **Q1 — Aggregation policy.**
  - (a) per-batch (one event per `journal_transactions` write).
  - **(b) per-time-window 100ms (R1.a) ← ANALYST DEFAULT**
  - (c) per-entity (one handle per `AuditEvent` variant).
- **Q2 — Label content.**
  - **(a) redacted ("Audit: N writes") (R2.1) ← ANALYST DEFAULT**
  - (b) verbose ("Audit: KillSwitchTripped" / per-event).
  - (c) kind-mix summary ("4 Fill · 1 Signal · 1 KillSwitch in 100ms").
- **Q3 — Failure handling.**
  - **(a) continue aggregator + sibling Failed event (R4.1) ←
    ANALYST DEFAULT**
  - (b) flip aggregated handle to Failed on any inner write error.

## Design

This section is the architect's M-T1 work. Captured here as the
analyst's M0 sketch.

### D1 — Crate layout

| Crate | What it owns at v0.1.0 |
|-------|--------------------------|
| `crates/agent` | NEW `src/activity_audit.rs` (~150 LOC): `Aggregator`, `spawn_aggregator(&Ledger, &EventBus) -> JoinHandle`, internal worker loop. Re-export at `lib.rs`. |
| `crates/ui` | EDIT `src/bin/cockpit_live.rs` (~5 LOC): wire the `spawn_aggregator` call after both `Ledger` + `EventBus` constructed. EDIT `src/strings.rs` (~2 LOC): `ACTIVITY_KIND_AUDIT_LABEL` + `ACTIVITY_AUDIT_COUNT_FORMAT`. EDIT `src/widgets/activity_tape.rs` (~5 LOC): add `ActivityKind::AuditLedgerWrite` arm to `activity_kind_label`. |
| `crates/audit` | **Zero changes.** Aggregator subscribes to the existing `tick_bus`. |

### D2 — Concurrency map

```
crates/audit::Ledger::tick_bus  (existing; broadcast::Sender<AuditTick<AuditEvent>>, capacity 1024)
                          │
                ┌─────────┴───────────────┐
                │                         │
   (existing) reflection journal     (NEW) agent::activity_audit::Aggregator
                │                            ├─ AtomicU32 counter
                │                            ├─ tokio::time::interval(100ms)
                │                            └─ on non-empty tick: bus.activity().start(...).tick(N)
                                                                              │
                                                                              ▼
                                              crates/agent::EventBus::activity_tx
                                                  (existing; capacity 256)
                                                                              │
                                                          (existing) ActivityRecipe → Cockpit::activity_tape
```

### D3 — ADR

- **ADR-NNNN — Audit-ledger activity aggregator** (architect M-T1
  authors at `spec/architecture/adr/00NN-audit-activity-aggregator.md`).
  Locks: 100 ms window, AtomicU32 counter, long-lived handle with
  idle-end semantics, the < 1 % overhead budget, and the zero-changes-
  to-crates-audit contract.

### D4 — Rollback path

- Single revert of the `spawn_aggregator()` call in `cockpit_live.rs`
  (~3 lines).
- Single revert of `crates/agent/src/activity_audit.rs` (delete file).
- Single revert of `widgets/activity_tape.rs` label arm + `strings.rs`
  constants.
- Total rollback diff: ~ 30 LOC across 3-4 files. Zero anchor changes.

## Backtest Scenarios

**None.** This brief is agent + UI only. Anchor risk zero by construction
(R-NR.1).

## Implementation

_developer fills this at Wave A_

## Verification

_tester links to reports here at M-FINAL_

## Changelog

- 2026-05-26 (analyst): authored v0.1.0 brief. R1-R6 + R-NR + K1-K6 +
  H1-H3 + Q1-Q3 + D1-D4 design closed. Analyst-recommended defaults
  set on all 3 Qs. Aggregator pattern (per-time-window 100ms + AtomicU32
  counter + long-lived handle with idle-end) chosen over per-batch /
  per-entity alternatives. Audit crate stays unchanged — aggregator
  subscribes to the existing `tick_bus`. HANDOFF → architect for M-T1.
