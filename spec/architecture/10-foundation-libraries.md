---
slug: architecture-10-foundation-libraries
status: shipped
owner: architect
updated: 2026-05-13
---

# Foundation libraries

We pull in proven Rust crates rather than reinventing them. Current
default picks below; substantive choices that defended against named
alternatives live in numbered ADRs (linked inline). The architect may
override per-feature with rationale recorded in the relevant ADR or
feature file.

## Async, observability, errors

- `tokio` (multi-thread runtime), `tokio-stream`, `futures`
- `tracing`, `tracing-subscriber`, `metrics`, `metrics-exporter-prometheus`
- `thiserror` (libraries), `anyhow` (binaries only)
- `serde`, `serde_json`, `toml`, `config`

## Numerics & ML

- `rust_decimal` — **mandatory** for prices, sizes, balances, P&L. No
  `f64` for money anywhere. See [ADR-0003](adr/0003-decimal-money-math.md).
- `ndarray`, `nalgebra` for general linear algebra.
- `polars` for in-memory DataFrame work and Parquet read/write.
- `candle` for DL prototyping; `tract` for ONNX serving.
- `linfa` for classical ML (regression, clustering) where heavier
  than RustQuant's `ml`.

## Money & currency types

Crypto doesn't fit ISO 4217 cleanly (BTC, USDT, stablecoins). We roll
our own `Money<C: Currency>` newtype around `rust_decimal::Decimal`
in `trading_core`. See [ADR-0003](adr/0003-decimal-money-math.md).
For fiat sides only: `iso_currency` (or RustQuant's `iso`). Custom
`Asset` enum for crypto symbols, sourced from venue metadata at
startup.

Do **not** use generic money crates (`moneylib`, `moneta`,
`cashmoney`) — they assume ISO currency lists.

## Quant primitives — RustQuant

Adopted as a helper, not a foundation. See [ADR-0021](adr/0021-rustquant-adoption.md)
for the module-by-module adoption table, the modules explicitly NOT
adopted, and the risk-mitigation plan (version pinning + adapter
isolation).

## Order book & matching engine

A `MatchingEngine` trait in `backtest` isolates the choice so the
implementation can swap without touching `strategy` / `risk` / `exec`.

v0 ships a simple paper engine — no LOB. See
[ADR-0026](adr/0026-v0-simple-paper-engine.md) for the v0 decision,
the alternatives deferred to v0.5 (`orderbook-rs`, `matchcore`,
`rust_ob`), and the trait-freeze rule that makes the future swap an
additive change.

## Technical analysis

Default `kand` (batch) + `quantedge-ta` (streaming), thin adapters in
`features`, no direct dependency from `strategy`. Surveyed alternatives:

- `kand` — pure-Rust TA-Lib clone, breadth comparable to TA-Lib (default pick).
- `quantedge-ta` — streaming-first; important for live bars where we
  must update per-tick instead of per-bar. Complements `kand`.
- `rust_ti` — 70+ indicators if `kand` lacks any.
- `mantis-ta` — composable indicators + strategy primitives; evaluate
  as inspiration for our `features` crate API.

## Pre-trade risk

- `openpit` — embeddable pre-trade risk SDK. Concern: our risk engine
  relies on Rust *type-level* limits (illegal orders fail at
  construction). If `openpit` is runtime-checks-only, use it as a
  second-line check, not the primary gate.

## Audit & ledger

Double-entry ledger of decisions, intents, orders, fills, and P&L
attribution. Lives in the `audit` crate.

v0 ships raw `sqlx` + `SQLite` + in-repo migrations. See
[ADR-0024](adr/0024-audit-sqlite-raw-sqlx.md) for the substantive
decision, the `sqlx-ledger` rejection rationale (Postgres-only), and
the v1+ migration path.

The v0.5 / v1 / v1.5a / v1.5b additions (strategy events, kill-switch
trip events, multi-venue typed `venue` column) layer on top — see
ADRs [0008](adr/0008-v05-strategy-event-journal-schema.md),
[0013](adr/0013-v1-cross-sectional-momentum.md),
[0014](adr/0014-v15a-mean-reversion-pairs.md),
[0015](adr/0015-operator-success-reports.md),
[0017](adr/0017-v15b-multi-venue.md).

## Tick aggregation

- `trade_aggregation` — proven candle aggregator from raw trades.
  Adopt for the tick → OHLCV path in `data`; avoids a class of
  off-by-one bugs.

## Cost telemetry

Standalone `cost` crate. See [ADR-0022](adr/0022-cost-telemetry-crate.md)
for the placement rationale and the `CostEvent` / `CostSink` /
`CostBudget` surface.

## Frontend — iced

Single UI stack across the project. See [ADR-0023](adr/0023-iced-frontend.md)
for the high-level decision. The detailed UI architecture (cockpit
screen routing, `audit::query` read-only surface, KPI strip, widget
contracts, Lumen design system integration) lives in
[`06-ui-and-cockpit.md`](06-ui-and-cockpit.md).

## Data / venues

- `reqwest` + `tokio-tungstenite` for REST and WebSocket feeds.
- `yahoo_finance_api` or `yfinance-rs` for optional macro overlay
  (DXY, SPX, US10Y) — evaluate `yfinance-rs` first.
- `clap` for CLI binaries.
- `time` workspace-wide (RustQuant uses `time`; we default to `time`
  to avoid two date libraries).

v0 ships a hand-rolled Binance WS adapter behind a `MarketDataSource`
trait. See [ADR-0025](adr/0025-hand-rolled-binance-ws.md) for the
trait shape, the rationale, and the alternatives surveyed
(`binance-rs-async`, `barter-data`, `ccxt-rs`, full hand-rolled with
no trait). v1.5b extends with Coinbase and Kraken; see
[ADR-0017](adr/0017-v15b-multi-venue.md).

**Explicitly NOT adopted from `lib.rs/finance`**: FIX engines
(`easyfix`, `fixer`, `quickfix`); institutional equities/futures data
(`databento`, `dbn`, `sec-fetcher`); personal finance ledgers
(`rustledger`, `tackler`, `aledgr`, `hledger-fmt`); payments / bank
statements (`async-stripe`, `mt940`, `ofx-rs`); options pricing
(`black_scholes`, `volsurf`, `optionrs`, `implied-vol`,
`stock-options`); `simm-rs`; unclear-maturity crates (`digifi`,
`finquant`, `quantrs`, `alfars`, `finalytics`).

## LLM

- `anthropic-sdk` (or our own thin client around `reqwest` if the SDK lags)
- `async-openai` for OpenAI-compatible providers (covers OpenRouter,
  DeepSeek, LM Studio)
- Custom tool-use schema layer in `llm` crate. See
  [ADR-0019](adr/0019-v2-llm-strategy.md) for the v2 LLM-strategy
  foundation decisions.

## Testing

- `proptest` for property tests on strategy invariants.
- `criterion` for benchmarks.
- `insta` for snapshot tests on prompt + report rendering.

## Changelog
- 2026-05-13 (architect): body migrated from `spec/architecture.md` §
  Foundation libraries during Phase 1A Session 11. Six substantive
  decisions extracted to ADRs 0021–0026 (RustQuant adoption, cost
  telemetry crate, iced frontend, audit ledger storage, Binance WS
  adapter, v0 paper engine). The iced UI architecture body moved to
  [`06-ui-and-cockpit.md`](06-ui-and-cockpit.md).
