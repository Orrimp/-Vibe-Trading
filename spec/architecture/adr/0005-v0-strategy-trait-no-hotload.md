---
adr: 0005
title: v0 — clean strategy trait shape with no hot-loading
status: accepted
date: 2026-04-17
supersedes: none
superseded-by: none
---

# ADR-0005: v0 — clean strategy trait shape with no hot-loading

## Context

v0 is the foundation release: one strategy (SMA crossover), one
exchange (Binance), paper-trading only. Hot-loading at v0 would be
premature complexity — the goal is to lock down the trait shape so
later hot-load mechanisms (v0.5 config-driven, v1+ WASM) don't churn
the public API every strategy author depends on.

## Decision

The `Strategy` trait shape is fixed at v0 and does **not change** to
accommodate later hot-loading. Strategies are compiled in. The registry
is a `HashMap<StrategyId, Box<dyn Strategy>>` populated at startup from
`config/agent.toml`. v0 ships only `sma_crossover`.

```rust
pub trait Strategy: Send + Sync {
    fn id(&self) -> StrategyId;
    fn on_bar(&mut self, bar: &Bar) -> Vec<Signal>;
    fn on_tick(&mut self, tick: &Tick) -> Vec<Signal>;
    fn config_schema() -> serde_json::Value where Self: Sized;
}
```

## Alternatives considered

- **Start with hot-loading at v0.** Would have meant designing the WASM
  ABI or the composition language before the trait shape was proven by
  one working strategy. Rejected — the trait would be designed to fit
  the loader rather than the domain.
- **Different trait per timeframe (e.g. `TickStrategy` vs
  `BarStrategy`).** Considered; rejected because strategies that
  consume both (SMA crossover already does in some variants) would
  need to compose two traits. Single trait with both methods is
  simpler.

## Consequences

- Adding a method to the `Strategy` trait is an ADR-worthy event after
  v0 — it forces every existing strategy to update and breaks the WASM
  ABI when v1+ lands.
- Strategies compiled in means tests use the same code paths as
  production. The backtest engine instantiates a `Box<dyn Strategy>`
  identically to live.
- The `config_schema()` method returns a JSON schema so cockpit /
  viewer UIs can render a strategy's config without hard-coding the
  shape. Future-proofing for [ADR-0006](0006-v05-config-driven-composition.md)
  composition.

## Changelog
- 2026-04-17 (architect): initial accept. Extracted from
  `spec/architecture.md` § Strategy registry & hot-loading during
  Phase 1A Session 4 (2026-05-13).
