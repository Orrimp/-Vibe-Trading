---
slug: cockpit-activity-audit-ledger-producer
status: in-review
owner: tester
updated: 2026-05-27 (developer M-DEV — Waves A-D shipped; T-D-N1..T-D-N9 all ticked; HANDOFF → tester)
---

# Tasks — cockpit-activity-audit-ledger-producer

> Analyst M0 (2026-05-26) authored R1-R6 + R-NR + K1-K6 + H1-H3 + Q1-Q3
> + D1-D4 in [feature.md](feature.md). Architect M-T1 (2026-05-26) locks
> the three operator Qs at analyst-recommended defaults (per the
> `M-T1 lock` rows below), authors
> [ADR-0044](../architecture/adr/0044-activity-aggregator-pattern.md),
> and expands the Wave plan to the honest-tick contract each row must
> satisfy: Owner / Milestone / Depends on / Blocks / file:line / test
> cmd / expected output line.
>
> Decomp sequence: Wave A (`crates/agent::activity_audit_aggregator`)
> → Wave B (UI strings + label arm + cockpit_live wire-up + idle-end
> rebirth) → Wave C (criterion benches, R5.1 + R5.2 K3-discharge) →
> Wave D (10k storm + K2 9999+ truncation). Wave A blocks B and C;
> B and C are parallel-safe; Wave D depends on Wave A.

## M0 — Analyst synthesis

_owner: analyst_

- [x] **T-AN-0** (2026-05-26) — feature.md authored at v0.1.0 with
  R1-R6 + R-NR.1-7 + H1-H3 + K1-K6 + Q1-Q3 + D1-D4. Analyst-recommended
  defaults set on all 3 Qs. Aggregation policy chosen
  (per-time-window 100 ms + long-lived handle with idle-end).
- [x] **T-AN-1** (2026-05-26) — tasks.md scaffolded.
- [x] **T-AN-2** (2026-05-26) — Backlog Active row added at
  [`spec/backlog.md`](../backlog.md).
- [x] **T-AN-3** (2026-05-26) — Trace row
  `REQ-COCKPIT-ACTIVITY-AUDIT-LEDGER-001` opened at EOF of
  [`spec/trace.toml`](../trace.toml).

## M-OD — Operator decides (Q1-Q3)

_owner: operator. AskUserQuestion-routed by orchestrator._

All three Qs are **Autoapprove-eligible** at the analyst-recommended
defaults per the feature.md § Open questions block. Architect M-T1
applies the defaults under the standing "Autoapprove cosmetic /
analyst-default" rule (orchestrator confirmed 2026-05-26). Operator can
override at any point before M-FINAL ship — the rollback diff is
~30 LOC across 3-4 files per feature.md § D4.

- [x] **T-OP-1** (2026-05-26, architect Autoapprove) — Q1 aggregation
  policy = (b) per-time-window 100 ms.
- [x] **T-OP-2** (2026-05-26, architect Autoapprove) — Q2 label
  content = (a) redacted "Audit: N writes".
- [x] **T-OP-3** (2026-05-26, architect Autoapprove) — Q3 failure
  handling = (a) continue aggregator + sibling Failed event.

## M-T1 — Architect decomposition

_owner: architect._

- [x] **T-AR-1** (2026-05-26) — **Lock Q1 = (b) per-time-window 100 ms
  aggregation.** Cited cadence: parent
  [ADR-0042 § D1.4](../architecture/adr/0042-cockpit-activity-broadcast.md)
  pins the `ActivityHandle::tick` producer-side throttle to 100 ms (the
  19.84 ns/call P99 measurement that gates the activity channel's
  fan-out budget). The aggregator's 100 ms `tokio::time::interval`
  cadence is the **same wall-clock window** — one timer per process,
  shared semantically with every other producer on the bus, so the
  status-bar render budget stays unchanged (≤ 10 events/sec per
  long-lived handle). Rejected alternatives recorded in ADR-0044
  § Alternatives: per-batch (1 event per journal txn) → 10-50 ev/sec
  on a fast backtest; per-entity (one handle per `AuditEvent` variant)
  → can't fit in the status bar's max-3-row R2.2 budget.
- [x] **T-AR-2** (2026-05-26) — **Lock aggregator module home =
  `crates/agent/src/activity_audit_aggregator.rs`**. The aggregator
  is a PRODUCER (`bus.activity().start(AuditLedgerWrite, …).tick(N)`
  on every non-empty 100 ms window) per the cockpit-activity D5
  producer-side semantic. It belongs cohesive with the existing
  `crates/agent/src/activity.rs` (`ActivitySender` + `ActivityHandle`)
  — siblings under the same crate that owns the `EventBus`. Filename
  matches the analyst's D1 `activity_audit.rs` (architect kept the
  longer `activity_audit_aggregator.rs` to make the "this is the
  aggregator producer module, not the audit-tick consumer" intent
  unambiguous to future readers; alias not needed). Rejected:
  `crates/ui/src/audit_activity_bridge.rs` — UI is a SUBSCRIBER, not
  a producer; ADR-0044 § D1 codifies the producer-side placement.
- [x] **T-AR-3** (2026-05-26) — **Lock Q3 = separate-handle Failed
  emission** (not main-handle `fail()`). On any post-commit error path
  (today these emit nothing — the existing `tick::emit` only fires on
  `commit().await.is_ok()`, see `crates/audit/src/tick.rs:138`), the
  aggregator's main long-lived handle stays in `Success` state — it
  faithfully reflects the writes that DID land. A separate, transient
  `bus.activity().start(AuditLedgerWrite, "Audit: write failed").fail(reason)`
  handle is spawned by a **future** caller-side observer (out-of-scope
  at v0.1.0 — the tick bus only carries successful commits). The hook
  is documented in ADR-0044 § D4 so the v0.1.1 K-switch-trip /
  partial-commit producer wiring (when it lands) knows to use the
  sibling-handle shape. **At v0.1.0 the aggregator emits zero Failed
  events on the happy path** — verified by Wave D T-D-N9 (assertion
  `no_failed_events_on_happy_path`).
- [x] **T-AR-4** (2026-05-26) — **ADR-0044 authored** at
  [`spec/architecture/adr/0044-activity-aggregator-pattern.md`](../architecture/adr/0044-activity-aggregator-pattern.md).
  Locks the "aggregator producer pattern" (broadcast-receiver +
  `AtomicU32` counter + `tokio::time::interval` + long-lived
  `ActivityHandle` with idle-end semantics) as reusable for future
  high-frequency event sources. Registry entry added at
  [`spec/architecture/adr/README.md`](../architecture/adr/README.md)
  + Changelog row.
- [x] **T-AR-5** (2026-05-26) — Frontmatter flipped
  `owner: analyst → developer`; `status: draft → in-progress`. Trace
  row `arch` column populated with ADR-0044 + tasks.md anchor.

## M-DEV — Developer execution

_owner: developer. Wave-parallelizable per
[ADR-0044 § D1](../architecture/adr/0044-activity-aggregator-pattern.md)._

> Wave A blocks Wave B + Wave C (everyone needs the aggregator module
> to exist). Wave B and Wave C run in parallel once T-D-N3 lands.
> Wave D depends on Wave A only and can start concurrent with B.

### Wave A — `crates/agent::activity_audit_aggregator` (blocks Wave B + Wave D)

- [x] **T-D-N1** — NEW `crates/agent/src/activity_audit_aggregator.rs`
  (~150 LOC). Aggregator struct + private worker loop + public
  `spawn_aggregator(ledger: &Arc<Ledger>, bus: &EventBus) -> JoinHandle<()>`.
  - Owner: developer • Milestone: M-DEV • Depends on: T-AR-4 •
    Blocks: T-D-N2, T-D-N4, T-D-N6, T-D-N8.
  - file:line target: `crates/agent/src/activity_audit_aggregator.rs:1-150`.
  - Internal shape per ADR-0044 § D2:
    ```
    rx: broadcast::Receiver<AuditTick<AuditEvent>>  // ledger.tick_bus.subscribe()
    counter: AtomicU32                              // fetch_add(1, Relaxed) per recv
    handle: Option<ActivityHandle>                  // long-lived; dropped on idle window
    interval: tokio::time::interval(Duration::from_millis(100))
    bus: ActivitySender                             // cloned from EventBus::activity()
    ```
  - Worker loop: `tokio::select!` over `rx.recv()` (Ok → fetch_add;
    Lagged → tracing::warn + counter += n; Closed → break) and
    `interval.tick()` (snapshot counter via `swap(0)`; if N == 0 and
    handle is Some → drop handle (idle-end); if N > 0 and handle is
    None → `bus.start(AuditLedgerWrite, "Audit: 0 writes")` then
    `handle.tick(N)`; if N > 0 and handle is Some → `handle.tick(N)`).
  - K2 truncation: if N > 9999 the label format flips to
    `"Audit: 9999+ writes"`; aggregator's internal counter still tracks
    the precise total (operator can drill via metrics if needed).
  - Re-export at `crates/agent/src/lib.rs`:
    `pub use activity_audit_aggregator::spawn_aggregator;`.
  - Test cmd: `cargo build -p agent` (compile gate; full tests follow
    at T-D-N2).
  - Expected: build clean, no warnings; `clippy -D warnings` PASS.

- [x] **T-D-N2** — NEW `crates/agent/tests/activity_audit_aggregator.rs`
  (4 unit tests).
  - Owner: developer • Milestone: M-DEV • Depends on: T-D-N1 • Blocks: T-T1.
  - file:line target: `crates/agent/tests/activity_audit_aggregator.rs:1-220`.
  - Tests:
    1. `aggregator_emits_one_tick_per_window` — fire 500 synthetic
       `AuditEvent::Fill` ticks across a 350 ms span; assert exactly
       3 activity-channel `Tick` events arrive (one per 100 ms
       boundary), with the last `Tick.current = N_total_observed` per
       window, plus exactly 1 `Start` and 1 `End { Success }`.
    2. `aggregator_idle_drops_handle` — push 1 tick, wait 250 ms
       (≥ 2 empty windows); assert the channel sees `Start`, exactly
       1 `Tick`, then `End { Success }`; aggregator task remains
       alive (JoinHandle not yet finished).
    3. `aggregator_handle_resumes_after_idle` — burst → 250 ms quiet
       → burst; assert `ActivityId` differs between bursts (proving
       idle-end fired and a fresh handle was spawned on the second
       burst).
    4. `aggregator_panic_isolated` (K5 falsifier) — inject a panic in
       a synthetic broadcast receiver (poison-pill `AuditEvent`); assert
       the worker logs a `tracing::warn` and continues; aggregator
       JoinHandle stays alive; subsequent ticks still increment the
       counter. (If the worker design is robust enough that this is
       unreachable, document it inline and skip with a `#[ignore]`
       + comment — but the test must compile.)
  - Test cmd: `cargo test -p agent --test activity_audit_aggregator`.
  - Expected: `test result: ok. 4 passed; 0 failed`.

### Wave B — UI wire-up (parallel with Wave C; depends on T-D-N1)

- [x] **T-D-N3** — EDIT `crates/ui/src/strings.rs`: add
  `pub const ACTIVITY_KIND_AUDIT_LABEL: &str = "Audit";` and
  `pub const ACTIVITY_AUDIT_COUNT_FORMAT: &str = "{N} writes";`
  (R-NR.4: no inline literals). Add `ACTIVITY_AUDIT_FLOOD_TRUNCATION: &str = "9999+ writes"`
  for the K2 path.
  - Owner: developer • Milestone: M-DEV • Depends on: T-D-N1 •
    Blocks: T-D-N4, T-D-N5.
  - file:line target: `crates/ui/src/strings.rs` (append after the
    existing `ACTIVITY_KIND_TRAINING_LABEL` block).
  - Test cmd: `cargo build -p ui`. Expected: build clean.

- [x] **T-D-N4** — EDIT `crates/ui/src/widgets/activity_tape.rs`:
  add the `ActivityKind::AuditLedgerWrite` arm to
  `activity_kind_label` and to the icon mapper. Match the existing
  `LabRun` / `Training` arm style verbatim.
  - Owner: developer • Milestone: M-DEV • Depends on: T-D-N3 •
    Blocks: T-D-N5, T-T1.
  - Test cmd: `cargo test -p ui --lib widgets::activity_tape` (existing
    test suite must pass; add a new test
    `audit_ledger_label_renders_correctly` asserting the label string
    matches `ACTIVITY_KIND_AUDIT_LABEL`).
  - Expected: existing 6 widget tests + 1 new = 7 passed.

- [x] **T-D-N5** — EDIT `crates/ui/src/bin/cockpit_live.rs`: wire
  `spawn_aggregator(&ledger, &bus)` AFTER the `iced::Subscription`
  boot (K6 ordering mitigation per feature.md § K6). Hold the
  returned `JoinHandle` on `AppState` (or a sibling shutdown-coordinator
  struct) for graceful task abortion on cockpit exit.
  - Owner: developer • Milestone: M-DEV • Depends on: T-D-N1, T-D-N3 •
    Blocks: T-D-N9, T-T1.
  - file:line target: `crates/ui/src/bin/cockpit_live.rs` (find the
    block right after `let bus = EventBus::new();` + `Ledger::open`;
    add the `spawn_aggregator` call there but **after** the
    `iced::application(...).subscription(...)` is staged for the
    runtime — wait for the iced startup to settle).
  - Test cmd: `cargo build --bin cockpit_live --release`. Expected:
    build clean; no warnings.
  - Smoke validation: run `cargo run --bin cockpit-smoke` (existing
    smoke harness) and confirm 0 panics + the activity tape renders
    audit events when a backtest is dispatched.

### Wave C — Perf gates (parallel with Wave B; depends on T-D-N1)

- [x] **T-D-N6** — NEW criterion bench
  `crates/agent/benches/activity_audit.rs` with **3 micro-benches**:
  - Owner: developer • Milestone: M-DEV • Depends on: T-D-N1 •
    Blocks: T-T3.
  - file:line target: `crates/agent/benches/activity_audit.rs:1-180`.
  - **`aggregator_counter_increment_per_tick`** — measures
    `AtomicU32::fetch_add(1, Relaxed)` overhead in the worker's hot
    `rx.recv() → counter +=` path. Budget per R5.1: **< 100 ns/tick**
    (target ~50 ns on Apple Silicon).
  - **`aggregator_interval_tick_fan_out`** — measures the cost of one
    100 ms boundary fan-out (`counter.swap(0)` + `handle.tick(N)` +
    activity-channel send). Budget per R5/R6: **< 1 µs per window**
    (handle.tick is already 19.84 ns/call P99 per ADR-0042 § D1.4;
    add ~500 ns for the swap + branch logic).
  - **`aggregator_idle_end_transition`** — measures the cost of the
    idle-end window: counter == 0, handle is Some → `drop(handle)` →
    emits `End { Success }` over the channel. Budget per R5/R6:
    **< 100 µs per transition** (one-shot path, dominated by the
    broadcast send + the next-window allocation when a fresh handle
    spawns).
  - Cargo.toml addition: register the bench in `[[bench]]` block of
    `crates/agent/Cargo.toml` (`name = "activity_audit"`,
    `harness = false`).
  - Test cmd: `cargo bench -p agent --bench activity_audit`.
  - Expected: 3 benches PASS absolute budget (the criterion HTML
    report at `target/criterion/.../report/index.html` shows green
    bars within budget; copy P50/P99/Mean into the M-FINAL test
    report).

- [x] **T-D-N7** — Anchor-replay parity bench (R5.2 / H1 falsifier;
  the **K3-discharge gate**):
  - Owner: developer • Milestone: M-DEV • Depends on: T-D-N1, T-D-N5 •
    Blocks: T-T3.
  - Mechanism: run the `top10-2024-fy-momentum-bs1` anchor end-to-end
    via the existing `cargo run --bin backtest --features realdata --
    --scenario top10-2024-fy-momentum-bs1` path, once WITHOUT the
    aggregator subscribed (control) and once WITH the aggregator
    subscribed (treatment). Wrap both runs in a criterion-driven
    repeated-measurement harness (10 reps each, median of medians).
  - Budget per R5.2 / H1: **wall-clock divergence < 1 % at p99**.
    Falsifier exit (architect-stoppable): if divergence ≥ 1 %, the
    aggregator design is wrong on this axis; halt M-FINAL ship and
    file a recovery brief.
  - This bench is the **single most important gate of this feature**
    — it is the contract that justifies enabling the aggregator on
    every cockpit boot.
  - Test cmd: `cargo bench -p agent --bench activity_audit -- aggregator_anchor_replay_parity`.
  - Expected: median wall-clock delta ≤ 0.5 %; p99 ≤ 1.0 %.

### Wave D — Storm integration test (parallel with Wave B + C; depends on T-D-N1)

- [x] **T-D-N8** — NEW integration storm test
  `crates/ui/tests/activity_tape_audit_ledger_event_storm.rs`. Push
  10 000 synthetic `AuditTick<AuditEvent>` events at the audit bus's
  max rate (no sleeps; tight `tick::emit_public` loop); subscribe the
  aggregator + a sibling activity-channel receiver.
  - Owner: developer • Milestone: M-DEV • Depends on: T-D-N1, T-D-N5 •
    Blocks: T-T2, T-T3.
  - Assertions:
    1. **Counter completeness**: the aggregator's internal counter
       observes **all 10 000 events** (sum across the duration of
       the test).
    2. **Activity-channel rate cap**: the sibling receiver observes
       ≤ `(elapsed_ms / 100) + 1` `Tick` events (one per 100 ms window
       + 1-boundary-flake allowance from parent `cockpit-activity-status-bar`
       T-D-N3 throttle test precedent).
    3. **Zero Failed events**: per T-AR-3 — no main-handle `fail()`,
       no sibling-handle Failed emissions. The integration storm only
       carries Ok ticks (no error events on the tick bus today).
    4. **K2 truncation observed**: at least one `Tick.current` value
       exceeds 9 999 → the rendered label flips to
       `"Audit: 9999+ writes"`.
  - file:line target: `crates/ui/tests/activity_tape_audit_ledger_event_storm.rs:1-180`.
  - Test cmd: `cargo test -p ui --test activity_tape_audit_ledger_event_storm`.
  - Expected: `test result: ok. 1 passed; 0 failed` (single
    test function, 4 assertions).

- [x] **T-D-N9** — Happy-path Failed-event invariant test
  `crates/agent/tests/activity_audit_no_failed_events.rs`. Run a
  60 s synthetic backtest with the aggregator subscribed; subscribe
  a sibling activity-channel receiver and filter for
  `ActivityPhase::End(ActivityOutcome::Failed(_))`. Assert the count
  is **exactly 0** for the entire run (T-AR-3 invariant — the
  aggregator's main handle never flips to Failed; sibling Failed
  handles are wired in v0.1.1+).
  - Owner: developer • Milestone: M-DEV • Depends on: T-D-N1, T-D-N5 •
    Blocks: T-T1.
  - Test cmd: `cargo test -p agent --test activity_audit_no_failed_events`.
  - Expected: `test result: ok. 1 passed; 0 failed`.

## M-FINAL — Tester gate

_owner: tester._

- [ ] **T-T1** — Run all M-DEV unit + integration tests:
  `cargo test -p agent && cargo test -p ui --test activity_tape_audit_ledger_event_storm`.
  Capture outputs into [feature.md](feature.md) § Verification.
- [ ] **T-T2** — `bash scripts/verify_anchors.sh` → expect
  `ANCHORS PASS  (34 / 34)`. Zero anchor migrations by construction
  (R-NR.1).
- [ ] **T-T3** — Run criterion benches T-D-N6 + T-D-N7:
  `cargo bench -p agent --bench activity_audit`. Capture P50/P99
  numbers; verify all budgets PASS (R5.1 < 100 ns/tick;
  R5.2 < 1 % wall-clock divergence at p99 — **K3-discharge gate**).
- [ ] **T-T4** — `python3 scripts/spec_lint.py spec/cockpit-activity-audit-ledger-producer/`
  → expect 0 NEW violation categories vs. main baseline.
- [ ] **T-T5** — Populate `tests` + `anchors` columns of trace row
  `REQ-COCKPIT-ACTIVITY-AUDIT-LEDGER-001`; flip `state = "passed"`.
- [ ] **T-T6** — Author `spec/cockpit-activity-audit-ledger-producer/reports/test-2026-MM-DD.md`
  per the `rust-test` skill template.

## M-PRESENTER — Sprint-review face

_owner: presenter._

- [ ] **T-P1** — Author
  `spec/cockpit-activity-audit-ledger-producer/presentations/cockpit-activity-audit-ledger-producer-2026-MM-DD.md`.
  Capture: (a) operator-visible UX change (a single "Audit: N writes"
  row appears on the status-bar tape during fast backtests / live
  trading; idle-ends after 5 s of quiet); (b) H3 hypothesis test —
  operator feedback "useful" vs "noise" recorded verbatim;
  (c) before/after screenshots of the activity tape under a moderate
  backtest. Includes rollback path (~30 LOC across 3-4 files per
  feature.md § D4).

## Cost estimate

| Wave  | Estimate          | Critical-path |
|-------|-------------------|---------------|
| M-OD  | 0 (Autoapprove)   | gates M-T1    |
| M-T1  | ~ 0.25 day        | gates M-DEV   |
| A     | ~ 0.5 day         | gates B + C + D |
| B     | ~ 0.5 day         | parallel C    |
| C     | ~ 1 day           | gates M-FINAL |
| D     | ~ 0.5 day         | parallel B + C |
| M-FINAL | ~ 0.5 day       | ship gate     |
| M-PRESENTER | ~ 0.25 day  | post-PASS     |
| **Total** | **~ 2-3 days end-to-end** | |

## Notes

- All times are wall-clock budget — actual coding is ~1-2 days; the
  rest is M-FINAL pass + M-PRESENTER ack-cycle.
- Cross-link to parent v0.1.0 [§ R5.2](../cockpit-activity-status-bar/feature.md#r5--out-of-scope-producers-v011)
  for the deferral history.
- Cross-link to the audit-tick-consumer-envelope brief (the upstream
  source for `AuditTick<AuditEvent>`) is the natural reading order for
  a developer picking this up cold:
  [`spec/audit-tick-consumer-envelope/feature.md`](../audit-tick-consumer-envelope/feature.md).
- ADR-0044 is the architectural lock; reading it is required before
  any code change to `crates/agent/src/activity_audit_aggregator.rs`.

## Architect M-T1 watch recipe (for long-running benches)

The Wave C anchor-replay parity bench (T-D-N7) can take 5-15 minutes
depending on disk speed (loads `top10-2024-fy-momentum-bs1` data twice
across 10 reps). Operator can monitor:

```bash
watch -n 5 'ls -la target/criterion/aggregator_anchor_replay_parity/ 2>/dev/null | tail -20 && echo "---" && tail -5 /tmp/audit-aggregator-bench.log 2>/dev/null'
```

Where `cargo bench -p agent --bench activity_audit > /tmp/audit-aggregator-bench.log 2>&1 &`
launches the bench in the background.
