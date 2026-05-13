---
slug: architecture-03-execution-and-venues
status: shipped
owner: architect
updated: 2026-05-13
---

# Execution and venues

The boundary between strategy decisions and the outside world:
order routing, paper-trade vs live execution, venue clients, and
the `MatchingEngine` abstraction that makes the choice swappable.

## Order routing

Strategies emit `Vec<ProposedOrder>` (or a `Signal` that the runtime
wraps into a proposed order). The `risk` crate intervenes between
strategy and execution — sizing, clamping, and rejecting per
configured limits. Only orders that pass risk reach
`crates/exec/`. See [`04-risk-and-money.md` § Risk engine](04-risk-and-money.md#risk-engine).

The "strategy proposes, risk disposes" rule
([ADR-0014](adr/0014-v15a-mean-reversion-pairs.md) Q9) applies to
every multi-leg / multi-symbol strategy: the strategy is unaware of
its risk envelope; risk clamps to the current state.

## Paper engine vs full LOB

v0 ships a simple paper engine that fills market orders at
`bar.close ± slippage_bps`. See
[ADR-0026](adr/0026-v0-simple-paper-engine.md) for the v0 decision,
the `MatchingEngine` trait freeze rule, and the v0.5+ alternatives
(`orderbook-rs`, `matchcore`, `rust_ob`) queued behind real
limit-order requirements. The simple engine carries the project
through v0.5 / v1 / v1.5a — every strategy released to date emits
market orders, so the LOB-pick decision is on a longer timer than
originally planned.

## Venue clients

v0: single venue (Binance spot), hand-rolled WebSocket adapter
behind a `MarketDataSource` trait. See
[ADR-0025](adr/0025-hand-rolled-binance-ws.md) for the trait shape
and the rejected multi-venue framework alternatives
(`barter-data`, `binance-rs-async`, `ccxt-rs`).

v1.5b: multi-venue execution scaffolding (Coinbase + Kraken alongside
Binance) via the typed `Venue` enum and per-venue
`tokio::JoinSet` isolation. See
[ADR-0017](adr/0017-v15b-multi-venue.md) for the twelve decisions
covering venue type shape, Coinbase Advanced Trade WS, ingest
topology, client-side 1s bar aggregation, USDC universe expansion,
failover via `bus.market_health`, free unauthenticated WS, rate
limits, test harness, audit migration 007, and zero-by-construction
anchor risk.

Anchor coverage stayed at 11/11 byte-identical across the v1.5b
ship — multi-venue is purely additive at the report-body level.

## Live execution

Not yet shipped. The trait shapes (`MatchingEngine`,
`MarketDataSource`) are deliberately the same surfaces a future
live-execution crate would consume — the
[ADR-0026](adr/0026-v0-simple-paper-engine.md) trait freeze and
[ADR-0025](adr/0025-hand-rolled-binance-ws.md) `MarketDataSource`
trait both ship with this in mind. When real-money execution
lands, it lands as a sibling implementation of `MatchingEngine`,
not as a refactor.

The cost-economics gating for live-execution lives in
[`../product.md` § Project scope boundary](../product.md#project-scope-boundary)
and is intentionally outside this file's scope.

## Authentication

Free, unauthenticated WS market data for all three venues at v1.5b
([ADR-0017](adr/0017-v15b-multi-venue.md) Q8). Zero secrets surface
at the v1.5b ship; auth shows up only when live-execution lands as
a separate sprint.

When auth arrives: secrets live in env / secret store per the
non-negotiable in [`../../CLAUDE.md`](../../CLAUDE.md). The audit
ledger ([ADR-0024](adr/0024-audit-sqlite-raw-sqlx.md)) captures
the order-side surface only; secrets never enter the journal.

## Changelog
- 2026-05-13 (architect): body synthesised from existing ADRs
  during Phase 1A Session 12. No content was in the monolith to
  migrate — this file aggregates references to ADRs 0014, 0017,
  0024, 0025, 0026 plus the `04-risk-and-money.md` cross-link.
