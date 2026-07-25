---
adr: 0008
title: v0.5 — strategy events get their own SQLite table, not the double-entry ledger
status: accepted
date: 2026-04-19
supersedes: none
superseded-by: none
---

# ADR-0008: v0.5 — strategy events get their own SQLite table, not the double-entry ledger

## Context

v0.5 introduces strategy lifecycle events: load, swap, unload, reject.
v0 had a `registry_event` writer that emitted zero-amount memo rows
into `journal_entries` (the double-entry ledger) against
`equity:opening_balance`. With v0.5's higher load/swap cadence (a
research session might swap a `ComposedStrategy` several times per
hour), this approach creates two problems: the reconciler that proves
`Σ debits == Σ credits` per transaction has to filter out memo rows;
and every future non-monetary event (kill-switch trips, mode changes,
cost-budget alerts) will face the same "do we hide it in the ledger
or add a sibling" decision. Settle it once.

## Decision

A dedicated `strategy_events` SQLite table holds all strategy
lifecycle events. A new `audit::journal::strategy_event(..)` writer
emits to it inside the same `sqlx` transaction machinery as fills.
`journal_entries` is reserved for balance-carrying double-entry rows
only.

Schema (`migrations/0003_strategy_events.sql`, approximate):

```sql
CREATE TABLE strategy_events (
    id            TEXT PRIMARY KEY,       -- uuid v4
    ts            TEXT NOT NULL,          -- RFC3339
    kind          TEXT NOT NULL,          -- 'Load' | 'Swap' | 'Unload' | 'Reject'
    strategy_id   TEXT,                   -- nullable for Reject when id unparsable
    old_hash      TEXT,                   -- sha256 hex, 64 chars
    new_hash      TEXT,                   -- sha256 hex, 64 chars
    source_path   TEXT,                   -- repo-relative
    operator      TEXT NOT NULL DEFAULT 'system',
    error_code    TEXT,                   -- Reject only
    error_summary TEXT                    -- Reject only, short human message
);
CREATE INDEX strategy_events_ts_idx ON strategy_events(ts);
CREATE INDEX strategy_events_sid_idx ON strategy_events(strategy_id, ts);
```

Writer signature:

```rust
pub enum StrategyEventKind { Load, Swap, Unload, Reject }

pub struct StrategyEventWrite<'a> {
    pub kind:          StrategyEventKind,
    pub strategy_id:   Option<&'a str>,
    pub old_hash:      Option<&'a str>,
    pub new_hash:      Option<&'a str>,
    pub source_path:   &'a str,
    pub operator:      &'a str,          // "system" in v0.5
    pub error_code:    Option<&'a str>,
    pub error_summary: Option<&'a str>,
}

pub async fn strategy_event(
    ledger: &Ledger,
    write: StrategyEventWrite<'_>,
) -> Result<(), LedgerError>;
```

Reader (in `audit::query`):

```rust
pub async fn strategy_events_since(
    ledger: &Ledger,
    ts: Timestamp,
) -> Result<Vec<StrategyEventView>, LedgerError>;

pub async fn strategy_history(
    ledger: &Ledger,
    id: StrategyId,
) -> Result<Vec<StrategyEventView>, LedgerError>;
```

`StrategyEventView` lives in `trading_core` alongside `FillView` /
`JournalEntryView`. The v0 zero-amount memo rows remain in
`journal_entries` as history — no migration; the new writer path
replaces them prospectively.

## Alternatives considered

- **Reuse `journal_entries` with an `entry_kind` discriminator column
  plus a `CHECK` constraint.** Conflates balance-carrying and metadata
  rows, poisons the reconciler query, turns a clean double-entry story
  into filter discipline a future developer will forget. Rejected.
- **Single-table polymorphism via sparse nullable metadata columns on
  `journal_entries`.** Same conceptual problems plus index bloat.
  Rejected.

## Consequences

- The minute-boundary reconciler walks `journal_entries` only.
  `strategy_events` rows do not affect `Σ debits == Σ credits`. Test
  T214 asserts the v0.5 R7 + R8 integration cycles leave the
  reconciler at zero imbalance.
- The pattern (sibling table, not ledger reuse) is now the precedent
  for future non-monetary event types — kill-switch trips, mode
  changes, cost-budget alerts. Each gets its own table when it lands;
  the rule is "balance-carrying → ledger; everything else → its own
  table".
- ADR-0012's broadcast types (`StrategyLoaded`, `StrategySwapped`,
  `StrategyLoadError`) are the runtime mirror of this table — the
  writer translates events from the bus into rows here.

## Changelog
- 2026-04-19 (architect): initial accept. Extracted from
  `spec/architecture.md` § v0.5 — strategy-event journal schema (Q1)
  during Phase 1A Session 5 (2026-05-13).
