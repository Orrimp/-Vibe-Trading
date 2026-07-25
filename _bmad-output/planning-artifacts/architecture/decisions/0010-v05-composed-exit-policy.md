---
adr: 0010
title: v0.5 — ComposedStrategy uses symmetric signal-flip exit; drawdown lives in risk
status: accepted
date: 2026-04-19
supersedes: none
superseded-by: none
---

# ADR-0010: v0.5 — ComposedStrategy uses symmetric signal-flip exit; drawdown lives in risk

## Context

v0.5's `ComposedStrategy` (the config-driven hot-loadable strategy
type from [ADR-0006](0006-v05-config-driven-composition.md)) needs an
exit policy. The obvious choices: (a) symmetric signal-flip — exit
when the rule that triggered entry goes false; (b) drawdown-triggered
exit inside the strategy itself; (c) a first-class DSL exit node. The
choice has downstream consequences for the rule DSL's shape and for
where per-strategy drawdown control lives.

## Decision

Symmetric signal-flip only. When the rule transitions `true → false`
the strategy emits `Sell` to close. No drawdown-triggered exit lives
inside `ComposedStrategy`. v0.5 matches v0 `sma_crossover`
edge-triggered semantics: buy on `false → true`, sell on `true →
false`.

Per-strategy drawdown control belongs in the `risk` crate, not in the
signal tree. The `risk` crate already owns `size_and_validate` and
the portfolio-level `max_drawdown_stop_pct` floor; a per-strategy
drawdown limit is a natural extension on the risk-limits struct.

## Alternatives considered

- **Ship drawdown-triggered exit inside `ComposedStrategy` in v0.5.**
  Bloats the rule tree, forces state (last-high, drawdown counter)
  into every node, and duplicates a risk concern. Rejected.
- **Make drawdown a first-class DSL node (`if drawdown(20) > 0.05
  then close`).** Possible but premature; commit after seeing real
  drawdown patterns in v1 paper trading. Deferred.

## Consequences

- v0.5 `ComposedStrategy` rule trees stay stateless beyond the
  indicator caches — the DSL grammar is simpler and testable.
- The v1+ hook: `risk::RiskLimits` grows an optional
  `max_strategy_drawdown_pct: Option<Decimal>` field;
  `risk::size_and_validate` clamps to zero (and emits a
  `StrategyRiskTripped` audit event) when a specific strategy's
  cumulative drawdown passes the limit. Leave a `// TODO(v1):
  max_strategy_drawdown` breadcrumb in the v0.5 Design section so the
  developer doesn't invent the feature in v0.5.
- Pattern: strategy concerns vs risk concerns are different
  responsibilities. Anything stateful about money or drawdown lives
  in `risk`. Anything about signal logic lives in `strategy`. This
  ADR is the first place that boundary is written down explicitly.

## Changelog
- 2026-04-19 (architect): initial accept. Extracted from
  `spec/architecture.md` § v0.5 — ComposedStrategy exit policy (Q3)
  during Phase 1A Session 6 (2026-05-13).
