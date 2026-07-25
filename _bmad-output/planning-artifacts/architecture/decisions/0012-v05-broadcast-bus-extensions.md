---
adr: 0012
title: v0.5 — Strategy broadcast types live in trading_core, not agent or strategy or audit
status: accepted
date: 2026-04-19
supersedes: none
superseded-by: none
---

# ADR-0012: v0.5 — Strategy broadcast types live in trading_core, not agent or strategy or audit

## Context

v0.5 broadcasts three new message types — `StrategyLoaded`,
`StrategySwapped`, `StrategyLoadError` — over the `agent::EventBus`.
The audit crate ([ADR-0008](0008-v05-strategy-event-journal-schema.md))
persists them; the cockpit ([ADR-0011](0011-v05-cockpit-strategies-panel.md))
subscribes to them. Where the types live determines who depends on
whom, and the audit-is-a-sink invariant from
[01-data-flow.md § Crate dependency edges](../../../../docs/archive/pre-bmad-spec/architecture/01-data-flow.md#crate-dependency-edges-runtime-non-test)
constrains the choice.

## Decision

The three types live in `trading_core` alongside `Fill`, `Bar`,
`PnlSnapshot`. The `agent::EventBus` gains three new
`broadcast::Sender`/`Receiver` pairs, same pattern as the existing
`fills` / `positions` / `bars` / `ticks` / `pnl` / `mode` channels.

Rust definitions:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyLoaded {
    pub id:          StrategyId,
    pub hash:        [u8; 32],        // sha256 of canonicalized AST
    pub source_path: SmolStr,
    pub ts:          Timestamp,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategySwapped {
    pub id:          StrategyId,
    pub old_hash:    [u8; 32],
    pub new_hash:    [u8; 32],
    pub source_path: SmolStr,
    pub ts:          Timestamp,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyLoadError {
    pub source_path:   SmolStr,
    pub strategy_id:   Option<StrategyId>,   // None if filename-stem unparsable
    pub error_code:    SmolStr,              // e.g. "unknown_indicator"
    pub error_summary: SmolStr,              // one-line human message
    pub ts:            Timestamp,
}
```

Bus extension:

| Channel             | Type                  | Capacity | Description                  |
|---------------------|-----------------------|----------|------------------------------|
| `strategy_loaded`   | `StrategyLoaded`      | 32       | Emitted on registry `Load`.  |
| `strategy_swapped`  | `StrategySwapped`     | 32       | Emitted on registry `Swap`.  |
| `strategy_error`    | `StrategyLoadError`   | 32       | Emitted on registry `Reject`.|

Backpressure follows the v0 pattern: `RecvError::Lagged(n)` triggers
log-and-continue in the UI subscriber; `RecvError::Closed` surfaces
as a `STRATEGIES_CONNECTION_CLOSED` panel-error copy. Capacity is
small (32) because publish rate is bounded by file-edit cadence.

## Alternatives considered

- **Put them in `agent`.** `audit::journal::strategy_event` needs
  these types to persist events; `agent`-resident would force
  `audit → agent`, inverting the existing `audit ← agent` edge and
  breaking the audit-is-a-sink invariant. Rejected.
- **Put them in `strategy`.** Forces `audit → strategy`. Same
  invariant break. Rejected.
- **Put them in `audit`.** Same cycle plus it ties the UI's
  broadcast-bus subscriber to the audit crate just to get a type.
  Rejected.

## Consequences

- `trading_core` is upstream of every other crate; no cycle.
- The audit-is-a-sink rule from
  [01-data-flow.md § Crate dependency edges](../../../../docs/archive/pre-bmad-spec/architecture/01-data-flow.md#crate-dependency-edges-runtime-non-test)
  holds: `audit` imports these types from `trading_core` and writes
  them to `strategy_events`; nothing audit imports imports back to
  audit.
- The same placement rule applies to future cross-crate event types:
  if `audit` will persist it and the UI will subscribe, it lives in
  `trading_core`.

## Changelog
- 2026-04-19 (architect): initial accept. Extracted from
  `docs/archive/pre-bmad-spec/architecture.md` § v0.5 — broadcast bus extensions (Q5)
  during Phase 1A Session 6 (2026-05-13).
