---
adr: 0006
title: v0.5 — config-driven strategy composition (hot-load A)
status: accepted
date: 2026-04-19
supersedes: none
superseded-by: none
---

# ADR-0006: v0.5 — config-driven strategy composition (hot-load A)

## Context

v0 strategies are compiled in (see [ADR-0005](0005-v0-strategy-trait-no-hotload.md)).
Once the trait shape is proven, the next research iteration is
combining existing indicators (MACD, RSI, Bollinger) and rules in new
ways without recompiling. Most strategy research at the v0.5 stage is
this kind of composition — not new low-level logic, but new arrangements
of known building blocks.

## Decision

A `ComposedStrategy` type implements the `Strategy` trait; its body is
a tree of indicator + rule nodes assembled at runtime from TOML. A
file watcher on `config/strategies/` reloads on change; the registry
swaps the `Box<dyn Strategy>` atomically. No process restart.

Example:

```toml
[strategies.btc_macd_rsi]
kind   = "composed"
signal = "macd_cross(12,26,9) AND rsi(14) < 35"
size   = "fixed_fraction(0.1)"
```

This pattern covers ~70–80% of research iteration without leaving
Rust.

## Alternatives considered

- **WASM plugins at v0.5.** Same reasons as [ADR-0007](0007-v1-wasm-plugin-deferred.md)
  — composition handles most cases at lower complexity. WASM is
  reserved for genuinely custom logic.
- **Restart-required reload.** Operator pain. The file watcher + atomic
  swap is the same complexity to implement and dramatically improves
  research velocity.

## Consequences

- Atomic swap semantics live in the registry and are covered by the
  v0.5 concurrency resolution (Q2) — see the architecture.md Q&A
  blocks pending extraction to a later ADR.
- The composition language (`macd_cross(12,26,9) AND rsi(14) < 35`)
  needs a parser. Initial implementation is a thin wrapper around a
  hand-rolled recursive-descent parser; the grammar is deliberately
  small (AND/OR/NOT, comparison operators, function calls).
- Reload events emit a journal entry (`strategy_events` table, v0.5
  migration 002).

## Changelog
- 2026-04-19 (architect): initial accept. Extracted from
  `spec/architecture.md` § Strategy registry & hot-loading during
  Phase 1A Session 4 (2026-05-13).
