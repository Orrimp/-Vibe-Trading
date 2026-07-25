---
adr: 0025
title: v0 ships a hand-rolled Binance WS adapter behind a `MarketDataSource` trait
status: accepted
date: 2026-04-17
supersedes: none
superseded-by: none
---

# ADR-0025: v0 ships a hand-rolled Binance WS adapter behind a `MarketDataSource` trait

## Context

v0 needs market data from a single venue (Binance spot) for one
symbol (BTCUSDT) on two streams (klines + trades). The crate
`binance-rs-async` exists but pulls in REST + margin/futures
endpoints we don't use, has historically slow release cadence on
upstream venue changes, and locks us to one venue. Multi-venue
adapters (`barter-data`) and broad-venue clients (`ccxt-rs`) exist
but their abstractions are heavy at v0 scale. The question is
whether the v0 surface should buy or build.

## Decision

v0 rolls its own thin adapter against the Binance spot WebSocket
using `tokio-tungstenite` + `serde` + `reqwest` (for the one-shot
symbol metadata / exchange-info fetch). Two streams only:
`btcusdt@kline_1m` and `btcusdt@trade`. Reconnect with exponential
backoff, heartbeat via ping/pong, optional testnet endpoint.

Everything is isolated behind a `MarketDataSource` trait in
`crates/data/` so the implementation is swappable:

```rust
#[async_trait]
pub trait MarketDataSource: Send + Sync {
    /// Symbol metadata + filters fetched once at startup.
    async fn exchange_info(&self, symbol: Symbol) -> Result<SymbolInfo, FeedError>;

    /// Bar stream (kline, venue-closed bars only).
    async fn subscribe_bars(&self, symbol: Symbol, tf: Timeframe)
        -> Result<BoxStream<'static, Result<Bar, FeedError>>, FeedError>;

    /// Raw trade stream (aggregated @trade channel).
    async fn subscribe_trades(&self, symbol: Symbol)
        -> Result<BoxStream<'static, Result<Tick, FeedError>>, FeedError>;
}
```

Implementations in v0: `BinanceFeed`, `ReplayFeed` (drives the same
trait off a Parquet fixture for backtests and UI smoke), `FakeFeed`
(in-memory for unit tests).

## Alternatives considered

- **`binance-rs-async`.** Binance-only, async, reasonable quality,
  but pulls REST + margin/futures endpoints we don't use; slow
  release cadence on upstream venue changes. Rejected — good
  fallback only if the hand-rolled adapter slips week 1.
- **`barter-data`.** Strong v0.5 candidate: normalised multi-venue
  streams (Binance / Coinbase / Kraken), converts into a single
  `MarketEvent` type, streaming-first. Overkill for v0's single
  venue; revisit when a second venue lands. Rejected for v0.
- **`ccxt-rs`.** CCXT port; broad venue list but historically thin
  surface and uneven async support. Rejected.
- **Full hand-rolled with no trait.** The trait is cheap insurance
  against venue lock-in and is the only thing the `strategy` /
  backtest code sees. Rejected.

## Consequences

- The v0 adapter is ~200 lines of serde + a reconnect loop. Cleanly
  deletable once v0.5+ multi-venue drives a real adapter pick.
- Strategy code, backtest code, and tests see only the
  `MarketDataSource` trait — never `BinanceFeed` directly. Mock
  feeds compose naturally.
- **Deletion criterion**: when a second venue enters the universe,
  re-evaluate. v1.5b ([ADR-0017](0017-v15b-multi-venue.md)) added
  Coinbase + Kraken via the same trait-extension pattern rather
  than adopting `barter-data` — the hand-rolled approach scaled to
  three venues at lower lock-in cost than a multi-venue framework.

## Changelog
- 2026-04-17 (architect): initial accept.
- 2026-05-13 (architect): extracted from `spec/architecture.md` §
  Foundation libraries — Data / venues during Phase 1A Session 11.
