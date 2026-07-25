---
adr: 0013
title: v1 — cross-sectional momentum architectural resolutions (Q1–Q6)
status: accepted
date: 2026-04-29
supersedes: none
superseded-by: none
---

# ADR-0013: v1 — cross-sectional momentum architectural resolutions (Q1–Q6)

## Context

v1 introduces cross-sectional momentum on a 10-symbol universe of
Binance USDT spot pairs. The architect-round answered six questions
from the analyst's brief that were too interdependent to resolve in
isolation: data fidelity (L2 books?), funding-rate ingest, long-only
vs long/short, multi-venue, registry fan-out semantics, and how to
surface risk-gate rejections in the ledger.

All six resolutions preserve the v0 `Strategy` trait shape ([ADR-0005](0005-v0-strategy-trait-no-hotload.md))
and the v0.5 audit / broadcast / strategies-panel surfaces (ADRs
[0008](0008-v05-strategy-event-journal-schema.md),
[0011](0011-v05-cockpit-strategies-panel.md),
[0012](0012-v05-broadcast-bus-extensions.md)). The trait shape stays
firm because v1 is still inside the v0.5 hot-load regime; WASM plugins
([ADR-0007](0007-v1-wasm-plugin-deferred.md)) remain deferred.

This ADR is six interconnected decisions in one record because each
Q's answer depended on the others. Splitting them into six ADRs would
fragment the cross-references and obscure why each piece holds
together.

## Decisions

### Q1 — L2 book ingest: deferred to v1.5

v1 ships **klines + trades only**, identical fan-out shape to v0.
The v1 momentum score (R3) is close-to-close vol-adjusted return; L2
microstructure adds latency and complexity without changing the
edge. Order-book features land in v1.5 when a strategy emerges that
needs them (e.g. v1.5a pairs depth-of-book filter).

### Q2 — Funding-rate ingest: observation-only at v1

v1 wires a Binance USDT-perpetual funding-rate REST poller and
persists rows to a new `funding_rates` SQLite table. The poller emits
to the bus; v1 strategies do **not** consume funding rates (no
perp exposure). Observation-only now so the data is available when
v2 brings perps online without a separate ingest sprint.

Schema (`migrations/0003_funding_rates.sql`, in the existing audit
SQLite file — same instance keeps audit + market-context queries
joinable):

```sql
CREATE TABLE funding_rates (
    venue        TEXT NOT NULL,
    symbol       TEXT NOT NULL,
    ts           TEXT NOT NULL,
    funding_rate TEXT NOT NULL,     -- Decimal as string
    next_funding TEXT,
    PRIMARY KEY (venue, symbol, ts)
);
```

New broadcast type `FundingObs` lives in `trading_core` alongside
`Fill` / `Bar` / etc. (same placement rule as
[ADR-0012](0012-v05-broadcast-bus-extensions.md): if audit will
persist it and UI will subscribe, it lives in `trading_core`). A new
`bus.funding` channel publishes at the poll cadence (~5min per
symbol, batched).

### Q3 — Long-only spot momentum confirmed

v1 ships **long-only spot momentum** with `K_long = 3`, `K_short = 0`.
Spot crypto on Binance / Coinbase / Kraken USDT pairs has no native
short-sell mechanism; perp-based shorting belongs to v2 per
[`../product.md` § Universe & data fidelity ladder](../../../../spec/product.md#universe--data-fidelity-ladder).

`MomentumStrategy::on_bar` constructs `Vec<ProposedOrder>` with at
most `K_long` `Side::Buy` legs plus `Side::Sell` legs to close
positions falling out of the top-K. Never `Side::Sell` to open a
short.

### Q4 — Multi-venue: Binance-only at v1

v1 stays Binance-only. Multi-venue scaffolding (Coinbase, Kraken)
lands in v1.5b as a separate feature ([ADR-0017](0017-v15b-multi-venue.md)
once extracted). Each new venue is its own client + reconnect
quirks + symbol-mapping table + integration tests; bundling the
multi-venue work into v1 would have doubled the scope and delayed
the cross-sectional edge claim.

v1.5 trigger: revisit when v1's cross-sectional momentum has live
metrics that motivate a venue-diversity claim (currently a hypothetical).

### Q5 — Universe filtering: strategy-side (pattern A)

v1 strategies filter bars by symbol internally in `Strategy::on_bar`
rather than via a registry-level `interested_in()` predicate. The
registry's fan-out
(`StrategyRegistry::on_bar(&Bar) → Vec<Signal>`) is unchanged — every
strategy sees every bar; out-of-universe bars are a fast `match
symbol { in_universe => …, _ => return Vec::new() }` in the strategy.

Rationale: at v1 scale (10 symbols × 1m bars × ≤10 active
strategies), the cost of an `if !self.universe.contains(...)` check
is a single hash lookup per bar — sub-microsecond. The trait stays
minimal; no new method shape; v0 `sma_crossover` and v0.5
`ComposedStrategy` continue to filter on `bar.symbol == self.symbol`
with zero changes.

Trade-off: registry-side filtering (pattern B) would save the
per-strategy bar dispatch in the contended case (many strategies, one
bar), but at v1 cadence and concurrency the saving is unmeasurable
and the added complexity in the registry trait isn't worth it.
Implementation contract: every multi-symbol `Strategy` impl carries a
`self.universe: HashSet<Symbol>` constructed at config-load and
filters at the top of `on_bar`.

### Q6 — `RebalanceRejected` ledger surface: extend `strategy_events`

Add a new `kind = "rebalance_rejected"` variant to the v0.5
`strategy_events` table (see [ADR-0008](0008-v05-strategy-event-journal-schema.md))
rather than create a parallel `decision_events` table.

The existing schema already carries `error_code` / `error_summary`
columns and is operator-event-shaped. A rebalance rejection is a
strategy-lifecycle event (the strategy proposed an action that the
risk gate refused), not a money movement, so it belongs alongside
`Load` / `Swap` / `Unload` / `Reject`. No schema migration —
`strategy_events.kind` is a `TEXT` column; new variants are values,
not new columns.

Writer extension (in `audit::journal`, additive):

```rust
pub async fn rebalance_rejected(
    ledger: &Ledger,
    strategy_id: &str,
    error_code: &str,
    error_summary: &str,
    proposed: &ProposedOrder,
) -> Result<(), LedgerError>;
```

Reader extension (in `audit::query`): an additional filter parameter
on `strategy_events_since` to select by `kind` so the operator
success report can count rebalance rejections per session.

## Alternatives considered (cross-Q)

- **Bundle multi-venue into v1.** Doubles scope, blocks the edge
  claim. Rejected in favour of a separate v1.5b feature.
- **Add an `interested_in()` predicate to the `Strategy` trait.**
  Changes the trait shape (which ADR-0005 keeps firm), gains
  sub-microsecond saving. Rejected on cost/benefit.
- **Create a parallel `decision_events` table for Q6.** Forks the
  schema for one non-monetary row type and complicates the operator
  success report. Rejected in favour of extending
  `strategy_events`.
- **Defer funding-rate ingest until v2.** Would force a separate
  ingest sprint when perps land. Rejected — the cost of poll+persist
  now is small; the cost of debugging a missing-data dependency at
  v2 launch is higher.

## Consequences

- The v0 `Strategy` trait shape ([ADR-0005](0005-v0-strategy-trait-no-hotload.md))
  ships through v1 unchanged. v1.5 / v2 changes will need their own
  superseding ADR.
- `funding_rates` table is now part of the audit-DB schema even
  though v1 strategies don't read from it. The reports crate uses it
  for the operator success report's "data-quality" section
  (downtime detection per venue/symbol).
- The pattern "extend the existing event table with a new `kind`
  variant" is now precedent for any future non-monetary
  strategy-lifecycle event. This is the second time we've made the
  call (first: ADR-0008 establishing the sibling-table pattern).
  Both rules co-exist: balance-carrying → ledger;
  strategy-lifecycle (any kind) → `strategy_events`; new
  non-monetary domains (e.g. cost-budget alerts) → their own table.

## Changelog
- 2026-04-29 (architect): initial accept. Six interconnected
  decisions captured as a single ADR.
- 2026-05-13 (architect): extracted from `spec/architecture.md` §
  v1 — cross-sectional momentum resolutions during Phase 1A Session 7.
  Link `features/v1-cross-sectional-momentum.md` rewritten to use the
  post-folder-migration path indirectly via the in-line tracking;
  intra-block `#v05--strategy-event-journal-schema` anchors rewritten
  to cite ADR-0008.
