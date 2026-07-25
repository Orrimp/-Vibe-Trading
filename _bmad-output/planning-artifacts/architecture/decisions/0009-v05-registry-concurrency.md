---
adr: 0009
title: v0.5 — strategy registry uses parking_lot::RwLock, not async or arc-swap
status: accepted
date: 2026-04-19
supersedes: none
superseded-by: none
---

# ADR-0009: v0.5 — strategy registry uses parking_lot::RwLock, not async or arc-swap

## Context

v0.5 hot-loading swaps strategies in the registry at runtime ([ADR-0006](0006-v05-config-driven-composition.md)).
The hot path (`StrategyRegistry::on_bar` calling each strategy's
`on_bar`) reads the registry on every bar; the file watcher writes
only when the operator edits a strategy file. The trait shape
([ADR-0005](0005-v0-strategy-trait-no-hotload.md)) keeps
`Strategy::on_bar` synchronous, so the concurrency primitive must
work without an `.await` point on the hot path.

## Decision

`parking_lot::RwLock<HashMap<StrategyId, Box<dyn Strategy>>>` for the
v0.5 `StrategyRegistry`. The hot path takes a read guard; the
file-watcher task takes a write guard only during swap. No new
dependency — `parking_lot` is already workspace-pulled.

## Alternatives considered

- **`arc-swap::ArcSwap<HashMap<..>>`** — lock-free hot-swap. Overkill
  for 1m bar cadence; adds a dep and cognitive overhead for readers
  familiar with lock semantics. Revisit in v1+ only if a tick-latency
  strategy pushes contention into the microsecond budget.
- **`tokio::sync::RwLock`** — introduces an unnecessary `.await` into
  the bar-close path at the current sync trait shape. Stop-gap if
  `Strategy::on_bar` later migrates to async.
- **`std::sync::RwLock`** — semantically fine; `parking_lot` wins on
  the faster uncontended path and zero new workspace deps.

## Consequences

- At 1m bar cadence the read frequency is ≤ a few per second across
  all active strategies. Writes are rare (file edit → debounce →
  parse → construct → swap; once per minute at worst during research
  iteration). Sub-microsecond acquire in the uncontended case keeps
  the hot path's latency budget untouched.
- If a future tick-latency strategy lands and pushes registry reads
  into the microsecond budget, supersede this ADR with `arc-swap` or
  the equivalent.
- If `Strategy::on_bar` migrates to async, supersede with
  `tokio::sync::RwLock`.

## Changelog
- 2026-04-19 (architect): initial accept. Extracted from
  `spec/architecture.md` § v0.5 — registry concurrency (Q2) during
  Phase 1A Session 6 (2026-05-13).
