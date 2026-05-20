---
adr: 0031
title: `AuditTick<Event, Context>` consumer envelope for audit ledger read path
status: accepted
date: 2026-05-17
accepted-on: 2026-05-20
supersedes: none
superseded-by: none
refined-by: spec/audit-tick-consumer-envelope/feature.md
decomposed-by: spec/audit-tick-consumer-envelope/decomp.md
---

> **Status `accepted` on 2026-05-20.** All five operator-decide
> questions resolved to analyst defaults via "Autoapprove all"
> directive. Implementation contract is at
> [`spec/audit-tick-consumer-envelope/feature.md`](../../audit-tick-consumer-envelope/feature.md);
> per-writer change list + ForecastEmitted call-site pin are at
> [`spec/audit-tick-consumer-envelope/decomp.md`](../../audit-tick-consumer-envelope/decomp.md).
> ADR remains source-of-truth for the direction; the brief +
> decomp are source-of-truth for the v0.1.0 contract.

# ADR-0031: `AuditTick<Event, Context>` consumer envelope for audit ledger read path

## Context

Today every consumer of the audit ledger writes to it via dedicated
tap-style hooks:

- `crates/exec/src/paper.rs:40` holds `ReflectionWriterTap = Arc<ReflectionWriter>` and calls `on_trade_close(...)` to feed the reflection memory.
- `crates/data/src/{binance,kraken,coinbase}.rs` directly call `audit::journal::feed_reconnect(...)` to journal feed-disconnect events.
- `crates/reports/` reads `audit::query::*` synchronously for backtest report rendering.

The pattern is **producer-pushes-to-named-consumer**. Each new consumer
needs a new tap or read path:

- Phase D Lab `Trail` screen ([spec/ui-rethink-phase-a-lab/feature.md](../../ui-rethink-phase-a-lab/feature.md) — UI rethink Section 2 J4) needs to drill from bar → features → signal → fill → P&L.
- v2.6 bake-off ([spec/v26-forecast-bakeoff/feature.md](../../v26-forecast-bakeoff/feature.md)) needs to read trade outcomes per forecast.
- v3 continuous-paper + success-reports (terminal milestone) needs to read everything.

Adding a tap per consumer would multiply the number of write call sites
in `crates/exec` and the auditor's hot path. The
[`barter-rs` AuditStream pattern](https://github.com/barter-rs/barter-rs/blob/main/barter/src/engine/audit/mod.rs)
(read 2026-05-17, documented in
[`spec/dev-notes/external-code-patterns-2026-05-17.md`](../../dev-notes/external-code-patterns-2026-05-17.md))
solves this with a generic `AuditTick<Event, Context>` envelope and
consumer-side `Iterator<Item = AuditTick<…>>` abstraction.

## Decision

Add a thin **read-direction envelope** over the existing `crates/audit`
schema:

```rust
// crates/audit/src/tick.rs (NEW)

/// Generic envelope over an audit event + run-time context.
/// Mirrors the barter-rs shape (https://github.com/barter-rs/barter-rs)
/// adapted to this project's double-entry ledger semantics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditTick<Event, Context = AuditContext> {
    pub event: Event,
    pub context: Context,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditContext {
    pub run_id: Uuid,           // backtest run id or live-session id
    pub posted_at: OffsetDateTime,
    pub agent_pid: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuditEvent {
    Fill { fill: Fill, fees: Decimal },
    StrategySignal { strategy_id: StrategyId, signal: Signal },
    StrategyEvent { kind: SmolStr, payload_json: String },
    ForecastEmitted { overlay: ForecastOverlay, cache_hit: bool },
    KillSwitchTripped { reason: SmolStr },
    FeedReconnect { venue: Venue, symbol: Symbol, gap_ms: u64 },
    UptimeIntervalOpened { run_id: Uuid },
    UptimeIntervalClosed { run_id: Uuid, duration_s: u64 },
}
```

Producer side (the existing audit writers) gains a single
**broadcast/mpsc tee**: every `journal::*` writer also enqueues an
`AuditTick` into a tokio broadcast channel.

Consumer side: `crates/reflection`, `crates/reports`, `crates/ui`
subscribe by wrapping the broadcast receiver as an
`Iterator<Item = AuditTick<AuditEvent>>` and consuming at their own
cadence. The state-replica logic in each consumer is decoupled from
the channel primitive (per `StateReplicaManager` in barter-rs).

## Alternatives considered

1. **Status quo (tap per consumer).** Cheap today, expensive at v3 when
   the consumer count grows. Each new tap is a new mutation of `exec` /
   `agent` / `data` — load-bearing crates that should stabilise.
2. **Direct SQL subscription on `audit::journal_entries`.** SQLite has
   no native pub/sub; polling is wasteful and racy on the hot path.
3. **Event sourcing rewrite of the entire audit module.** Throws out
   the working double-entry ledger. Not justified for a read-path
   refactor.

The chosen approach is **additive**: existing tap-style writers stay;
a new broadcast tee is layered on top. Zero hot-path impact (broadcast
send is constant-time).

## Consequences

### Producer side

- `crates/audit/src/journal.rs` gains an `Option<broadcast::Sender<AuditTick<AuditEvent>>>` field. When `Some`, every writer (`post_fill`, `feed_reconnect`, `strategy_event`, etc.) also calls `.send(tick)`. Send failure (no consumers) is silently dropped.
- ~50 LOC addition. No existing writer signatures change.

### Consumer side

- `crates/reflection` is the first migration target. The current
  `ReflectionWriterTap` write-direction tap can stay for v2.x compat,
  but new reflection logic reads from `AuditTick` stream.
- Lab `Trail` screen (Phase D) subscribes to the same stream.
- v2.6 bake-off's performance-attribution loop reads from the stream.

### Anchor preservation

- The `AuditTick` enum and broadcast tee are **strictly additive**. No existing journal-write call site changes its row shape; no migration to the SQL schema. The 11 locked anchors stay byte-identical.
- The new test surface is: `cargo test -p audit -- tick::tests` (subscribe → write fill → assert tick received with correct event variant).

### Reflection-memory write path

- Today: `exec → ReflectionWriterTap → reflection::store::write_lesson_card`.
- Future: `audit → broadcast → reflection::consumer → store::write_lesson_card`.
- Migration is gradual; both can coexist behind a `cfg(feature = "audit-tick-consumer")` until the broadcast path is proven.

### Architecture invariants

- `audit` still imports nothing from sibling crates (the invariant from [section 01](../01-data-flow.md) stays intact). The broadcast channel is internal to `crates/audit`.
- The architecture edge table at section 01 gains a *read-only* edge: `reflection → audit (via AuditTick stream)`. This is symmetric with the existing `reports → audit` read-only edge.

## Open questions

- **Q1** — Channel choice: `tokio::sync::broadcast` (lossy if consumer lags) vs `tokio::sync::mpsc` per consumer (lossless, more setup overhead). Recommendation: broadcast for v2.x simplicity; revisit at v3 if consumers prove lossy-sensitive.
- **Q2** — Backfill on subscribe: a consumer joining mid-run only sees ticks from subscribe time onward. Should there be a "replay from run start" mode reading from the SQL journal? Recommendation: yes for `reports` (analytics need full history), no for `reflection` (lessons are forward-looking).
- **Q3** — Tick durability: ticks are in-memory only. If the process crashes, ticks are lost — but the underlying `journal_entries` SQL rows are durable, so a restart consumer can backfill from SQL. Document this as the durability contract.

## References

- [`spec/dev-notes/external-code-patterns-2026-05-17.md`](../../dev-notes/external-code-patterns-2026-05-17.md) — the survey that led to this ADR.
- [`barter-rs` engine/audit module](https://github.com/barter-rs/barter-rs/blob/main/barter/src/engine/audit/mod.rs) — pattern source.
- [`spec/architecture/01-data-flow.md`](../01-data-flow.md) — edge table this ADR extends.
- [`crates/audit/src/journal.rs`](../../../crates/audit/src/journal.rs) — primary write surface this ADR augments.

## Phase D amendment (2026-05-20)

> Closes deferred `T-D-14` from the predecessor
> `audit-tick-consumer-envelope v0.1.0`. K5 spike outcome
> documented in
> [`spec/ui-rethink-phase-d-trail/decomp.md §1`](../../ui-rethink-phase-d-trail/decomp.md);
> per-wave T-D-N rows in
> [`spec/ui-rethink-phase-d-trail/tasks.md`](../../ui-rethink-phase-d-trail/tasks.md).

### Context

The v0.1.0 predecessor landed `TcnForecaster::with_ledger` at
[`crates/forecast/src/tcn.rs:571-576`](../../../crates/forecast/src/tcn.rs)
and the two emit sites at `tcn.rs:822-831` (cache-hit) and `:937-947`
(post-inference). However, the v0.1.0 runtime construction path
(`agent::runtime::build_registry` at
[`crates/agent/src/runtime.rs:129-141`](../../../crates/agent/src/runtime.rs))
registers `SmaCrossover` only — `TcnOverlayMomentumStrategy` is never
constructed at live-agent runtime. Consequently
`AuditEvent::ForecastEmitted` ticks are sourced from zero call sites
in production, leaving the broadcast bus's most informative event
empty.

Phase D's trail-mirror needs `ForecastEmitted` ticks (R6.4) to close
the four-stage `Fill → Signal → Forecast → LLM-debate`
correlation chain. The wiring shape below closes T-D-14 without
breaking the H2 anchor invariant or the backtest determinism gate.

### Decision (Phase D)

Add **two** new functions; modify **zero** existing function
signatures:

1. **`TcnSyncForecaster::with_ledger(self, ledger: audit::Ledger) -> Self`**
   in `crates/strategy/src/tcn_overlay_momentum.rs` (sibling of
   `load_bs1` / `load_bs2`). Forwards to the existing
   `forecast::tcn::TcnForecaster::with_ledger` at `tcn.rs:573`.
   Feature-gated on `audit-tick` (the `strategy` crate already
   enables this feature for the `forecast` dep per
   `crates/strategy/Cargo.toml:13`).
2. **`agent::runtime::build_registry_with_ledger(cfg, ledger) -> Arc<StrategyRegistry>`**
   in `crates/agent/src/runtime.rs` (sibling of `build_registry`).
   Identical to `build_registry` plus a guarded
   `TcnOverlayMomentumStrategy::with_tcn_bs1_ledger(base, ledger)`
   registration when
   `cfg.strategies.tcn_overlay_momentum.enabled == true`. The new
   config knob defaults to `false` (mirrors the `[signal_log]`
   precedent at `crates/agent/src/config.rs:284`).

The paper-mode binary at
[`crates/agent/src/main.rs:184-186`](../../../crates/agent/src/main.rs)
switches to the new sibling; backtests and tests keep calling the
zero-ledger `build_registry(cfg)` and stay determinism-clean.

### Determinism / anchor invariants (preserved)

- **H2 anchor argument.** Backtests open the ledger via
  `audit::Ledger::open(path)` at `crates/audit/src/ledger.rs:33,59`
  which sets `tick_bus = None`. The static-branch tee at
  `crates/audit/src/tick.rs:104-107` returns early. Any new
  `tick::emit_public(...)` call inside the TCN runtime path is a
  no-op under backtest. The 22 body-SHA-256 anchors in
  `spec/anchors.toml` remain byte-identical by construction —
  Wave A exit gate is `scripts/verify_anchors.sh → 22/22 PASS`.
- **Paper-mode tick-bus liveness.** Paper mode uses
  `Ledger::open_with_tick_bus(path, cap)` at `main.rs:103`. The
  ledger's `tick_bus = Some(TickBus { … })` and the static-branch
  tee fires. `build_registry_with_ledger` is invoked one call frame
  later (`main.rs:184`), so the TCN strategy receives a
  tick-bus-armed ledger by construction.
- **Compile-time enforcement.** `build_registry(cfg)` is the
  zero-ledger sibling; `build_registry_with_ledger(cfg, ledger)`
  requires the `Arc<audit::Ledger>` parameter. Calling the wrong
  sibling at a backtest call site is a compile error class — the
  parameter is required.

### Anchor-safe schema co-deliverable: mig 011

Phase D ships SQL migration `011_trail_correlation_chain.sql` as the
durable side of the four-stage chain. The migration is
**strictly additive**: 4 `ALTER TABLE … ADD COLUMN` (all
NULL-default), 1 `CREATE TABLE IF NOT EXISTS forecast_events`,
4 `CREATE INDEX IF NOT EXISTS`. The shape mirrors migrations
008 / 009 / 010 — all three precedents are anchor-safe and the same
proof carries forward. See
[`decomp.md §2`](../../ui-rethink-phase-d-trail/decomp.md) for the
column-level SQL and §5 for the anchor-preservation proof sketch.

### Consequences (Phase D-specific)

- **New consumer-side state.** `crates/reflection` gains a
  `trail_mirror.rs` module (sibling of
  `audit_tick_consumer.rs:30-32`) holding an LRU<UUID,
  ReconstructedTrail> capped at N=16 (H4 falsification gate). The
  mirror is the second concrete broadcast consumer; the first
  (audit-tick-consumer stub) stays untouched.
- **Architecture edge unchanged.** ADR-0031 § "Architecture
  invariants" already permits `reflection → audit (via AuditTick
  stream)`. The trail-mirror lives behind that same edge. No new
  edge introduced; `ui → audit` direct edge is **not** added (the
  UI talks to the trail-mirror via a `tokio::sync::mpsc` request
  channel + a `tokio::sync::broadcast` snapshot channel surfaced
  as an iced Subscription — see
  [`decomp.md §3`](../../ui-rethink-phase-d-trail/decomp.md)).
- **No new external crates.** The amendment uses
  `tokio::sync::broadcast` (already imported), `audit::Ledger`
  (already in scope), and the existing `with_ledger` builder. LRU
  capacity is enforced via the workspace `lru` crate; if absent,
  Wave F falls back to a hand-rolled `IndexMap` eviction (cheap at
  N=16).

### Alternatives considered (Phase D)

1. **Eager ledger threading through `TcnSyncForecaster::load_bs1`.**
   Rejected — would force every backtest call site (Wave-A grep
   confirms `with_tcn_bs1` is the dominant ctor and is used in 4+
   test files) to pass a ledger argument they don't have. The
   sibling-builder pattern (`with_tcn_bs1` keeps current sig;
   `with_tcn_bs1_ledger` is new) is backwards-compatible.
2. **Downcast `Box<dyn SyncForecaster>` to concrete
   `TcnSyncForecaster`.** Rejected — fragile, type-unsafe, and
   contradicts the existing trait-object pattern. The builder mirror
   stays type-safe.
3. **Defer T-D-14 again** (fallback path). Documented in
   [`decomp.md §1.4`](../../ui-rethink-phase-d-trail/decomp.md). Not
   exercised — the spike succeeded.

### Test gates (Phase D)

- **K7 / M-FINAL T-F7.** Counter
  `reflection_audit_tick_seen_total{variant="ForecastEmitted"} >= 1`
  observed in a paper-mode TCN-overlay smoke. Asserts the wiring is
  live end-to-end.
- **H2 / M-FINAL T-F4.** `scripts/verify_anchors.sh → 22/22 PASS`.
  Non-negotiable.
- **R7.4 / H3 / M-FINAL T-F6.** Cockpit-performance v1.0.0 idle-CPU
  ≤13.6% (13.1% floor + 0.5% Phase D budget) with the trail-mirror
  subscriber running + universal chevron on every audit/Live row.
