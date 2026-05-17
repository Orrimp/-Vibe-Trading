---
adr: 0031
title: `AuditTick<Event, Context>` consumer envelope for audit ledger read path
status: proposed
date: 2026-05-17
supersedes: none
superseded-by: none
---

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
