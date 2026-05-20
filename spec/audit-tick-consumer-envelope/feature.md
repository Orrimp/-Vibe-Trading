---
slug: audit-tick-consumer-envelope
status: proposed
owner: pending-analyst
updated: 2026-05-20
version: 0.1.0
predecessor: ADR-0031
---

# Audit tick consumer envelope (`audit-tick-consumer-envelope`)

> Process-tooling feature. Promotes the **canonical design** at
> [ADR-0031](../architecture/adr/0031-audit-tick-consumer-envelope.md)
> (status `proposed`) into an implementation contract.

## Why

The audit journal currently uses **per-consumer write taps** — every new
consumer (`crates/reflection`, future Lab Trail screen, v2.6 bake-off,
v3 success-reports) requires its own mutation of the audit writers in
`crates/audit`, `crates/exec`, and `crates/agent`. As consumer count
grows, this load-bearing surface stops stabilising.

ADR-0031 proposes a **thin read-direction envelope** layered on top of
the existing journal: every `journal::*` writer also enqueues an
`AuditTick<AuditEvent, AuditContext>` into a tokio broadcast channel.
Consumers subscribe by wrapping the broadcast receiver. The existing
double-entry ledger and SQLite schema are untouched.

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
   `StateReplicaManager`).
4. **Per-consumer state-replica stub** for `crates/reflection` to
   demonstrate the consumer side end-to-end.

## Out of scope

- Rewriting the existing audit journal (additive only).
- SQL pub/sub (rejected per ADR-0031 §Alternatives 2).
- Event sourcing rewrite (rejected per ADR-0031 §Alternatives 3).
- Other consumers (Lab Trail, v2.6 bake-off) — they subscribe in their
  own feature briefs.

## Non-regression contract

1. **22 body-SHA-256 anchors stay byte-identical.** The broadcast tee
   is read-only over the existing journal; producer writes are
   unchanged. R10.1.
2. **Zero hot-path impact.** `tokio::broadcast::Sender::send` is
   constant-time; backpressure is handled by lagging consumers (they
   drop instead of blocking producers).
3. **Anchor preservation gate per commit** — any commit touching
   `crates/audit/src/journal.rs` runs `scripts/verify_anchors.sh` before
   advancing. R10.1 / H2.
4. `cockpit-smoke` PASS 0 panics.
5. No new external crate deps beyond what's already in the workspace
   (`tokio` is already a dep; `broadcast` is a sub-feature).

## Open questions for analyst

- **Q1:** Should the broadcast channel be bounded (`broadcast::channel(N)`)
  or unbounded? Per ADR-0031, bounded with drop-on-lag for consumers.
  Confirm N (32? 256?).
- **Q2:** Does the producer-side tee live in `journal::*` or in a new
  middleware layer that wraps `Ledger::post(...)`?
- **Q3:** Should the broadcast sender be exposed via `Ledger` (so
  callers explicitly opt in) or be a hidden side-effect of every
  journal write?
- **Q4:** What's the consumer pattern for `crates/ui` — `iced::Subscription`
  recipe that converts broadcast `recv` into `Message::AuditTickReceived`?
- **Q5:** Where does `AuditContext.agent_pid` come from — `std::process::id()`
  at journal-write time, or pre-seeded at session start?

## Trace

Trace row `REQ-AUDIT-TICK-001` to be opened in proposed state by analyst.

## Changelog

- 2026-05-20 (orchestrator, promotion): promoted from candidate
  (`spec/backlog.md ## Queue / Process / tooling`) to active. ADR-0031
  pre-existing as the canonical design.
