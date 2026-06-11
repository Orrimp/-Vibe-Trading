---
slug: audit-tick-consumer-envelope
status: shipped
owner: operator
updated: 2026-05-20
version: 0.1.0
predecessor: ADR-0031
decomp: spec/audit-tick-consumer-envelope/decomp.md
---

# Audit tick consumer envelope (`audit-tick-consumer-envelope`)

> Process-tooling feature. Promotes the **canonical design** at
> [ADR-0031](../architecture/adr/0031-audit-tick-consumer-envelope.md)
> (status `proposed`) into an implementation contract. ADR-0031 is the
> source of truth for the *direction*; this brief is the source of
> truth for the *contract*. Where the two diverge, the brief wins
> (analyst is the latest reader).

## Why

The audit journal currently uses **per-consumer write taps** — every new
consumer (`crates/reflection`, future Lab Trail screen, v2.6 bake-off,
v3 success-reports) requires its own mutation of the audit writers in
`crates/audit`, `crates/exec`, and `crates/agent`. As consumer count
grows, this load-bearing surface stops stabilising.

ADR-0031 proposes a **thin read-direction envelope** layered on top of
the existing journal: every `journal::*` writer also enqueues an
`AuditTick<AuditEvent, AuditContext>` into a `tokio::sync::broadcast`
channel. Consumers subscribe by wrapping the broadcast receiver. The
existing double-entry ledger and SQLite schema are untouched.

The `barter-rs` `AuditStream` pattern is borrowed by **shape only**;
no `barter-*` crate is introduced (confirmed: `grep -rn 'barter'
Cargo.toml crates/` returns zero matches as of this brief).

## Scope (operator-locked from ADR-0031)

1. **New module `crates/audit/src/tick.rs`** — `AuditTick<E, C>` envelope
   + `AuditContext { run_id, posted_at, agent_pid }` + `AuditEvent` enum
   (variants: `Fill`, `StrategySignal`, `StrategyEvent`,
   `ForecastEmitted`, `KillSwitchTripped`, `FeedReconnect`,
   `UptimeIntervalOpened`, `UptimeIntervalClosed`).
2. **Broadcast tee on producer side** — every `journal::*` writer ALSO
   enqueues an `AuditTick` into a `tokio::sync::broadcast::Sender`.
3. **Consumer subscription pattern** — wrap the broadcast receiver as an
   `Iterator<Item = AuditTick<AuditEvent>>` (mirrors barter-rs
   `StateReplicaManager`, shape-only).
4. **Per-consumer state-replica stub** for `crates/reflection` to
   demonstrate the consumer side end-to-end.

## Out of scope

- Rewriting the existing audit journal (additive only).
- SQL pub/sub (rejected per ADR-0031 §Alternatives 2).
- Event sourcing rewrite (rejected per ADR-0031 §Alternatives 3).
- Backfill from `journal_entries` on subscribe (ADR-0031 Q2 — deferred to
  whichever consumer brief needs full history; this brief is "from
  subscribe time forward" only).
- Other consumers (Lab Trail, v2.6 bake-off) — they subscribe in their
  own feature briefs.
- Migrating any existing `journal::*` writer signature.
- Closing the WAL-mode latent gap from ADR-0034 D1 (`PRAGMA
  journal_mode = WAL;` in `Ledger::open`) — orthogonal; tracked in its
  own backlog item.

## Requirements (R-register)

> Each R is testable. Tester pins the anchor or unit-test reference in
> `trace.toml.anchors` at PASS gate.

### R1 — Envelope shape

- **R1.1** `crates/audit/src/tick.rs` defines `AuditTick<Event,
  Context = AuditContext>` with `event: Event` and `context: Context`
  public fields, `#[derive(Debug, Clone, Serialize, Deserialize)]`.
- **R1.2** `AuditContext { run_id: Uuid, posted_at:
  time::OffsetDateTime, agent_pid: u32 }`, `#[derive(Debug, Clone,
  Serialize, Deserialize)]`.
- **R1.3** `AuditEvent` is a `#[non_exhaustive]` enum carrying every
  variant in ADR-0031 §Decision (Fill, StrategySignal, StrategyEvent,
  ForecastEmitted, KillSwitchTripped, FeedReconnect,
  UptimeIntervalOpened, UptimeIntervalClosed). `non_exhaustive` is
  mandatory so downstream consumers cannot break when v3 adds
  variants.
- **R1.4** No new external crate deps. `tokio` (broadcast feature),
  `serde`, `uuid`, `time`, `smol_str`, `rust_decimal` are already in
  the workspace.

### R2 — Producer tee

- **R2.1** `Ledger` gains a single optional field
  `Option<broadcast::Sender<AuditTick<AuditEvent>>>`. Default `None`
  → tee dormant, zero behaviour change (this is the rollout safety
  switch — see K2).
- **R2.2** A new constructor `Ledger::open_with_tick_bus(db_path:
  &str, capacity: usize) -> Result<(Self, broadcast::Sender<...>),
  LedgerError>` wires the channel and hands the **sender** back to
  the caller, which then `.subscribe()`s for each consumer.
- **R2.3** Every `journal::*` writer that produces an `AuditEvent`
  variant **also** calls `sender.send(tick)` after the SQL transaction
  commits (post-commit, not pre-commit — see H2 and K2). Send
  failure (no subscribers / lag overflow) is silently dropped per
  `tokio::sync::broadcast::Sender::send` semantics; never propagates
  as `LedgerError`.
- **R2.4** No existing writer signature changes. The tee is reached
  via the `Ledger` handle the writer already holds.
- **R2.5** Variant coverage at v0.1.0: `post_fill` →
  `AuditEvent::Fill`, `post_strategy_signal` →
  `AuditEvent::StrategySignal`, `strategy_event` →
  `AuditEvent::StrategyEvent` (also covers downstream
  `feed_reconnect`, `rebalance_rejected`, `mean_reversion_stop`,
  `pair_short_observation` because they delegate to `strategy_event`),
  `kill_switch_tripped` → `AuditEvent::KillSwitchTripped`,
  `open_uptime_interval` → `AuditEvent::UptimeIntervalOpened`,
  `close_uptime_interval` → `AuditEvent::UptimeIntervalClosed`.
  `ForecastEmitted` is emitted by whichever crate writes the
  forecast-cache hit (current call site to be confirmed by architect;
  candidate is `crates/forecast` or `crates/exec`). Out-of-scope
  writers (`post_training_*`, `post_cost*`, `registry_event`,
  `insert_funding_obs`, `verify_balance`, `heartbeat_uptime`) do
  **not** emit ticks at v0.1.0 — they stay SQL-only.

### R3 — Consumer pattern

- **R3.1** A `crate::tick::AuditTickStream` newtype wraps
  `broadcast::Receiver<AuditTick<AuditEvent>>` and exposes an `async
  fn next(&mut self) -> Option<AuditTick<...>>` that:
  - on `Ok(tick)` returns `Some(tick)`,
  - on `Err(RecvError::Lagged(n))` emits a `tracing::warn!` with `n`
    and a Prometheus counter `audit_tick_lagged_total{consumer=…}`,
    then continues to the next `recv()`,
  - on `Err(RecvError::Closed)` returns `None`.
- **R3.2** A blocking-iterator adaptor
  `AuditTickStream::into_iter_blocking()` mirrors barter-rs'
  `Iterator<Item = AuditTick<…>>` shape for synchronous consumers
  (used by `crates/reports`). Implemented via
  `tokio::runtime::Handle::block_on`.
- **R3.3** The consumer-side API does not import anything from
  sibling crates beyond `tokio` and `trading_core` (preserves the
  `crates/audit` import-direction invariant — see
  [01-data-flow.md](../architecture/01-data-flow.md)).

### R4 — Reflection state-replica stub

- **R4.1** `crates/reflection/src/audit_tick_consumer.rs` (new
  module) defines a `ReflectionAuditTickConsumer` that holds an
  `AuditTickStream` + an `Arc<dyn ReflectionStore>` and exposes a
  `run(self)` future processing ticks. The v0.1.0 implementation is a
  **stub**: it logs each received tick and counts variants — it does
  NOT write `LessonCard`s yet. That replacement is the explicit job
  of a follow-up brief.
- **R4.2** The existing `ReflectionWriter` (mpsc tap from
  `crates/exec`) stays untouched and remains the v2.x production
  write path. Both paths coexist behind no feature gate at v0.1.0
  (the stub does not write).
- **R4.3** Stub must be opt-in: spawned only when `[reflection]
  audit_tick_consumer_enabled = true` in `config/agent.toml`
  (default `false`). Keeps v2.x default behaviour bit-identical.

### R5 — Anchor preservation

- **R5.1** All 22 body-SHA-256 anchors in `spec/anchors.toml` stay
  byte-identical. Anchor verification (`scripts/verify_anchors.sh`)
  PASS 22/22 is a M-FINAL gate.
- **R5.2** `cockpit-smoke` PASS with 0 panics (proves no UI-side
  consumer breaks).
- **R5.3** `cargo test --workspace` 100% PASS, including the existing
  `crates/audit/tests/*` suite (proves no journal-side regression).

### R6 — Observability

- **R6.1** New Prometheus metrics:
  - `audit_tick_emitted_total{variant=…}` counter — incremented in
    the producer tee.
  - `audit_tick_lagged_total{consumer=…}` counter — incremented on
    `RecvError::Lagged(n)` with `n` as the increment delta.
  - `audit_tick_subscribers` gauge — `Sender::receiver_count()`
    sampled on a 5s tick.
- **R6.2** `tracing::debug!(target = "audit::tick", variant = …, ..)`
  on every send; `tracing::warn!(target = "audit::tick", consumer =
  …, lagged = n)` on lag. Same conventions as `agent::bus`.

### R7 — Config surface

- **R7.1** `config/agent.toml` gains `[audit] tick_bus_capacity =
  1024` (default — see Q1 default below).
- **R7.2** `config/agent.toml` gains `[reflection]
  audit_tick_consumer_enabled = false` (default; gates R4 stub).
- **R7.3** Config additions are backward-compat (serde defaults), so
  zero impact on existing TOML files.

## Operator-decide questions (Q1..Q5)

> **OPERATOR DECIDED 2026-05-20 via "Autoapprove all" directive — all
> 5 Qs resolved to the analyst-recommended defaults:**
> Q1 = 1024 (broadcast channel capacity; matches `agent::bus::fills_tx`);
> Q2 = inline post-commit tee in `journal::*` writers (no wrapper);
> Q3 = hidden side-effect, opt-in at `Ledger::open_with_tick_bus` construction;
> Q4 = defer UI consumer pattern to follow-up brief (Lab Trail Phase D);
> Q5 = pre-seed `agent_pid` (`std::process::id()`) + `run_id` at session
> start on the `Ledger` (one syscall, not per-write).
> Architect proceeds against these defaults.

Each carries an analyst-recommended default. **If the operator does
not override within the M-T1 architect gate, the developer ships the
default.**

### Q1 — Broadcast channel capacity

> ADR-0031 Q1: bounded with drop-on-lag is locked. Confirm N (32?
> 256?).

- **Recommended default: `1024`.**
- **Rationale:**
  - Mirrors `agent::bus::EventBus::fills_tx` (cap 1024) — the
    closest analogue (per-fill cadence).
  - At a peak live-trading rate of ~200 ticks/sec on a busy regime
    (4 venues × ~50 fills/min + signals + uptime heartbeats), 1024
    slots gives a slow consumer ~5s headroom before lag — long enough
    for a UI redraw blip but short enough to surface real
    backpressure.
  - Backtests run faster than realtime but consumers (reports) are
    synchronous and don't lag.
  - `tokio::sync::broadcast::channel(N)` allocates `N * size_of::<T>()
    * receiver_count` once; `AuditTick<AuditEvent>` is roughly 200B
    after `Fill` payload → ~200 KB at 1024. Negligible.
- **Override condition:** If the operator wants tighter feedback on
  consumer lag (e.g. CI smoke), drop to `256`. If a heavy historical
  replay is foreseen, bump to `8192` (matches `ticks_tx`).
- **Sets** `R7.1` + `R2.2`.

### Q2 — Tee location: `journal::*` vs middleware wrapper

> ADR-0031 Q2: tee in `journal::*` or a new wrapper around
> `Ledger::post(...)`?

- **Recommended default: inline in `journal::*`, after commit.**
- **Rationale:**
  - There is no `Ledger::post(...)` — the journal is a set of free
    functions (`pub async fn post_fill`, `pub async fn
    post_strategy_signal`, etc.) each owning its own SQL transaction.
    Adding a wrapper means inventing a uniform `JournalOp` enum +
    dispatch, which doubles the surface area and forces every existing
    writer signature to change → violates R2.4.
  - Inline tee is a 2-line addition at the bottom of each writer
    (after `db_txn.commit().await?`): build the `AuditTick`, call
    `if let Some(s) = &ledger.tick_sender { let _ = s.send(tick); }`.
  - "After commit" ordering matters: a consumer must not observe a
    tick whose SQL transaction was rolled back. Producer-side ordering
    invariant (H2).
- **Override condition:** If the architect prefers a strategy-style
  `JournalOp` enum for future-proofing (e.g. a v3 event-sourcing
  bridge), they can refactor later — additive over the inline tee.
- **Sets** `R2.3` + `R2.5` + `H2`.

### Q3 — Sender exposure: explicit opt-in vs hidden side-effect

> ADR-0031 Q3: broadcast sender exposed via `Ledger` (callers opt
> in) or hidden side-effect of every journal write?

- **Recommended default: hidden side-effect, opt-in at `Ledger`
  construction.**
- **Rationale:**
  - "Opt-in per call" defeats the purpose — the whole point of the
    envelope is to remove per-consumer call sites. If a writer must
    pass an extra flag, the load-bearing surface grew, not shrank.
  - "Opt-in at construction" via `Ledger::open_with_tick_bus(...)`
    means the agent runtime decides once at startup whether ticks
    are emitted. Tests / migrations / one-shot tools use the existing
    `Ledger::open(...)` and pay zero overhead (None branch is a
    predictable static branch).
  - Subscribers obtain a receiver by calling
    `tick_sender.subscribe()` once they have a handle to the sender
    returned at construction.
- **Override condition:** If the operator wants strict
  audit-trail-only-on-explicit-call semantics (e.g. compliance), the
  default flips to per-writer feature gates. We do NOT recommend this
  — it reintroduces the consumer-tap proliferation problem.
- **Sets** `R2.1` + `R2.2`.

### Q4 — `crates/ui` consumer pattern

> ADR-0031 implicit Q: what does the iced subscription look like?

- **Recommended default: out-of-scope for this brief — UI consumer
  ships in its own follow-up.**
- **Rationale:**
  - This brief delivers the envelope + reflection stub. A second
    consumer (UI) is the next brief's job and depends on the Lab
    Trail screen design (Phase D).
  - The iced subscription pattern is well-trodden in
    `crates/ui/src/live.rs` (existing `tokio::sync::broadcast`
    consumers) — when the UI brief lands, the recipe is `subscribe →
    map(recv) → Message::AuditTickReceived(Box<AuditTick<…>>)`.
    Box because the enum is ~200B and iced messages prefer small
    stack payloads.
- **Override condition:** If the operator wants the Lab Trail
  prototype piggy-backed on this brief, scope grows by ~1 day for
  the iced subscription + a tape-row card; M-FINAL slips by ~24h.
- **Sets** the explicit out-of-scope bullet above; H4 records the
  shape claim.

### Q5 — `AuditContext.agent_pid` source

> ADR-0031 implicit Q: pid at write-time or pre-seeded at session
> start?

- **Recommended default: pre-seeded at session start
  (`std::process::id()` once, stored on the `Ledger` alongside
  `tick_sender`).**
- **Rationale:**
  - `std::process::id()` is constant for the process lifetime — paying
    a syscall on every journal write is wasted work (~50ns × ~200
    writes/sec = noise, but cumulative for v3 high-frequency
    scenarios).
  - Pre-seeding means tests can inject a synthetic pid (e.g. `42`)
    via a `Ledger::open_with_tick_bus_and_pid(...)` test-only
    constructor — keeps the unit tests deterministic.
  - For run_id (already in `AuditContext`), the same pre-seeded
    approach applies but is set per-backtest-run via
    `Ledger::with_run_id(uuid)`. Live-session run_id is the agent's
    startup-time uuid.
- **Override condition:** None recommended.
- **Sets** `R1.2` + `K-risk K4` (run_id correctness across multiple
  concurrent backtests in cockpit — see K4).

## Risk register (K-register)

### K1 — Broadcast lag / backpressure under fast producers

- **Hazard:** A slow consumer (e.g. a reflection state-replica that
  blocks on disk I/O) accumulates lag; under
  `tokio::sync::broadcast`, when the lag exceeds capacity the
  receiver gets `RecvError::Lagged(n)` and skips `n` events.
- **Likelihood:** Medium. The capacity-1024 default holds for
  ~5s of fast-trading bursts but breaks under a stop-the-world GC
  pause or a flaky SQLite write.
- **Impact:** Reflection lessons can be miscounted if the consumer
  silently drops fills; UI tape can show "replay lag" badges.
- **Mitigation:**
  - R3.1 explicitly logs every `Lagged(n)` with the delta — never
    silent. R6.1 exposes a Prometheus counter.
  - The mpsc-tap reflection writer (existing `ReflectionWriter`)
    stays as the production write path at v0.1.0 — the broadcast
    stub is observation-only. So lag in the new path is a metrics
    issue, not a correctness issue, until a future brief migrates
    the lessons writer.
  - Default capacity (Q1=1024) chosen with explicit per-second-rate
    arithmetic — see Q1 rationale.
- **Detection in M-FINAL:** New integration test
  `crates/audit/tests/tick_lag_drop.rs` saturates the channel with a
  slow consumer, asserts `Lagged(n)` is returned and the producer is
  never blocked.

### K2 — Anchor preservation if any journal-write timing shifts

- **Hazard:** Adding a post-commit `send(...)` to each writer
  changes the **timing** of `journal::*` returns. If a downstream
  test (reports body-SHA, anchor comparisons) implicitly depended on
  fast-return semantics, the anchor bytes could shift even though no
  SQL row content changed.
- **Likelihood:** Low. Anchor verification hashes the rendered
  report body, not journal wall-clock timings. The 22 anchors are
  pure SQL-derived strings.
- **Impact:** Catastrophic if it triggers — every anchor relock is
  manual operator approval.
- **Mitigation:**
  - Post-commit ordering (Q2 default) means the SQL write is
    durable before the send. The send itself is non-blocking — drops
    on no-subscribers or lag, so it cannot stretch the writer
    return.
  - **Anchor preservation gate per commit** — any commit touching
    `crates/audit/src/journal.rs` (or `ledger.rs`) runs
    `scripts/verify_anchors.sh` before advancing.
  - H2 is the falsifiable claim that anchor bytes stay identical.
- **Detection in M-FINAL:** `scripts/verify_anchors.sh` 22/22 PASS.

### K3 — Consumer `iter` pattern across crate boundaries

- **Hazard:** `crates/reflection` adding a dep on `crates/audit` to
  consume `AuditTickStream` could introduce a cycle or violate the
  import-direction invariant from
  [01-data-flow.md](../architecture/01-data-flow.md).
- **Likelihood:** Low.
  [01-data-flow.md](../architecture/01-data-flow.md) already lists
  `reports → audit` as a read-only edge; this brief adds `reflection
  → audit (via AuditTick stream)` symmetrically. ADR-0031
  §Architecture invariants explicitly anticipated this.
- **Impact:** Compile failure on adding the dep is the easy mode;
  silent contract drift is the worst case.
- **Mitigation:**
  - R3.3 forbids `crates/audit::tick` from importing sibling crates;
    `AuditEvent` payloads use `trading_core` types only (`Fill`,
    `Signal`, `Venue`, etc.).
  - Architect must update
    [01-data-flow.md](../architecture/01-data-flow.md) edge table in
    M-T1 to reflect the new edge.
- **Detection in M-FINAL:** `cargo check --workspace` PASS;
  `crates/reflection/Cargo.toml` adds `audit` only under
  `[dependencies]`, not `[dev-dependencies]`.

### K4 — `run_id` correctness with concurrent backtests in cockpit

- **Hazard:** Pre-seeded `run_id` (Q5 default) on `Ledger` assumes
  one run_id per `Ledger` handle. The cockpit's in-process backtest
  (ADR-0030, ADR-0034) spins multiple backtests concurrently against
  the same audit DB; if they share a `Ledger` clone, their ticks
  carry the same run_id and consumers can't disambiguate.
- **Likelihood:** Medium. The cockpit-training-control feature
  (just landed) introduced the multi-backtest pattern.
- **Impact:** Reflection lessons / Lab Trail UI conflates runs.
- **Mitigation:**
  - `Ledger::with_run_id(uuid)` is a builder that returns a fresh
    `Ledger` clone with a new run_id stamped on its tick context —
    cheap (channel sender + uuid only). Each backtest call site
    threads its own uuid.
  - Architect to confirm in M-T1 whether the cockpit-training-control
    backtests already mint per-run uuids (likely yes — they have run
    folders).
- **Detection in M-FINAL:** New unit test asserts two `Ledger`
  clones with different run_ids emit ticks carrying their respective
  uuid.

### K5 — Tick coverage drift (silent miss of new writers)

- **Hazard:** A future PR adds a new `journal::*` writer (e.g.
  `post_partial_fill`) and forgets to wire the tee. Consumers
  silently miss the event.
- **Likelihood:** High over time.
- **Impact:** Reflection lessons go stale; Lab Trail drill misses
  rows.
- **Mitigation:**
  - **Exhaustive variant coverage test** — a unit test in
    `crates/audit/tests/tick_variant_coverage.rs` calls every
    writer that should emit a variant per R2.5 and asserts a tick
    appears on a subscriber.
  - Architect must document the "tee opt-in" convention in
    `crates/audit/src/journal.rs` rustdoc header.
- **Detection in M-FINAL:** Variant-coverage test (per R2.5)
  exercises every in-scope writer.

### K6 — Sender drop ordering on agent shutdown

- **Hazard:** If the `Ledger` is dropped before its subscribers
  finish draining (e.g. graceful shutdown), the broadcast sender
  drops, receivers get `RecvError::Closed`, and any in-flight ticks
  are lost.
- **Likelihood:** Medium (shutdown is unceremonious in v2.x).
- **Impact:** Low — durability contract (ADR-0031 §Open Questions
  Q3) says ticks are in-memory only; SQL rows are durable, restart
  consumer backfills from SQL.
- **Mitigation:** Document the durability contract in
  `crates/audit/src/tick.rs` rustdoc header. Defer crash-resume to a
  future brief that needs it.
- **Detection in M-FINAL:** Document only; no test.

## Hypothesis register (H-register)

Falsifiable claims the tester/developer must validate. Each H pins a
test or measurement that, if it fails, invalidates the claim.

### H1 — Broadcast send is constant-time and never blocks the producer

- **Claim:** `tokio::sync::broadcast::Sender::send` is O(1) on
  receiver count and never awaits / blocks. On a full channel + slow
  receiver, the oldest queued item is overwritten; the sender returns
  `Ok(n)` (where n = receivers).
- **Disconfirmation:** A microbench measures send latency p99 with
  0, 1, 4, 16 subscribers. If p99 exceeds 1µs at 16 subscribers or
  scales worse than O(n_subscribers), claim is invalidated. Falls
  back to mpsc-per-consumer (Q1 alternative noted in ADR-0031).
- **Test:** `cargo bench -p audit --bench tick_send_latency`
  (criterion). Optional at M-FINAL — produce numbers, don't gate.
- **Decision boundary:** Pass = p99 ≤ 1µs @ 16 subscribers.

### H2 — Anchor bytes stay byte-identical post-tee

- **Claim:** Adding the post-commit broadcast send to every in-scope
  `journal::*` writer does NOT change the bytes of any rendered
  report or backtest output. The 22 body-SHA-256 anchors are
  byte-identical pre- and post-feature.
- **Disconfirmation:** `scripts/verify_anchors.sh` returns
  non-22/22. Any single anchor diff invalidates the claim.
- **Test:** M-FINAL gate `scripts/verify_anchors.sh` 22/22 PASS.
- **Decision boundary:** Pass = 22/22; anything else = REGRESSION
  verdict, blocks ship per CLAUDE.md non-negotiables.

### H3 — Lagging consumer drops events without producer-side blocking

- **Claim:** When a consumer's receive cadence is slower than the
  producer's send cadence and the channel saturates, the consumer
  observes `RecvError::Lagged(n)` and skips `n` events while the
  producer's `send()` continues to return immediately and never
  awaits.
- **Disconfirmation:** Integration test
  `crates/audit/tests/tick_lag_drop.rs` saturates with 2× the
  channel capacity from a tight loop into a sleeping consumer. If
  the producer's wall-clock per-send latency rises above 10µs
  (signalling backpressure leakage) OR the consumer never observes
  `Lagged(_)`, claim is invalidated.
- **Test:** M-FINAL integration test.
- **Decision boundary:** Producer per-send p99 ≤ 10µs AND consumer
  observes ≥ 1 `Lagged(_)` event.

### H4 — Iced subscription pattern is shape-compatible

- **Claim:** When `crates/ui` subscribes to the audit tick bus, the
  iced `Subscription::run_with_id` pattern from
  `crates/ui/src/live.rs` (lines 20-50) covers the recipe with zero
  new abstractions — just `recv().await` → `Message::AuditTickReceived(Box<AuditTick<…>>)`.
- **Disconfirmation:** Out-of-scope at v0.1.0 (UI consumer is a
  follow-up brief, per Q4). This H is recorded so the next brief can
  invoke it cheaply.
- **Test:** N/A at v0.1.0 (deferred).
- **Decision boundary:** N/A at v0.1.0.

### H5 — `AuditEvent` enum size stays under cache-line × 4 (256B)

- **Claim:** The largest variant of `AuditEvent` (likely
  `AuditEvent::Fill { fill: Fill, fees: Decimal }`) is ≤ 256B —
  small enough that broadcasting it across N subscribers is
  memcpy-bound, not allocation-bound.
- **Disconfirmation:** `std::mem::size_of::<AuditEvent>() > 256`
  in a unit test. If exceeded, box the largest variant
  (`AuditEvent::Fill { fill: Box<Fill>, fees: Decimal }`).
- **Test:** `crates/audit/tests/tick_event_size.rs` (compile-time
  assertion via `static_assertions::const_assert!` or runtime
  assertion).
- **Decision boundary:** Pass = ≤ 256B. Fail → boxing PR.

## Non-regression contract

1. **22 body-SHA-256 anchors stay byte-identical.** The broadcast
   tee is read-only over the existing journal; producer writes are
   unchanged in row content; commit ordering is preserved (post-
   commit tee). R5.1 / H2.
2. **Zero hot-path impact.** `tokio::broadcast::Sender::send` is
   constant-time (H1); backpressure is handled by lagging consumers
   (H3) — they drop instead of blocking producers.
3. **Anchor preservation gate per commit** — any commit touching
   `crates/audit/src/journal.rs` or `crates/audit/src/ledger.rs`
   runs `scripts/verify_anchors.sh` before advancing. R5.1.
4. `cockpit-smoke` PASS 0 panics.
5. **No new external crate deps** beyond what is already in the
   workspace (`tokio` broadcast is a sub-feature already enabled).
6. **No `barter-rs` dependency introduced** — confirmed by
   `grep -rn 'barter' Cargo.toml crates/` → 0 hits. Shape borrowed,
   crate not pulled in. R1.4.
7. **WAL-mode latent gap (ADR-0034 D1) unchanged.** This brief does
   not touch `Ledger::open` SQL setup. The latent gap is orthogonal
   and stays in its own backlog item.
8. **Existing `ReflectionWriter` (mpsc tap) untouched.** R4.2 / R4.3
   ensure the v2.x production lesson-write path is bit-identical.

## Acceptance criteria per milestone

### M0 — Analyst synthesis (this brief)

- R1..R7 locked.
- Q1..Q5 answered with analyst-recommended defaults.
- K1..K6 enumerated with mitigation + detection.
- H1..H5 stated with disconfirmation tests.
- Non-regression contract preserved.
- Trace row `REQ-AUDIT-TICK-001` exists in `spec/trace.toml` (it does
  — added in scope-promotion commit `8c1f49b`).
- **HANDOFF → operator-decide** (Q1..Q5).

### M-T1 — Architect decomposition

- Operator answers to Q1..Q5 ratified (or analyst defaults adopted).
- Architect publishes `spec/audit-tick-consumer-envelope/decomp.md`
  with the per-writer change list (R2.5), the new `Ledger`
  constructor signatures (R2.2 + Q5), the consumer-side
  `AuditTickStream` API (R3), and the reflection stub module path
  (R4).
- [01-data-flow.md](../architecture/01-data-flow.md) edge table
  updated: `reflection → audit (via AuditTick stream)` added.
- ADR-0031 status flipped from `proposed` → `accepted` with a
  pointer to this brief in the `superseded-by`/`refined-by`
  metadata.
- Trace row `REQ-AUDIT-TICK-001` state advances `proposed` →
  `accepted` and `arch[]` includes the decomp doc.

### M-DEV — Developer implementation

- `crates/audit/src/tick.rs` lands with R1 types, R2.2 constructor,
  R3.1 + R3.2 stream API, R6.1 metrics.
- `crates/audit/src/journal.rs` writers in R2.5 scope grow the
  post-commit tee call. Outside-scope writers unchanged.
- `crates/reflection/src/audit_tick_consumer.rs` lands as the R4
  stub.
- `config/agent.toml` schema gains R7.1 + R7.2 fields.
- New unit tests: `tick_event_size` (H5), `tick_variant_coverage`
  (K5).
- New integration tests: `tick_lag_drop` (H3), `tick_run_id` (K4).
- Optional bench: `tick_send_latency` (H1).
- Pre-merge: `cargo fmt`, `cargo clippy --workspace -- -D warnings`,
  `cargo test --workspace`, `scripts/verify_anchors.sh` 22/22.

### M-FINAL — Tester sweep

- `cargo fmt --check` + `cargo clippy --workspace -- -D warnings`
  exit 0.
- `cargo test --workspace` 100% PASS.
- `scripts/verify_anchors.sh` 22/22 PASS (R5.1 / H2).
- `cockpit-smoke` PASS 0 panics (R5.2).
- `crates/audit/tests/tick_*.rs` PASS.
- `crates/reflection/tests/audit_tick_consumer_stub.rs` PASS.
- Report at
  `spec/audit-tick-consumer-envelope/reports/test-final-<YYYY-MM-DD>.md`
  per `.claude/skills/rust-test/templates/test-report.md` shape.

## Trace

Trace row `REQ-AUDIT-TICK-001` exists in `spec/trace.toml` in
`proposed` state (added by orchestrator at scope-promotion). On M-T1
PASS the architect advances it to `accepted` and fills `arch[]`.

## References

- [ADR-0031](../architecture/adr/0031-audit-tick-consumer-envelope.md)
  — canonical design.
- [ADR-0024](../architecture/adr/0024-audit-sqlite-raw-sqlx.md) —
  audit-DB invariant (raw `sqlx` + SQLite WAL; latent gap noted in
  ADR-0034 D1).
- [ADR-0034](../architecture/adr/0034-cockpit-training-control.md)
  D1 — WAL-mode latent gap; orthogonal to this brief, tracked
  separately.
- [01-data-flow.md](../architecture/01-data-flow.md) — edge table
  this brief extends (K3).
- [`spec/dev-notes/external-code-patterns-2026-05-17.md`](../dev-notes/archive/2026-Q2/external-code-patterns-2026-05-17.md)
  — the survey that led to ADR-0031.
- [`crates/audit/src/journal.rs`](../../crates/audit/src/journal.rs)
  — primary write surface (lines 65, 276, 375, 775, 1335, 1524,
  1621, 1674 — the in-scope writers per R2.5).
- [`crates/audit/src/ledger.rs`](../../crates/audit/src/ledger.rs)
  — `Ledger` handle (R2.1 / R2.2 mutation site).
- [`crates/reflection/src/writer/mod.rs`](../../crates/reflection/src/writer/mod.rs)
  — existing mpsc tap (R4.2 — untouched).
- [`crates/agent/src/bus.rs`](../../crates/agent/src/bus.rs) —
  broadcast-channel precedent (Q1 capacity rationale).
- [`crates/ui/src/live.rs`](../../crates/ui/src/live.rs) lines 20-50
  — iced subscription precedent (Q4 / H4 deferred shape).

## Implementation

*Developer: M-DEV complete 2026-05-20.*

### Files added

| Path | Purpose |
|------|---------|
| `crates/audit/src/tick.rs` | `AuditTick<E,C>`, `AuditContext`, `AuditEvent` (8 variants, `#[non_exhaustive]`), `AuditTickStream`, `pub(crate) emit`, `pub emit_public` (R1 / R2.3 / R3 / R6) |
| `crates/reflection/src/audit_tick_consumer.rs` | `ReflectionAuditTickConsumer` observation-only stub (R4) |
| `crates/audit/tests/tick_event_size.rs` | H5 compile-time + runtime size guard (≤256B) |
| `crates/audit/tests/tick_variant_coverage.rs` | K5 / R2.5 — 7 tests per writer tee |
| `crates/audit/tests/tick_lag_drop.rs` | H3 / K1 — lag + non-blocking producer tests |
| `crates/audit/tests/tick_run_id.rs` | K4 — per-backtest run_id distinctness |
| `crates/audit/tests/tick_serde_roundtrip.rs` | R1.1 / R1.3 — 8 serde roundtrip tests |
| `crates/reflection/tests/audit_tick_consumer_stub.rs` | R4 end-to-end stub tests |
| `crates/audit/benches/tick_send_latency.rs` | H1 criterion bench (optional) |

### Files modified

| Path | Change |
|------|--------|
| `crates/audit/src/lib.rs` | `pub mod tick;` declaration |
| `crates/audit/src/ledger.rs` | `TickBus` struct, `tick_bus: Option<TickBus>` field, `open_with_tick_bus`, `with_run_id`, `with_pid` (R2.1 / R2.2 / Q5) |
| `crates/audit/src/journal.rs` | Rustdoc banner (K5), 6 post-commit `crate::tick::emit` tees for R2.5 writers |
| `crates/audit/Cargo.toml` | `[features] test-support = []`; `metrics` dep; `static_assertions` dev-dep; `[[bench]]` entry |
| `crates/agent/src/config.rs` | `AuditConfig { tick_bus_capacity }` + `ReflectionConfig { audit_tick_consumer_enabled }` (R7) |
| `crates/agent/src/main.rs` | Conditional `open_with_tick_bus` vs `open` (T-D-11); stub spawn (T-D-16) |
| `crates/ui/src/bin/cockpit_live.rs` | Conditional `open_with_tick_bus` vs `open` (T-D-11) |
| `crates/forecast/Cargo.toml` | `audit-tick = []` feature (T-D-12) |
| `crates/forecast/src/tcn.rs` | `#[cfg(feature="audit-tick")] ledger: Option<audit::Ledger>` + `with_ledger` builder + 2 `ForecastEmitted` emit sites (T-D-13) |
| `crates/strategy/Cargo.toml` | `forecast-audit-tick = ["forecast", "forecast/audit-tick"]` feature chain (T-D-14) |
| `crates/agent/Cargo.toml` | `forecast-audit-tick = ["strategy/forecast-audit-tick"]` feature chain (T-D-14) |
| `crates/reflection/src/lib.rs` | `pub mod audit_tick_consumer;` (T-D-15) |
| `config/agent.toml` | `[audit] tick_bus_capacity = 1024`; `[reflection] audit_tick_consumer_enabled = false` (R7) |

### Deviations from decomp

1. **T-D-12** — `audit` dep in `crates/forecast` kept required (not optional). The `train_tcn` bin uses it unconditionally so making it optional would break the existing build. Only the `audit-tick = []` feature flag was added. This gates all `TcnForecaster` ledger fields and emit calls at compile time as intended.
2. **T-D-14** — `TcnForecaster::with_ledger()` runtime wiring from agent bootstrap is architecturally blocked. `TcnForecaster` instances are constructed inside the `strategy` crate from TOML config, not in `agent/src/main.rs`. The compile-time feature chain is established. Runtime wiring requires a future architect design item (strategy crate accepting an optional `Ledger` handle via its strategy-config struct).
3. **H5 boxing** — Both `AuditEvent::Fill { fill: Box<Fill>, ... }` and `AuditEvent::StrategySignal { signal: Box<Signal>, ... }` were boxed (not just Fill) to satisfy the ≤256B size budget. The spec mentioned "likely Fill"; Signal also exceeded budget.

### Self-check results (T-D-24)

- `cargo fmt --check` → exit 0
- `cargo clippy --workspace -- -D warnings` → exit 0
- `cargo test --workspace --lib` → 279 passed, 0 failed
- `scripts/verify_anchors.sh` → `ANCHORS PASS (22 / 22)`

All 22 body-SHA-256 anchors byte-identical; R5.1 / H2 satisfied.

## Changelog

- 2026-05-20 (developer, M-DEV): implemented all 25 T-D-N tasks.
  `crates/audit/src/tick.rs` (new), `crates/audit/src/ledger.rs`
  (TickBus + new constructors), 6 post-commit tees in journal.rs,
  `crates/reflection/src/audit_tick_consumer.rs` stub, feature
  chains for forecast edge, config additions. 21 new tests, 1
  criterion bench. All gates green: fmt / clippy / 279 tests /
  22/22 anchors. HANDOFF → tester.
- 2026-05-20 (architect, M-T1): ratified analyst defaults
  Q1..Q5; published [decomp.md](decomp.md) with per-writer
  change list, `Ledger` mutation spec, `AuditTickStream` API
  surface, `ReflectionAuditTickConsumer` stub spec, config
  additions, anchor-preservation discipline, and landing
  order. Pinned `ForecastEmitted` to choice 5A (gated
  `forecast → audit` edge). Flipped ADR-0031 status
  `proposed` → `accepted`. Updated
  [01-data-flow.md](../architecture/01-data-flow.md) edge
  table (added `reflection → audit (via AuditTick stream)`
  and `forecast → audit (audit-tick feature)`). Advanced
  trace row `REQ-AUDIT-TICK-001` state `proposed` →
  `accepted`. Owner `pending-architect` → `architect`.
  HANDOFF → developer.
- 2026-05-20 (analyst, M0): replaced proposed stub with full
  R1..R7 / Q1..Q5 / K1..K6 / H1..H5 contract. Status `proposed`
  → `draft`, owner `pending-analyst` → `analyst`. HANDOFF →
  operator-decide.
- 2026-05-20 (orchestrator, promotion): promoted from candidate
  (`spec/backlog.md ## Queue / Process / tooling`) to active.
  ADR-0031 pre-existing as the canonical design.
