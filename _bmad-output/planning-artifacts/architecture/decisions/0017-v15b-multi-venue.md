---
adr: 0017
title: v1.5b — multi-venue execution scaffolding (Q1–Q12)
status: accepted
date: 2026-05-03
supersedes: none
superseded-by: none
---

# ADR-0017: v1.5b — multi-venue execution scaffolding (Q1–Q12)

## Context

v1.5b lands Coinbase + Kraken alongside the existing Binance feed,
introducing the typed `Venue` system and per-venue task isolation.
Twelve Q's covered the `Venue` type, three venue clients, ingest
topology, schema changes, USDC universe expansion, failover,
authentication, rate limits, test harness, audit migration, and the
anchor risk surface. The largest queued backend feature at v1.5
time. All twelve preserve the 11/11 anchor body-stability invariant
by construction.

## Decisions

### Q1 — `Venue` type: closed enum in `trading_core`

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Venue { Binance, Coinbase, Kraken }
```

Closed enum (not an open trait or string) so exhaustive `match`
catches the multi-venue branches at compile time. Lives in
`trading_core::venue`; every crate that touches venue logic
imports it.

### Q2 — Coinbase: Advanced Trade WebSocket

Target `wss://advanced-trade-ws.coinbase.com`. Free, unauthenticated
for market data, supports klines + trades fan-out. Subscription
format mirrors Binance's combined-stream shape closely enough for
a common `MarketStream` trait.

### Q3 — Ingest topology: per-venue `tokio::JoinSet`

`agent::runtime::run` spawns one `tokio::JoinSet` task per enabled
venue. Panic isolation — a Coinbase parser panic does NOT poison
Binance / Kraken. Each task reconnects independently; the
`market_health` channel publishes per-venue freshness.

### Q4 — `venue` field on `Tick` / `Bar`: required

`Tick.venue: Venue` and `Bar.venue: Venue` are required fields,
not `Option<Venue>`. Mechanical migration across ~30+ fixture
sites; all literals default to `Venue::Binance`. Forces every
strategy that's aware of multi-venue to handle the branch
explicitly.

### Q5 — 1s bar aggregation: client-side

New `crates/data/src/bar_aggregator.rs` aggregates 1s bars
client-side on `i64` epoch microseconds. Deterministic; no venue
inconsistency. Coinbase emits raw trades; Kraken emits raw trades
with a different shape; both feed a common aggregator that emits
identical 1s bars.

### Q6 — USDC universe: doubled (operator-gated)

New `[universe]` section in `config/agent.toml` with `usdt_enabled`
/ `usdc_enabled` toggles. Default: USDT only. Operator opt-in
expands the universe by 10 USDC mirror pairs. Pre-trade risk caps
scale per the active universe count.

### Q7 — Failover: per-venue stale-data pause + bus event

`MarketHealth { Fresh, Stale, Recovered }` enum publishes on a new
`bus.market_health` channel (capacity 64). Stale = no bar in N
seconds (configurable per venue, default 90s). On stale: the
strategy receives a `MarketHealth::Stale(venue)` event and SHOULD
pause emitting orders against that venue. Architect-level rule
(strategy-side, not registry-side per
[ADR-0013](0013-v1-cross-sectional-momentum.md) Q5).

### Q8 — Authentication: free unauthenticated WS for all three

Binance / Coinbase / Kraken public WS market data are all free and
unauthenticated. Zero secrets surface at v1.5b. Auth shows up only
when live order execution lands (separate sprint).

### Q9 — Rate limits: 30–60 subscription slots within free tier

All three venues' free tiers comfortably accommodate the v1.5b
universe (10 USDT + 10 USDC pairs × 3 venues × klines+trades ≈ 120
subscriptions, well under per-venue limits). No backoff infra
needed at v1.5b.

### Q10 — Test harness: `MockFeed` over `wiremock`

New `crates/data/src/mock_feed.rs` for unit + integration tests.
Replays canned exchange WS frames; supports synthesized
disconnect/reconnect cycles. No external `wiremock` server needed
for the common cases — uses in-memory channels.

### Q11 — Audit migration 007: additive ALTER TABLE

Migration `007_strategy_events_venue.sql` — purely additive
`ALTER TABLE strategy_events ADD COLUMN venue TEXT;` (NULLABLE, no
default). Pre-migration rows have `venue = NULL`; readers handle
`Option<Venue>` semantics. Writer signature change:
`feed_reconnect(ledger, symbol, venue, ts)` gains required
`venue: Venue`. `kill_switch_tripped` writer gains optional
`venue: Option<Venue>` (R8.3 — kill-switch may not be venue-scoped).

Architect's principled override of analyst's R8.2 recommendation
(encode in `error_summary`): typed column wins because v1.5b is the
load-bearing introduction of the `Venue` type, and audit is the
boundary where typed attribution matters most.

### Q12 — Anchor risk: zero by construction (re-confirmed)

Independent grep on `spec/*/reports/backtest-*.md` and
`spec/operator-success-reports/reports/success-*.md` returned zero
hits on `venue|coinbase|kraken`. All 11 anchors stay byte-identical
across the v1.5b ship. The v1.5b path is purely additive at the
report-body level.

## Alternatives considered

- **Open `Venue` trait.** Surrenders compile-time exhaustiveness;
  every new venue adds opportunities for missed `match` arms.
  Rejected.
- **Server-side bar aggregation.** Surrenders determinism (each
  venue has its own clock skew). Rejected.
- **Encode venue in `strategy_events.error_summary`** (analyst's
  R8.2). Loses typed query capability; complicates the operator
  success report's per-venue rollup. Rejected.

## Consequences

- The closed `Venue` enum is now project-load-bearing. Adding a
  fourth venue (Bybit, Bitfinex, OKX) requires touching every
  exhaustive `match`; that's the desired forcing function.
- `bus.market_health` is a new permanent bus channel. Cockpit's
  status-bar widget consumes it for the per-venue freshness
  badges.
- The "writer-signature-change-not-schema-change" pattern (Q11 —
  required venue on `feed_reconnect`, optional on
  `kill_switch_tripped`) is the precedent for typed audit
  attribution. Future venues / dimensions can extend writer
  signatures without touching the schema.

## Changelog
- 2026-05-03 (architect): initial accept.
- 2026-05-13 (architect): extracted from `spec/architecture.md` §
  v1.5b — multi-venue resolutions during Phase 1A Session 9.
