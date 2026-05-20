---
slug: audit-tick-consumer-envelope
status: draft
owner: architect
updated: 2026-05-20
version: 0.1.0
predecessor: ADR-0031
---

# Decomposition — audit-tick-consumer-envelope

> M-T1 architect deliverable. Implements the contract in
> [feature.md](feature.md) (R1..R7 / Q1..Q5 / K1..K6 / H1..H5) against
> the canonical design in
> [ADR-0031](../architecture/adr/0031-audit-tick-consumer-envelope.md)
> (status `accepted` as of this commit). All five operator questions
> resolved to analyst defaults on 2026-05-20 ("Autoapprove all").
>
> The developer implements against this document. Every change site
> below is pinned with `file:line`. No code may land outside the
> listed sites.

## 1. New public surface (single locus)

The **only** new public surface added by this feature lives in two
crates. Everything else (config, journal tee, reflection stub) is
private wiring around these types.

### 1.1 `crates/audit/src/tick.rs` (NEW)

```rust
// crates/audit/src/tick.rs — NEW MODULE, mod-declared in lib.rs.
// File-level rustdoc states (a) durability contract (K6): ticks are
// in-memory only; SQL rows are durable; restart consumers backfill
// from SQL; (b) the tee opt-in convention (K5 mitigation): every new
// journal::* writer added in scope MUST grow a post-commit
// tick_emit!(...) call.

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use smol_str::SmolStr;
use time::OffsetDateTime;
use tokio::sync::broadcast;
use trading_core::{
    fill::Fill,
    forecast::ForecastOverlay,
    signal::{Signal, StrategyId},
    venue::Venue,
    Symbol,
};
use uuid::Uuid;

/// Generic envelope over an audit event plus run-time context.
/// Mirrors the barter-rs `AuditTick` shape (no crate dep).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditTick<Event, Context = AuditContext> {
    pub event: Event,
    pub context: Context,
}

/// Run-time context attached to every tick. Pre-seeded on the
/// `Ledger` at session start (Q5). `agent_pid` is set once via
/// `std::process::id()`; `run_id` defaults to `Uuid::nil()` for the
/// live agent's startup-time uuid OR is overridden per-backtest via
/// `Ledger::with_run_id(uuid)` (K4 mitigation). `posted_at` is
/// stamped at each `send()` call site (one syscall per emit, NOT
/// per write — the syscall happens on the tee path that we already
/// hot-pathed).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditContext {
    pub run_id: Uuid,
    pub posted_at: OffsetDateTime,
    pub agent_pid: u32,
}

/// Variants emitted at v0.1.0. `#[non_exhaustive]` is MANDATORY so v3
/// can add `PartialFill`, `OrderPlaced`, etc., without breaking
/// downstream consumers (R1.3 / K5).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub enum AuditEvent {
    /// Emitted from `journal::post_fill` (R2.5).
    Fill { fill: Fill, fees: Decimal },
    /// Emitted from `journal::post_strategy_signal` (R2.5).
    StrategySignal {
        strategy_id: StrategyId,
        signal: Signal,
    },
    /// Emitted from `journal::strategy_event` (R2.5). Covers the four
    /// delegating writers (`feed_reconnect`, `rebalance_rejected`,
    /// `mean_reversion_stop`, `pair_short_observation`) through the
    /// `kind` discriminator. Typed `FeedReconnect` variant below is
    /// reserved for a future refactor that lifts those delegations.
    StrategyEvent {
        kind: SmolStr,
        payload_json: String,
    },
    /// Emitted from `crates/forecast/src/tcn.rs` cache-hit + post-
    /// inference call sites (see §5). At v0.1.0 the call site is
    /// guarded by a `Ledger` handle held on `TcnForecaster` (developer
    /// adds field; see §5).
    ForecastEmitted {
        overlay: ForecastOverlay,
        cache_hit: bool,
    },
    /// Emitted from `journal::kill_switch_tripped` (R2.5). Does NOT
    /// delegate to `strategy_event` — owns its own transaction.
    KillSwitchTripped { reason: SmolStr },
    /// Reserved for v3 typed-feed refactor. Not emitted at v0.1.0;
    /// `feed_reconnect` produces `StrategyEvent { kind = "FeedReconnect" }`
    /// via the `strategy_event` delegation today.
    FeedReconnect {
        venue: Venue,
        symbol: Symbol,
        gap_ms: u64,
    },
    /// Emitted from `journal::open_uptime_interval` (R2.5).
    UptimeIntervalOpened { run_id: Uuid },
    /// Emitted from `journal::close_uptime_interval` (R2.5).
    UptimeIntervalClosed { run_id: Uuid, duration_s: u64 },
}

/// Newtype wrapping a `broadcast::Receiver`. Provides an async
/// `next()` with explicit lag handling (R3.1) and a blocking-
/// iterator adaptor for synchronous consumers (R3.2).
pub struct AuditTickStream {
    rx: broadcast::Receiver<AuditTick<AuditEvent>>,
    consumer_label: SmolStr,
}

impl AuditTickStream {
    /// Wrap a fresh receiver. `consumer_label` flows into the
    /// `audit_tick_lagged_total{consumer = label}` Prometheus
    /// counter on `RecvError::Lagged` (R6.1).
    pub fn new(
        rx: broadcast::Receiver<AuditTick<AuditEvent>>,
        consumer_label: impl Into<SmolStr>,
    ) -> Self {
        Self {
            rx,
            consumer_label: consumer_label.into(),
        }
    }

    /// Returns the next tick.
    /// - `Ok(tick)` → `Some(tick)`.
    /// - `Err(Lagged(n))` → log + counter, recurse to next `recv()`.
    /// - `Err(Closed)`   → `None` (sender dropped; consumer should
    ///   wind down).
    pub async fn next(&mut self) -> Option<AuditTick<AuditEvent>> {
        loop {
            match self.rx.recv().await {
                Ok(tick) => return Some(tick),
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!(
                        target = "audit::tick",
                        consumer = %self.consumer_label,
                        lagged = n,
                        "audit tick stream lagged"
                    );
                    metrics::counter!(
                        "audit_tick_lagged_total",
                        "consumer" => self.consumer_label.to_string(),
                    )
                    .increment(n);
                    continue;
                }
                Err(broadcast::error::RecvError::Closed) => return None,
            }
        }
    }

    /// Blocking adaptor for synchronous consumers (R3.2). Uses
    /// `tokio::runtime::Handle::block_on` internally; panics if
    /// called from inside an async runtime. Intended for
    /// `crates/reports` synchronous renderers.
    pub fn into_iter_blocking(
        mut self,
    ) -> impl Iterator<Item = AuditTick<AuditEvent>> {
        let handle = tokio::runtime::Handle::current();
        std::iter::from_fn(move || handle.block_on(self.next()))
    }
}
```

**No other public items.** `AuditTick`, `AuditContext`, `AuditEvent`,
`AuditTickStream`, plus the constructor on `Ledger` (§2) are the
complete v0.1.0 public surface.

### 1.2 `crates/reflection/src/audit_tick_consumer.rs` (NEW)

```rust
// crates/reflection/src/audit_tick_consumer.rs — NEW MODULE,
// mod-declared in `crates/reflection/src/lib.rs` next to `writer`.
// Observation-only stub at v0.1.0 (R4.1). Replaces nothing; the
// existing `ReflectionWriter` mpsc tap is bit-identical (R4.2).

use audit::tick::{AuditEvent, AuditTickStream};
use std::sync::Arc;

/// v0.1.0 stub consumer — logs and counts variants. Does NOT write
/// `LessonCard`s (R4.1). The lesson-write migration is a follow-up
/// brief.
pub struct ReflectionAuditTickConsumer<S> {
    stream: AuditTickStream,
    #[allow(dead_code)] // kept for the follow-up brief that writes lessons
    store: Arc<S>,
}

impl<S: Send + Sync + 'static> ReflectionAuditTickConsumer<S> {
    pub fn new(stream: AuditTickStream, store: Arc<S>) -> Self {
        Self { stream, store }
    }

    /// Drain ticks until the sender drops. Variants → tracing::info
    /// + per-variant counter `reflection_audit_tick_seen_total{variant=…}`.
    pub async fn run(mut self) {
        while let Some(tick) = self.stream.next().await {
            let variant = match &tick.event {
                AuditEvent::Fill { .. } => "Fill",
                AuditEvent::StrategySignal { .. } => "StrategySignal",
                AuditEvent::StrategyEvent { .. } => "StrategyEvent",
                AuditEvent::ForecastEmitted { .. } => "ForecastEmitted",
                AuditEvent::KillSwitchTripped { .. } => "KillSwitchTripped",
                AuditEvent::FeedReconnect { .. } => "FeedReconnect",
                AuditEvent::UptimeIntervalOpened { .. } => "UptimeIntervalOpened",
                AuditEvent::UptimeIntervalClosed { .. } => "UptimeIntervalClosed",
                _ => "Unknown", // non_exhaustive: future variants land here
            };
            tracing::info!(
                target = "reflection::audit_tick",
                variant,
                run_id = %tick.context.run_id,
                "audit tick observed"
            );
            metrics::counter!(
                "reflection_audit_tick_seen_total",
                "variant" => variant,
            )
            .increment(1);
        }
    }
}
```

**No new public surface from `reflection`** beyond
`ReflectionAuditTickConsumer::{new, run}`. Spawned only when
`[reflection].audit_tick_consumer_enabled = true` (R4.3 / R7.2).

## 2. `Ledger` mutation spec

Current state: `Ledger` is a thin `Clone` wrapper around an
`sqlx::SqlitePool` ([`crates/audit/src/ledger.rs:8-55`](../../crates/audit/src/ledger.rs)).
Two fields, no behavior beyond connect + migrate.

Target state (single field addition + three new constructors):

```rust
// crates/audit/src/ledger.rs (mutated; existing fields preserved)

use crate::tick::{AuditContext, AuditEvent, AuditTick};
use tokio::sync::broadcast;
use uuid::Uuid;

#[derive(Clone)]
pub struct Ledger {
    pub(crate) pool: sqlx::SqlitePool,
    /// Pre-seeded tick context — cloned + posted_at-stamped on every
    /// post-commit tee call. `None` means tee dormant (R2.1 / Q3).
    /// Cheap to clone (sender is `Arc`-backed; context is two `u32`s
    /// + a `Uuid` + an `OffsetDateTime`).
    pub(crate) tick_bus: Option<TickBus>,
}

#[derive(Clone)]
pub(crate) struct TickBus {
    pub(crate) sender: broadcast::Sender<AuditTick<AuditEvent>>,
    pub(crate) run_id: Uuid,
    pub(crate) agent_pid: u32,
}

impl Ledger {
    // EXISTING — preserved bit-identical. Default branch hits None
    // arm at every tee site (R2.1 / K2 anchor preservation).
    pub async fn open(db_path: &str) -> Result<Self, LedgerError> { /* unchanged */ }

    // EXISTING — preserved bit-identical.
    pub async fn in_memory() -> Result<Self, LedgerError> { /* unchanged */ }

    // NEW (R2.2 + Q3 + Q5): wires the channel and pre-seeds pid +
    // run_id. Returns the (Ledger, sender) pair so the caller can
    // .subscribe() once per consumer.
    pub async fn open_with_tick_bus(
        db_path: &str,
        capacity: usize,
    ) -> Result<(Self, broadcast::Sender<AuditTick<AuditEvent>>), LedgerError> {
        let mut ledger = Self::open(db_path).await?;
        let (sender, _) = broadcast::channel(capacity);
        ledger.tick_bus = Some(TickBus {
            sender: sender.clone(),
            run_id: Uuid::nil(),               // operator overrides via .with_run_id()
            agent_pid: std::process::id(),     // one syscall, session-lifetime (Q5)
        });
        Ok((ledger, sender))
    }

    // NEW (K4 mitigation): per-backtest run-id stamping. Returns a
    // FRESH Ledger clone with a new run_id on its tick context. The
    // SQLite pool and the broadcast sender are shared (cheap Arc clone);
    // the run_id field is the only mutation. Concurrent backtests get
    // distinct run_ids without contending on the same Ledger.
    #[must_use]
    pub fn with_run_id(&self, run_id: Uuid) -> Self {
        let mut next = self.clone();
        if let Some(bus) = next.tick_bus.as_mut() {
            bus.run_id = run_id;
        }
        next
    }

    // NEW (Q5 test helper, #[cfg(any(test, feature = "test-support"))]):
    // synthetic pid for deterministic assertions in
    // `tests/tick_run_id.rs` and `tests/tick_variant_coverage.rs`.
    #[cfg(any(test, feature = "test-support"))]
    #[must_use]
    pub fn with_pid(&self, pid: u32) -> Self {
        let mut next = self.clone();
        if let Some(bus) = next.tick_bus.as_mut() {
            bus.agent_pid = pid;
        }
        next
    }

    pub fn pool(&self) -> &sqlx::SqlitePool { &self.pool } // unchanged
}
```

**Crate-level helper** (private — lives in `crates/audit/src/tick.rs`):

```rust
// crates/audit/src/tick.rs — pub(crate) helper used by journal.rs
// tee sites. Single function so every tee call is one line at the
// writer's tail.

pub(crate) fn emit(ledger: &crate::Ledger, event: AuditEvent) {
    let Some(bus) = ledger.tick_bus.as_ref() else { return };
    let tick = AuditTick {
        event,
        context: AuditContext {
            run_id: bus.run_id,
            posted_at: time::OffsetDateTime::now_utc(),
            agent_pid: bus.agent_pid,
        },
    };
    // Variant label for the emitted-counter (R6.1).
    let variant = match &tick.event {
        AuditEvent::Fill { .. } => "Fill",
        AuditEvent::StrategySignal { .. } => "StrategySignal",
        AuditEvent::StrategyEvent { .. } => "StrategyEvent",
        AuditEvent::ForecastEmitted { .. } => "ForecastEmitted",
        AuditEvent::KillSwitchTripped { .. } => "KillSwitchTripped",
        AuditEvent::FeedReconnect { .. } => "FeedReconnect",
        AuditEvent::UptimeIntervalOpened { .. } => "UptimeIntervalOpened",
        AuditEvent::UptimeIntervalClosed { .. } => "UptimeIntervalClosed",
        _ => "Unknown",
    };
    metrics::counter!("audit_tick_emitted_total", "variant" => variant).increment(1);
    tracing::debug!(target = "audit::tick", variant, "audit tick emitted");
    // Silently drop on Lagged / no-receivers (R2.3 / H1 / H3).
    let _ = bus.sender.send(tick);
}
```

The tee call at each writer is exactly **one line** at the bottom,
after `db_txn.commit().await?` (or after the bare `execute` for
writers that do not use an explicit txn):

```rust
crate::tick::emit(ledger, AuditEvent::Fill { fill: fill.clone(), fees: fee });
Ok(/* existing return */)
```

The `None`-branch in `emit` is a single predictable static branch.
H1 (constant-time) holds; H2 (anchor preservation) holds because
the tee is post-commit and read-only over the journal rows.

## 3. Per-writer change list (R2.5)

Each row pins a `file:line` from
[`crates/audit/src/journal.rs`](../../crates/audit/src/journal.rs).
"Tee location" = the line **after** which the developer inserts the
single `crate::tick::emit(ledger, …);` call. Writers marked
**delegate** reach the tee through `strategy_event` and gain **no
additional code** in this feature.

| # | Writer                       | file:line | Variant emitted                                                          | Tee site                          | Delegates? |
|---|------------------------------|-----------|--------------------------------------------------------------------------|-----------------------------------|------------|
| 1 | `post_fill`                  | `crates/audit/src/journal.rs:65`   | `AuditEvent::Fill { fill: fill.clone(), fees: fill.fee.amount() }`        | after `db_txn.commit()`           | no         |
| 2 | `post_strategy_signal`       | `crates/audit/src/journal.rs:276`  | `AuditEvent::StrategySignal { strategy_id, signal: signal.clone() }`     | after the single `execute(...)`   | no         |
| 3 | `kill_switch_tripped`        | `crates/audit/src/journal.rs:775`  | `AuditEvent::KillSwitchTripped { reason: SmolStr::new(reason) }`         | after `db_txn.commit()` (line 863) | no         |
| 4 | `strategy_event`             | `crates/audit/src/journal.rs:1335` | `AuditEvent::StrategyEvent { kind: SmolStr::new(write.kind), payload_json }` (payload built from `write.error_summary` JSON) | after the single `execute(...)` (line 1378) | no         |
| 5 | `rebalance_rejected`         | `crates/audit/src/journal.rs:1435` | (none — delegates to `strategy_event`)                                   | — (covered by row 4)              | yes        |
| 6 | `mean_reversion_stop`        | `crates/audit/src/journal.rs:1473` | (none — delegates to `strategy_event`)                                   | — (covered by row 4)              | yes        |
| 7 | `feed_reconnect`             | `crates/audit/src/journal.rs:1524` | (none — delegates to `strategy_event`, kind=`"FeedReconnect"`)            | — (covered by row 4)              | yes        |
| 8 | `pair_short_observation`     | `crates/audit/src/journal.rs:1563` | (none — delegates to `strategy_event`)                                   | — (covered by row 4)              | yes        |
| 9 | `open_uptime_interval`       | `crates/audit/src/journal.rs:1621` | `AuditEvent::UptimeIntervalOpened { run_id: bus.run_id }`                | after the single `execute(...)`   | no         |
| 10 | `close_uptime_interval`      | `crates/audit/src/journal.rs:1674` | `AuditEvent::UptimeIntervalClosed { run_id, duration_s }` (`duration_s` derived from the `SELECT started_at` already-fetched value, or set to `0` if not available at the writer — see note below) | after the single `execute(...)`   | no         |

**Note on `duration_s` in row 10.** The current
`close_uptime_interval` writer at line 1674 issues only `UPDATE
agent_uptime SET stopped_at = ? WHERE boot_id = ?` — it does not
read `started_at`. To populate `duration_s` cheaply, the developer
issues one `SELECT started_at FROM agent_uptime WHERE boot_id = ?`
**before** the UPDATE (still inside the writer, no transaction
needed — single-row read), and computes
`OffsetDateTime::now_utc() - started_at`. If the SELECT returns no
row (defensive), emit `duration_s: 0`. The SELECT does NOT touch
anchor-relevant rows; H2 holds.

### Explicit out-of-scope writers (do NOT grow a tee)

These stay SQL-only at v0.1.0 (R2.5 last paragraph). The developer
**must not** add a `crate::tick::emit(...)` call to any of them.

| Writer                       | file:line | Reason not in scope |
|------------------------------|-----------|---------------------|
| `update_signal_clamp_status` | `crates/audit/src/journal.rs:375`  | Mutator of an existing signal row; no consumer needs it at v0.1.0. |
| `post_training_start`        | `crates/audit/src/journal.rs:436`  | Training-side; covered by `crates/training` future brief. |
| `post_training_epoch`        | `crates/audit/src/journal.rs:482`  | Training-side; future. |
| `post_training_finish`       | `crates/audit/src/journal.rs:545`  | Training-side; future. |
| `post_training_failed`       | `crates/audit/src/journal.rs:607`  | Training-side; future. |
| `registry_event`             | `crates/audit/src/journal.rs:700`  | Config-load event; consumer brief deferred. |
| `strategy_paused`            | `crates/audit/src/journal.rs:890`  | Watcher-side; covered by `strategy_event` already if needed (delegates upstream). |
| `risk_veto_overridden`       | `crates/audit/src/journal.rs:999`  | Operator-veto path; consumer brief deferred. |
| `post_cost`                  | `crates/audit/src/journal.rs:1101` | Cost accounting; consumer brief deferred. |
| `post_cost_llm`              | `crates/audit/src/journal.rs:1137` | Cost accounting; consumer brief deferred. |
| `post_llm_budget_event`      | `crates/audit/src/journal.rs:1259` | LLM budget tripwire; consumer brief deferred. |
| `heartbeat_uptime`           | `crates/audit/src/journal.rs:1649` | Heartbeat noise; no consumer needs per-beat ticks. |
| `insert_funding_obs`         | `crates/audit/src/journal.rs:1698` | Observation-only data row. |
| `verify_balance`             | `crates/audit/src/journal.rs:1740` | Read-side admin check; no row written. |

**K5 mitigation locus.** The developer adds a rustdoc convention
banner to `crates/audit/src/journal.rs` header — the comment block
linked from `lib.rs` — stating:

> Every writer that calls `db_txn.commit()` (or `execute(...)` on a
> single-shot row) and represents an event a consumer might care
> about MUST grow a `crate::tick::emit(ledger, AuditEvent::…)` call
> after the commit. The in-scope writers at v0.1.0 are enumerated
> in `spec/audit-tick-consumer-envelope/decomp.md §3`. Adding a new
> variant requires an ADR amendment.

Plus the variant-coverage test (§7) is the runtime guard against
drift.

## 4. `crates/data` and `crates/agent` — no source changes

The `feed_reconnect`, `kill_switch_tripped`, `strategy_event` call
sites in `crates/data/{binance,kraken,coinbase}.rs` (see
[`crates/data/src/binance.rs:303`](../../crates/data/src/binance.rs),
[`crates/data/src/kraken.rs:421`](../../crates/data/src/kraken.rs),
[`crates/data/src/coinbase.rs:424`](../../crates/data/src/coinbase.rs))
and `crates/agent/src/{kill_switch.rs,runtime.rs,watcher.rs}` (see
[`crates/agent/src/kill_switch.rs:292`](../../crates/agent/src/kill_switch.rs),
[`crates/agent/src/runtime.rs:865`](../../crates/agent/src/runtime.rs),
[`crates/agent/src/watcher.rs:287`](../../crates/agent/src/watcher.rs))
already pass the `Ledger` handle to `audit::journal::*`. The tee
fires through the `Ledger` reference — **no signature changes** at
these sites (R2.4).

The single agent-side wiring change is the choice at startup
between `Ledger::open(...)` and `Ledger::open_with_tick_bus(...)`.
The developer threads that decision through `crates/agent/src/main.rs`
and `crates/ui/src/bin/cockpit_live.rs` behind the
`[audit].tick_bus_capacity` config field (§6).

## 5. `ForecastEmitted` call-site pin

**Current state.** `crates/forecast/src/tcn.rs` emits two
`tracing::info!(target = "forecast.audit", kind = "forecast_emitted", …)`
events — one for cache-hit, one for post-inference — at lines
**786-795** and **889-898** of
[`crates/forecast/src/tcn.rs`](../../crates/forecast/src/tcn.rs).
There is **no current write to `audit::journal`** for this event;
the `forecast` crate has no `audit` dep, and the tracing event is
the de facto audit surface today.

**Target state at v0.1.0.** The `ForecastEmitted` tick is the
**single** new tap that requires a crate-graph edge addition. Two
implementation choices, in decreasing order of architect preference:

### 5A (preferred) — `TcnForecaster` gains an optional `Ledger`

`crates/forecast/Cargo.toml` adds `audit = { path = "../audit",
optional = true }` under `[dependencies]` and a feature
`audit-tick = ["dep:audit"]`. The default build is unchanged —
default features list stays empty per
[`crates/forecast/Cargo.toml`](../../crates/forecast/Cargo.toml)
line 18.

`TcnForecaster` ([`crates/forecast/src/tcn.rs:420`](../../crates/forecast/src/tcn.rs))
gains an optional field (gated by `feature = "audit-tick"`):

```rust
#[cfg(feature = "audit-tick")]
pub(crate) ledger: Option<audit::Ledger>,
```

And a builder `with_ledger(self, ledger: audit::Ledger) -> Self`.
The two existing tracing-only emit sites at
**`crates/forecast/src/tcn.rs:786-795`** and
**`crates/forecast/src/tcn.rs:889-898`** grow one line each:

```rust
#[cfg(feature = "audit-tick")]
if let Some(l) = self.ledger.as_ref() {
    audit::tick::emit_public(l, audit::tick::AuditEvent::ForecastEmitted {
        overlay: overlay.clone(),
        cache_hit: true, // or false at the post-inference site
    });
}
```

A small companion `pub fn emit_public(ledger: &Ledger, event:
AuditEvent)` is exposed from `crates/audit/src/tick.rs` for
cross-crate emission. (Internal `pub(crate) emit` stays for the
in-`audit` tee sites.)

**Risk:** introduces `crates/forecast → crates/audit` edge — a new
write edge in
[01-data-flow.md](../architecture/01-data-flow.md). Architect adds
this row to the edge table in §6.

### 5B (fallback) — emit from `crates/strategy/src/tcn_overlay_momentum.rs`

If 5A is rejected as too invasive on the forecast crate, the
overlay strategy at
[`crates/strategy/src/tcn_overlay_momentum.rs:265`](../../crates/strategy/src/tcn_overlay_momentum.rs)
already wraps `TcnForecaster::forward` in a sync wrapper and knows
when a forecast is materialised. The strategy gains the optional
`Ledger` handle, the emit site lives there, and the
`crates/strategy → crates/audit` edge is added instead.

**Trade-off:** 5B loses cache-hit visibility (the cache lookup is
inside `TcnForecaster::forecast`, not exposed to the strategy).
For v0.1.0 this is **acceptable** because `cache_hit` is a
nice-to-have, not a contract guarantee. If cache-hit signal is
required, 5A is the only option.

### Architect decision

**Choose 5A.** Cache-hit visibility is on the v2.6 bake-off
follow-up brief's wish list (the bake-off compares replay cache vs
live inference); ripping it out now means a re-architect when that
brief lands. The `audit-tick` feature gate keeps default builds
free of the new edge.

The developer:
1. Adds `audit = { path = "../audit", optional = true }` and the
   `audit-tick` feature to
   [`crates/forecast/Cargo.toml`](../../crates/forecast/Cargo.toml).
2. Adds `audit-tick` to the workspace default-features list **only
   for binaries that already build the agent runtime** (i.e. the
   `trading` bin and the `cockpit_live` bin). The training bins
   (`train_tcn`, `forecast_distribution`) do NOT enable
   `audit-tick` — they have no `Ledger`.
3. Grows the field and the two emit sites as shown above.
4. Wires `TcnForecaster::with_ledger(ledger)` from the agent
   bootstrap when `[audit].tick_bus_capacity` is set.

If the developer hits a compile failure tracing this edge
(unexpected — `forecast` does not currently import `audit`), they
escalate back to architect rather than inventing 5B silently.

## 6. Config additions (R7)

`config/agent.toml` schema additions — both are backward-compat
serde defaults (R7.3):

```toml
[audit]
# capacity of the broadcast tick bus, 0 disables (R7.1 / Q1=1024 default).
tick_bus_capacity = 1024

[reflection]
# v0.1.0 stub: observation-only consumer (R7.2 / R4.3).
audit_tick_consumer_enabled = false
```

Rust-side, the developer adds these fields to the existing
config structs in `crates/agent/src/config.rs` with
`#[serde(default)]` and helper constants:

```rust
#[derive(Debug, Clone, Deserialize, Default)]
pub struct AuditConfig {
    #[serde(default = "default_tick_bus_capacity")]
    pub tick_bus_capacity: usize,
}
fn default_tick_bus_capacity() -> usize { 1024 }

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ReflectionConfig {
    #[serde(default)]
    pub audit_tick_consumer_enabled: bool,
    // (plus existing fields if any)
}
```

`tick_bus_capacity = 0` → use `Ledger::open(...)` (tee dormant).
Any non-zero → use `Ledger::open_with_tick_bus(path, cap)`.
This keeps the default agent boot path bit-identical (R5.1 / H2).

## 7. Test surface (developer-authored, tester-validated)

Six new test files; all but the bench gate at M-FINAL.

| File                                                             | Type        | Covers      | Notes |
|------------------------------------------------------------------|-------------|-------------|-------|
| `crates/audit/tests/tick_event_size.rs`                           | unit        | H5          | `static_assertions::const_assert!(size_of::<AuditEvent>() <= 256)`. If size grows, dev boxes the offending variant. |
| `crates/audit/tests/tick_variant_coverage.rs`                     | integration | K5 / R2.5   | Exercises all 4 non-delegating writers (`post_fill`, `post_strategy_signal`, `kill_switch_tripped`, `open_uptime_interval`, `close_uptime_interval`) + the 4 delegators through their dedicated callers; asserts a tick of the expected variant lands on a subscriber. |
| `crates/audit/tests/tick_lag_drop.rs`                             | integration | H3 / K1     | Saturates with 2× capacity from a tight loop into a `tokio::time::sleep` consumer; asserts `Lagged(n>0)` observed AND producer per-send p99 ≤ 10µs. |
| `crates/audit/tests/tick_run_id.rs`                               | integration | K4          | Opens one `Ledger::open_with_tick_bus`, clones via `with_run_id(uuid_a)` and `with_run_id(uuid_b)`, writes one fill on each, asserts the two ticks carry their respective uuids. |
| `crates/audit/tests/tick_serde_roundtrip.rs`                      | unit        | R1.1 / R1.3 | For each `AuditEvent` variant, `serde_json::to_string(&tick) -> from_str` produces bit-identical round-trip. Guards `#[non_exhaustive]` against accidental field reorders. |
| `crates/reflection/tests/audit_tick_consumer_stub.rs`             | integration | R4          | End-to-end: opens `Ledger::open_with_tick_bus`, spawns the stub, writes one fill, asserts the per-variant counter reaches 1. |
| `crates/audit/benches/tick_send_latency.rs` *(optional)*          | bench       | H1          | Criterion bench; produce numbers, don't gate. |

## 8. Anchor preservation discipline (R5.1 / H2)

The tee is read-only over the journal:

- Every emit site is **post-commit** (or post-`execute` for
  single-shot writers). The SQL row is durable before the send;
  send failure cannot retroactively change the row.
- `tokio::sync::broadcast::Sender::send` is non-blocking and
  drops-on-lag; producer wall-clock per-write is unchanged
  (H3 guards the upper bound).
- The `None` arm of `tick_bus` is a single predictable static
  branch; the developer must wrap the emit call in
  `if let Some(...) = ledger.tick_bus { … }` indirectly via
  `tick::emit(...)` (which encapsulates the branch).
- Anchor verification (`scripts/verify_anchors.sh`) runs at every
  commit touching `crates/audit/src/{journal,ledger,tick}.rs` and
  is a non-negotiable M-FINAL gate. 22/22 PASS required.

If a single anchor diffs:

1. The developer first re-runs with `tick_bus_capacity = 0` to
   prove the tee is the cause.
2. If anchors stay byte-identical with capacity = 0, the cause
   is in the tee path. The developer hands back to architect for
   an emergency post-mortem (escalate to operator before mutating
   `spec/anchors.toml`).
3. If anchors diff with capacity = 0 too, the cause is elsewhere
   (unlikely — this brief touches no SQL-shaping code).

## 9. Data-flow edge additions (K3)

Two new edges added to
[01-data-flow.md](../architecture/01-data-flow.md):

1. `reflection → audit (via AuditTick stream)` — read-only, mirrors
   the existing `reports → audit` row. Already a `[dependencies]`
   line in
   [`crates/reflection/Cargo.toml`](../../crates/reflection/Cargo.toml)
   (verified — see line `audit = { path = "../audit" }`); no new
   Cargo edge needed.
2. `forecast → audit (via AuditTick emit_public)` — **NEW edge**,
   guarded behind `forecast`'s `audit-tick` feature (§5A). Default
   `cargo build` does not link this edge.

The `audit imports nothing from sibling crates` invariant
([01-data-flow.md §"The single rule"](../architecture/01-data-flow.md))
holds: `crates/audit/src/tick.rs` imports only `trading_core`,
`tokio`, `serde`, `uuid`, `time`, `smol_str`, `rust_decimal`,
`metrics`, `tracing` — all third-party / domain types (R3.3).

## 10. Ordering of developer landings

The developer should land in this order so each step is
independently anchor-verifiable:

1. **Land §1.1 + §2** (`tick.rs` module + `Ledger` mutation) with
   the `None` default — no tee fires yet. Run
   `scripts/verify_anchors.sh` → 22/22 expected (tee dormant).
2. **Land §3 rows 1-4 + 9-10** (the 6 non-delegating writers'
   post-commit tees) with `tick_bus_capacity = 0` still the
   default. Run `verify_anchors.sh` → 22/22 expected (capacity 0
   means no `Ledger::open_with_tick_bus` callsite, tee dormant).
3. **Land config changes (§6)** and switch the agent bootstrap to
   `Ledger::open_with_tick_bus(...)` when capacity > 0. Run
   `verify_anchors.sh` with default config (`tick_bus_capacity =
   1024`) → 22/22 expected (anchors are deterministic on row
   bytes, not tee timing). If any anchor drifts here, the cause
   is timing-coupled — escalate to architect.
4. **Land §5A** (`ForecastEmitted` via feature gate). Re-run
   `verify_anchors.sh` with the gate enabled → 22/22 expected.
5. **Land §1.2 + R4 reflection stub** behind
   `audit_tick_consumer_enabled = false`. No behaviour change at
   default; test enables the flag.
6. **Land the test surface (§7)** — `cargo test --workspace` 100%
   PASS gate.

Each step is one commit; the developer pushes the whole chain
together after `cargo fmt && cargo clippy --workspace -- -D
warnings && cargo test --workspace && scripts/verify_anchors.sh`
all PASS locally.

## 11. Open items handed back to analyst (none)

All five operator-decide questions resolved by "Autoapprove all"
on 2026-05-20. No items remain for analyst re-entry at M-T1. If
the developer hits an unforeseen choice during M-DEV, escalate to
architect before guessing.

## References

- [feature.md](feature.md) — contract (R1..R7 / Q1..Q5 / K1..K6 /
  H1..H5).
- [ADR-0031](../architecture/adr/0031-audit-tick-consumer-envelope.md)
  — canonical design (status `accepted` post-this-decomp).
- [`crates/audit/src/journal.rs`](../../crates/audit/src/journal.rs)
  — per-writer tee sites pinned in §3.
- [`crates/audit/src/ledger.rs`](../../crates/audit/src/ledger.rs)
  — mutation site (§2).
- [`crates/forecast/src/tcn.rs`](../../crates/forecast/src/tcn.rs)
  — `ForecastEmitted` call sites pinned in §5.
- [`crates/reflection/Cargo.toml`](../../crates/reflection/Cargo.toml)
  — already lists `audit` as a dep; no new edge needed for §1.2.
- [01-data-flow.md](../architecture/01-data-flow.md) — edge-table
  updates per §9.

## Changelog

- 2026-05-20 (architect, M-T1): Initial decomposition; ratified
  analyst defaults Q1..Q5; pinned all writer file:line sites;
  resolved `ForecastEmitted` to choice 5A (gated edge from
  `forecast → audit`); enumerated landing order; HANDOFF →
  developer.
