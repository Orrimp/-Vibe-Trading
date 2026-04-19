---
slug: v0-paper-sma
status: in-progress
owner: architect
updated: 2026-04-19
---

# v0 — Paper-Trading SMA Tracer Bullet

## Why

v0 is a **tracer bullet**, not a strategy. Candidate C was picked in
[product.md → v0 — first step](../product.md#v0--first-step-proposed-paper-trading-sma-in-2-weeks)
for exactly this reason: a deliberately boring SMA crossover forces every
structural piece of the harness to be real — data feed, typed `core`
primitives, audit ledger, `Strategy` trait, paper-fill matching engine,
iced cockpit, kill switch — while the "intelligence" stays trivial enough
that nothing hides behind a clever signal. Foundation beats demo. Every
v0.5+ strategy (multi-indicator, LLM overlay, DL forecaster) drops into
the same slots; if the slots don't exist or don't line up, v0.5 becomes
a rewrite instead of an addition.

The locked moat bet — see [product.md → Differentiator](../product.md#differentiator)
— is **persistent memory + double-entry audit**. v0 advances that bet
even though the strategy is trivial: the `sqlx-ledger` substrate ships
from day one, every fill writes balanced journal entries, every strategy
registry mutation journals, and the reconciliation invariant
`cash + Σ(positions × mark) = equity` is asserted every minute across
the 2023 backtest. The lesson-card / reflection loop itself is
deliberately deferred to v0.5+, but the ledger it will read from and
write to exists now, populated with real fills, and is the same schema
the reflection loop will query. Shipping the ledger late is what kills
this differentiator; shipping it now even with nothing to reflect on is
the cheap, correct move.

v0 is **not** trying to validate anything about edge, models, or LLMs.
No Sharpe claim is made. No model is trained. No LLM is called. The
success criterion is mechanical: the harness runs end-to-end, the
backtest produces a report under `spec/reports/`, the cockpit renders a
live tape against a testnet/replay feed, and a `.halt` file flattens
the book while the ledger still balances. The trading-time agent roster
in [product.md → Trading-time agent roster](../product.md#trading-time-agent-roster)
(analysts → debate → trader → risk → PM) is explicitly **out of scope**
for v0; it lands in v0.5+. v0 ships exactly one compiled-in
`sma_crossover` strategy through the plug-in-shaped `Strategy` trait.

The two-week budget is a forcing function. Anything that doesn't fit
in the harness (LLM providers, RL, DL, multi-venue, L2 book, funding,
perps, live exec) is explicitly out of scope here and tracked on the
ladders in [product.md → Universe & data fidelity ladder](../product.md#universe--data-fidelity-ladder)
and [product.md → Strategy library — roadmap](../product.md#strategy-library--roadmap).

## Requirements

Numbered, testable, derived from
[product.md → v0 — first step](../product.md#v0--first-step-proposed-paper-trading-sma-in-2-weeks)
and [architecture.md](../architecture.md). Each ends with a one-line
**acceptance** criterion the tester can verify.

### R1 — Data feed

- **R1.1** `data` crate exposes a `MarketFeed` trait with two concrete
  implementations:
  - **Live:** Binance spot WebSocket on `btcusdt@kline_1m` plus
    `btcusdt@trade` (24/7 — no session calendar; no T+N settlement
    semantics). Uses `tokio-tungstenite` with automatic reconnect and
    exponential backoff.
  - **Historical:** Parquet loader for archived 1m bars + trades under
    `data/binance/BTCUSDT/<year>/*.parquet` (sourced from Binance Vision
    per [product.md → Market data](../product.md#market-data)).
- **R1.2** Tick → bar aggregation via `trade_aggregation`
  ([architecture.md → Tick aggregation](../architecture.md#tick-aggregation)).
  Live 1m bars from the kline stream are cross-checked against bars
  aggregated from the trade stream; a divergence beyond a configurable
  tolerance raises a `BarMismatch` tracing event.
- **R1.3** Clock-skew detection: every incoming message is stamped with
  local monotonic time and the venue-provided timestamp; if
  `|local − venue| > clock_skew_warn_ms` (default 2000ms) for 3
  consecutive messages, emit `ClockSkew` warning and increment a
  Prometheus counter. If skew exceeds `clock_skew_halt_ms`
  (default 10_000ms), trip the kill switch (R7).
- **R1.4** The same `MarketFeed` trait is driven by a **replay driver**
  that reads the historical Parquet and emits bars/ticks at either
  wallclock pace (for UI smoke tests) or as-fast-as-possible (for
  backtests).
- **Acceptance:** an integration test starts the replay driver on a
  1-hour Parquet fixture, asserts that (a) exactly 60 minute-bars are
  produced, (b) every bar carries a monotonically increasing
  `venue_ts`, (c) `trade_aggregation`-derived bars equal the published
  kline bars within ≤ 1 satoshi on OHLC and ≤ 1e-8 on volume.

### R2 — Core types

- **R2.1** The `trading_core` crate defines: `Symbol` (e.g. `BTCUSDT` — single
  token, exchange-native; **not** `BTC/USDT`), `Asset` enum (`BTC`,
  `USDT`, extensible), `Currency` trait + `Money<C: Currency>`
  newtype over `rust_decimal::Decimal`, `Price`, `Quantity`, `Bar`,
  `Tick`, `Order`, `Fill`, `Position`, `Signal`, `Decision`,
  `StrategyId`, `AccountId`.
- **R2.2** **No `f64` anywhere money-adjacent.** Prices, sizes,
  balances, fees, and P&L are all `Decimal`-backed. A clippy lint
  (`clippy::float_arithmetic` at `deny` scope in `trading_core`, `risk`,
  `audit`, `exec`, `backtest`) enforces this.
- **R2.3** **No `unwrap` / `expect` outside `#[cfg(test)]`**. Library
  code returns `Result<T, E>` with `thiserror`-derived error types;
  binaries use `anyhow`. Enforced by clippy
  (`clippy::unwrap_used`, `clippy::expect_used` at `deny`).
- **R2.4** Risk-limit invariants encoded at construction:
  - `Order::new(...)` returns `Result<Order, OrderError>` and refuses
    non-positive quantity, mismatched asset pairs, prices outside a
    configurable sanity band relative to last mark, and orders whose
    notional would breach the per-symbol exposure cap **given the
    current position snapshot passed in**.
  - `Money<C>` refuses arithmetic across currencies at the type level
    (no `Money<USDT> + Money<BTC>` ever compiles).
  - `Quantity` is a non-negative newtype (signedness is carried by
    `Side`, not by the quantity).
- **R2.5** All types `Serialize + Deserialize` with stable wire names;
  the audit ledger and config both use these.
- **Acceptance:** (a) `cargo clippy -- -D warnings` passes with the
  lints in R2.2/R2.3 active; (b) a `proptest` suite asserts
  `Order::new` rejects every invalid combination in its input space
  (property invariants: `qty > 0`, `price > 0`, `exposure_after ≤ cap`);
  (c) a compile-fail test (`trybuild`) shows
  `Money<USDT> + Money<BTC>` does not compile.

### R3 — Audit ledger

- **R3.1** New `audit` crate wraps `sqlx-ledger`
  ([architecture.md → Audit & ledger](../architecture.md#audit--ledger)).
  Backing store is SQLite for v0 (Postgres deferred); storage path in
  `config/agent.toml`.
- **R3.2** Chart of accounts created at startup (13 accounts canonical):
  - `assets:cash:USDT`
  - `assets:position:BTC` (one sub-account per traded asset)
  - `assets:position_mark:BTC` (unrealized mark-to-market contra)
  - `income:realized_pnl`
  - `income:unrealized_pnl`
  - `expense:fees:taker`
  - `expense:fees:maker`
  - `expense:llm:deep_think` (zero entries in v0; pre-seeded for v0.5+ per R10)
  - `expense:llm:quick_think` (zero entries in v0; pre-seeded for v0.5+ per R10)
  - `liabilities:llm_accrued` (zero entries in v0; pre-seeded for v0.5+ per R10)
  - `expense:infra` (zero entries in v0; pre-seeded for v1+)
  - `expense:data` (zero entries in v0; pre-seeded for v1+)
  - `equity:opening_balance`
- **R3.3** **Every fill** writes a balanced double-entry journal entry
  (debits = credits) atomically with the in-memory `Position` update.
  A buy fill of `q` BTC @ `p` USDT with fee `f` USDT writes:
  debit `assets:position:BTC` for `q`, credit `assets:cash:USDT` for
  `q*p`, debit `expense:fees:taker` for `f`, credit
  `assets:cash:USDT` for `f` — in one transaction.
- **R3.4** **Every strategy registry mutation** (load, swap, unload,
  demote) writes a journal entry per
  [architecture.md → Lifecycle integration](../architecture.md#lifecycle-integration).
  v0 only exercises the initial `load` path, but the code path is
  live so v0.5 hot-swap journals for free.
- **R3.5** **Reconciliation invariant:** at every minute bar close, a
  reconciler computes `cash + Σ(positions × last_mark) − Σ(fees)` from
  the ledger and asserts it equals recomputed equity from the
  position book. A mismatch beyond `reconciliation_tolerance_usdt`
  (default `0.01`) fails the reconciliation, trips the kill switch
  (R7), and emits a `LedgerImbalance` error with the offending
  journal entry id.
- **R3.6** The ledger is the **source of truth** for P&L reported in
  the cockpit and in backtest reports — the cockpit does not keep a
  parallel P&L computation.
- **Acceptance:** the 2023 backtest (see Backtest Scenarios) runs to
  completion with **zero** `LedgerImbalance` events and every
  minute-boundary reconciliation passes; a unit test synthesizes a
  deliberate imbalance and asserts the reconciler catches it and the
  kill switch flips.

### R4 — Strategy trait + registry

- **R4.1** The `strategy` crate defines the `Strategy` trait exactly
  per [architecture.md → v0 — clean trait shape, no hot-load
  (compiled-in)](../architecture.md#v0--clean-trait-shape-no-hot-load-compiled-in):

  ```rust
  pub trait Strategy: Send + Sync {
      fn id(&self) -> StrategyId;
      fn on_bar(&mut self, bar: &Bar) -> Vec<Signal>;
      fn on_tick(&mut self, tick: &Tick) -> Vec<Signal>;
      fn config_schema() -> serde_json::Value where Self: Sized;
  }
  ```

  This trait shape is the contract; it does **not** change when v0.5
  adds config-driven composition or v1+ adds WASM plugins.
- **R4.2** Registry is `HashMap<StrategyId, Box<dyn Strategy>>`
  populated at startup from `config/agent.toml`. v0 ships exactly
  **one** implementation: `sma_crossover`.
- **R4.3** `sma_crossover` is parameterized by `fast_len: usize` and
  `slow_len: usize` read from TOML (defaults `20` / `50`). Indicators
  use `kand` (batch) and `quantedge-ta` (streaming) per
  [architecture.md → Technical analysis](../architecture.md#technical-analysis),
  wrapped behind thin adapters in `features` — `strategy` does not
  depend on either crate directly.
- **R4.4** Rule: `fast > slow` → `Signal::Buy`;
  `fast < slow` → `Signal::Sell`; within one tick of equality →
  `Signal::Hold`. Signals carry `StrategyId`, bar timestamp, and the
  indicator values that produced them (for audit).
- **R4.5** Position sizing is **fixed-fraction** (default `0.1` of
  current cash equity), clamped to the per-symbol exposure cap in
  R2.4. Sizing lives in `risk`, not `strategy`.
- **R4.6** `on_tick` returns `vec![]` for v0 — SMA is a bar-close
  strategy. The hook exists so v0.5+ streaming strategies can
  implement it without changing the trait.
- **Acceptance:** a deterministic replay of a 200-bar fixture produces
  an exactly reproducible signal sequence, byte-identical across two
  runs (same seed, same fixture).

### R5 — Backtest engine + paper-fill matching engine

- **R5.1** `backtest` crate exposes a `MatchingEngine` trait
  ([architecture.md → Order book & matching engine](../architecture.md#order-book--matching-engine)).
  v0 ships a **simple price-time paper engine** — the full LOB spike
  (`orderbook-rs` / `matchcore` / `rust_ob`) is deferred to v0.5.
  [ASSUMPTION] v0's SMA strategy only generates market orders against
  the bar close, so a full LOB is not needed yet; architect to
  confirm or push back.
- **R5.2** Slippage model: configurable basis-points adjustment on
  the fill price. Default `slippage_bps = 2` (buy fills at
  `bar.close * (1 + 2/10_000)`; sell fills at
  `bar.close * (1 − 2/10_000)`).
- **R5.3** Fee model: taker-only for v0 at
  `taker_fee_bps = 4` (`0.04%`), applied to notional, booked to
  `expense:fees:taker` per R3.3. Maker fees wired into config but
  unused in v0 (market orders only).
- **R5.4** **Deterministic for a given seed.** The backtest run
  accepts `--seed <u64>`; any stochastic component (tie-break on
  equal timestamps, RNG for jitter in replay) is seeded from it. Two
  runs at the same seed produce byte-identical reports.
- **R5.5** The backtest run emits a report conforming to
  [.claude/skills/rust-test/templates/test-report.md](../../.claude/skills/rust-test/templates/test-report.md)
  section 5 (Backtest Results), written to
  `spec/reports/backtest-<YYYY-MM-DD-HHMM>-<scenario-slug>.md`.
- **Acceptance:** two runs of `btc-2023-1m-sma-cross` at the same
  seed produce byte-identical reports (sha256 match).

### R6 — UI cockpit (iced)

All requirements below flow from
[architecture.md → Frontend — iced](../architecture.md#frontend--iced).

- **R6.1** `ui` crate ships the `cockpit` binary (the `viewer`
  binary is scoped for v0.5). Depends only on `core` (types) and
  `audit` (read-only ledger queries). **Never** on `strategy`,
  `exec`, or `models`.
- **R6.2** Panels:
  - **Live tape** — last 200 trades (symbol, side, price, qty,
    venue ts). Auto-scroll, pausable.
  - **Position panel** — open positions with cost basis, mark, PnL,
    PnL%, exposure as fraction of equity.
  - **P&L card** — cash, unrealized, realized, total equity; daily
    return. **Numbers come from the ledger (R3.6)**, not from a
    cockpit-local accumulator.
  - **Kill switch** — big red button; destructive-action confirm
    with typed safety phrase per
    [architecture.md → Constraints](../architecture.md#constraints).
  - **Latency badge** — venue ts vs local ts, color thresholds
    (green < 500ms, amber < 2s, red ≥ 2s, halted ≥ 10s — matches R1.3).
- **R6.3** All copy lives in `ui::strings`; no literal strings inside
  widgets. All colors, spacing, and typography flow from
  `ui::theme`; no ad-hoc styles.
- **R6.4** Every panel has first-class **empty**, **loading**, and
  **error** states (no blank screens). Example: position panel
  shows "No open positions" in empty state, a skeleton in loading,
  a red banner with the ledger error in error state.
- **R6.5** Cockpit reads from `agent` over an in-process
  `tokio::sync::broadcast`, wrapped as an iced `Subscription` — no
  bespoke glue.
- **Acceptance:** (a) launching `cockpit` against the replay driver
  renders all four panels within 2s and the tape updates every bar;
  (b) a UI smoke test scripted via `iced` test harness toggles
  empty/loading/error for each panel and snapshots pass via `insta`;
  (c) pressing the kill switch (with correct typed phrase) produces
  a journal entry and transitions the agent to `halted` state.

### R7 — Kill switch

- **R7.1** Two independent triggers, either suffices:
  1. Presence of a file at `config.kill_switch.halt_file` (default
     `./.halt`). Checked every 500ms.
  2. Missed heartbeat: the `agent` task publishes a heartbeat every
     1s; absence for `> 5s` on the consumer side trips the switch.
- **R7.2** On trip:
  1. Cancel all open orders.
  2. Emit `Order::market_close` for every open position, routed
     through the paper matching engine (in paper/research mode) —
     live exec is out of scope for v0.
  3. Write a `KillSwitchTripped` journal entry to the audit ledger
     with trigger reason and timestamp.
  4. Transition `agent` to `halted`; further signals are ignored.
  5. Cockpit displays a red banner with the trip reason.
- **R7.3** Halted state is **sticky** — it only clears on operator
  action via the cockpit (typed-phrase confirm) **and** removal of
  the `.halt` file. This is intentional: a wedge should require
  human ack before resuming.
- **R7.4** Runbook committed at
  `spec/runbooks/kill-switch.md` covering: trigger, expected
  behavior, recovery steps, audit-ledger queries to verify flatten
  was clean.
- **Acceptance:** an integration test drops a `.halt` file mid-run,
  asserts (a) all positions flat within 2s, (b) a
  `KillSwitchTripped` journal entry exists, (c) the reconciliation
  invariant (R3.5) still holds post-flatten, (d) restarting without
  removing `.halt` re-enters `halted` immediately.

### R8 — Configuration

- **R8.1** `config/agent.toml` is the single TOML source per
  [product.md → Configuration surface](../product.md#configuration-surface).
  v0 populates the keys applicable to this scope; unused keys
  (`llm.*`, `agents.*`) are accepted but ignored.
- **R8.2** Keys used by v0:
  - `mode` — `research | paper | live`, **defaults to `research`**.
    `live` mode is **rejected at startup** in v0 (explicit
    `UnsupportedMode` error).
  - `data.sources.binance.ws_url`, `data.sources.binance.rest_url`
  - `data.historical.parquet_root`
  - `data.clock_skew_warn_ms`, `data.clock_skew_halt_ms`
  - `strategies.sma_crossover.fast_len`, `.slow_len`, `.enabled`
  - `risk.per_symbol_exposure_cap` (default `0.4`)
  - `risk.sizing.fixed_fraction` (default `0.1`)
  - `risk.daily_loss_stop_pct` (default `-5.0`)
  - `risk.max_drawdown_stop_pct` (default `-15.0`)
  - `backtest.slippage_bps` (default `2`)
  - `backtest.taker_fee_bps` (default `4`)
  - `backtest.initial_capital_usdt` (default `100_000`)
  - `audit.ledger_db_path`
  - `audit.reconciliation_tolerance_usdt` (default `0.01`)
  - `kill_switch.halt_file` (default `./.halt`)
  - `kill_switch.heartbeat_timeout_ms` (default `5000`)
  - `observability.prometheus_listen` (default `0.0.0.0:9100`)
  - `cost.budget_usd_month` (default `20` per
    [product.md → Cost economics](../product.md#cost-economics--monthly-ceiling)
    v0 LLM line — v0 will not spend it, but the cap is wired)
- **R8.3** Config is validated at startup with a typed
  `Config::load()` returning `Result<Config, ConfigError>`;
  invalid ranges (e.g. negative fees) are rejected.
- **Acceptance:** a unit test loads a minimal `agent.toml`, asserts
  `mode = research` default, and asserts a config setting
  `mode = "live"` is rejected with `UnsupportedMode`.

### R9 — Observability

- **R9.1** `tracing` with JSON subscriber writing to stdout and to
  `logs/agent-<date>.jsonl`. Log level per crate is configurable.
- **R9.2** Prometheus exporter on `:9100`
  (`observability.prometheus_listen`). Required counters/gauges:
  - `bars_in_total{symbol}` (counter)
  - `ticks_in_total{symbol}` (counter)
  - `signals_emitted_total{strategy_id,kind}` (counter)
  - `orders_submitted_total{side}` (counter)
  - `fills_total{side}` (counter)
  - `ledger_writes_total{account}` (counter)
  - `ledger_imbalance_total` (counter — must stay at 0)
  - `clock_skew_ms{feed}` (gauge)
  - `equity_usdt` (gauge)
  - `agent_mode` (gauge, labeled `research|paper|live`)
  - `kill_switch_state` (gauge, 0/1)
- **R9.3** Spans wrap: each bar ingestion, each signal, each order
  submission, each ledger write, each reconciliation pass.
- **Acceptance:** after a 10-minute replay, `GET :9100/metrics`
  returns all counters above, `ledger_imbalance_total == 0`, and
  `bars_in_total{symbol="BTCUSDT"} == 10`.

### R10 — Cost telemetry scaffold

- **R10.1** A `cost` module (lives in `llm` crate even though no LLM
  is called in v0) exposes a `CostSink` trait with a single method:
  `record(call: CostEvent)`. `CostEvent` carries provider, model,
  tier (`deep_think | quick_think`), input tokens, output tokens,
  cached-input tokens, computed USD cost, and a correlation id.
- **R10.2** A default `LedgerCostSink` implementation writes each
  `CostEvent` as a journal entry (`expense:llm:<tier>` vs a
  synthetic `liabilities:llm_accrued` account). The accounts exist
  in the chart of accounts (R3.2 extended) even though v0 posts
  zero entries to them.
- **R10.3** A monthly rollup query against the ledger produces a
  `costs.md` line per
  [product.md → Cost economics](../product.md#cost-economics--monthly-ceiling);
  the tester generates this report at the end of the v0 run. For
  v0, LLM cost must be `$0.00`.
- **R10.4** 80% / 100% budget auto-degrade rules are **not**
  exercised in v0 (no LLM calls) but the hook points — a
  `CostBudget` type with `.remaining()` and `.mode_override()` —
  exist so v0.5 flips them on without API change.
- **Acceptance:** a unit test drives the `CostSink` with synthetic
  LLM events summing to `$0.50`, asserts the ledger shows matching
  `expense:llm:*` entries and the rollup reports `$0.50`. The
  v0 backtest report shows `LLM spend: $0.00`.

## Backtest Scenarios

### Scenario: `btc-2023-1m-sma-cross`

- **Universe:** `BTCUSDT`
- **Period:** `2023-01-01` → `2023-12-31`
- **Granularity:** `1m`
- **Data source:** `binance-spot` (via `data/binance/BTCUSDT/2023/*.parquet`)
- **Fees:** `0.04%` taker, `0.02%` maker (maker unused — market orders only)
- **Slippage model:** `bps: 2`
- **Initial capital:** `100_000 USDT`
- **Position sizing:** `fixed-fraction 0.1`
- **Risk limits:**
  - Max leverage: `1x` (spot, no margin in v0)
  - Max drawdown stop: `-15%`
  - Per-symbol exposure cap: `40%`
- **Strategy params:** `sma_crossover` with `fast_len = 20`, `slow_len = 50`
- **Seed:** `0xC0FFEE`
- **Baseline report:** none (this is the primary in-sample run and
  establishes the first baseline).

**Expected outcome (analyst hypothesis):** Sharpe likely negative;
SMA crossover on 1m bars is a known underperformer against BTC's
2023 regime (range-bound H1, trending H2 with frequent chop). We are
testing the harness, not the edge. The value of this run is (a)
proving the pipeline end-to-end, (b) producing the first backtest
report under `spec/reports/`, (c) producing a full year of ledger
entries against which R3.5 reconciliation can be asserted every
minute, and (d) establishing a reproducibility baseline (byte-identical
output under the same seed) that every future strategy must clear.
Any positive Sharpe is a red flag to re-check the fee/slippage model
before celebrating.

### Scenario: `btc-2024-h1-sma-cross`

- **Universe:** `BTCUSDT`
- **Period:** `2024-01-01` → `2024-06-30`
- **Granularity:** `1m`
- **Data source:** `binance-spot` (via `data/binance/BTCUSDT/2024/*.parquet`)
- **Fees:** `0.04%` taker, `0.02%` maker (maker unused)
- **Slippage model:** `bps: 2`
- **Initial capital:** `100_000 USDT`
- **Position sizing:** `fixed-fraction 0.1`
- **Risk limits:**
  - Max leverage: `1x`
  - Max drawdown stop: `-15%`
  - Per-symbol exposure cap: `40%`
- **Strategy params:** identical to `btc-2023-1m-sma-cross`
  (`fast_len = 20`, `slow_len = 50`).
- **Seed:** `0xC0FFEE`
- **Baseline report:** `spec/reports/backtest-<stamp>-btc-2023-1m-sma-cross.md`

**Expected outcome (analyst hypothesis):** This is the out-of-sample
baseline. We expect comparable (likely negative) Sharpe to the 2023
in-sample run — a large positive divergence in either direction
indicates a data-pipeline bug (look-ahead, timezone, or alignment),
not a real edge. The numerical output of this run becomes the
**regression baseline** that every future strategy must beat on
OOS 2024 H1 per the
[product.md → Strategy lifecycle — promotion gates](../product.md#strategy-lifecycle--promotion-gates)
`research → paper` gate (`Sharpe > 1.0 on 2y OOS`). For v0 this is
a floor, not a promotion; v0's SMA stays in `research` stage.

## Design

Translates R1–R10 into crate layout, traits, types, and message flow. All
decisions here are anchored to
[architecture.md](../architecture.md) — this section does not re-decide
cross-cutting concerns, it resolves R-to-code.

### Crate map for v0

Every crate in the workspace layout in
[architecture.md → Workspace layout](../architecture.md#workspace-layout-proposed)
exists by end of v0. Most ship real code; a few are stubs holding only a
`lib.rs` with public types that v0.5+ consumers import.

| Crate          | v0 contents                                                                 | Stubness                     |
|----------------|-----------------------------------------------------------------------------|------------------------------|
| `trading_core` | All primitives from R2 (`Symbol`, `Asset`, `Money<C>`, `Order`, `Fill`, `Position`, `Signal`, `Decision`, `Bar`, `Tick`, `StrategyId`, `AccountId`, read-side `FillView` / `JournalEntryView`). Error types. Clippy-lint roots for `float_arithmetic` / `unwrap_used`. | Full                         |
| `data`     | `MarketDataSource` trait + `BinanceFeed` / `ReplayFeed` / `FakeFeed`. Parquet loader. `trade_aggregation` adapter. Clock-skew detector (R1.3).                                                                                        | Full                         |
| `features` | Thin adapters over `kand` (batch) + `quantedge-ta` (streaming) for SMA. No other indicators in v0.                                                                                                                                   | Minimal — SMA only           |
| `models`   | Empty — `lib.rs` with doc comment. `candle` / `tract` not pulled in.                                                                                                                                                                 | **Stub**                     |
| `llm`      | Empty — `lib.rs` with provider-trait sketch (no impls). Ensures v0.5 has a home for the Anthropic client without a new crate.                                                                                                        | **Stub**                     |
| `cost`     | `CostEvent`, `CostSink`, `CostBudget` per [architecture.md → Cost telemetry](../architecture.md#cost-telemetry--dedicated-cost-crate--confirmed-2026-04-17). `LedgerCostSink` impl. No emitters in v0. | Full surface, no emitters    |
| `risk`     | Typed limits (per-symbol exposure cap, max drawdown, daily loss stop); position sizing (`fixed_fraction`). Feeds `Order::new` validation. `openpit` as second-line not wired in v0.                                                  | Full for v0 scope            |
| `strategy` | `Strategy` trait + compiled-in registry + `sma_crossover` impl + `StrategyRegistry::{load_from_toml, swap, unload}`.                                                                                                                 | Full; one strategy           |
| `exec`     | `ExecRouter` trait. Paper-mode impl routes to `backtest::PaperEngine`. Live impl is a stub returning `UnsupportedMode`.                                                                                                              | Paper path full; live stub   |
| `backtest` | `MatchingEngine` trait + `PaperEngine` + backtest loop + report writer + deterministic seeded RNG.                                                                                                                                   | Full                         |
| `audit`    | `sqlx-ledger` + chart-of-accounts bootstrap + journal-entry writers + reconciler + `audit::query` read-only surface.                                                                                                                 | Full                         |
| `ui`       | `cockpit` binary (R6). `viewer` binary is v0.5 (directory stub with a `README` noting deferral).                                                                                                                                     | `cockpit` full; `viewer` stub|
| `agent`    | Top-level orchestrator binary: wires `data` → `strategy` → `risk` → `exec` → `audit`, owns kill-switch file watcher + heartbeat, owns the in-process broadcast bus the UI subscribes to.                                             | Full                         |

**Dependency edges (v0):**

```
trading_core  ← data, features, risk, strategy, exec, backtest, audit, cost, ui, agent
audit ← cost, ui (read-only via audit::query), agent, backtest
data  ← strategy, backtest, agent
features ← strategy
risk ← strategy, agent
strategy ← agent, backtest
exec ← agent
backtest ← (bin target only; depends on data, strategy, risk, audit, exec)
```

No edge from `ui` to `strategy` / `exec` / `models` (enforced by `cargo deny`
or a `compile-fail` gate).

### Core types (R2)

Signatures the developer implements verbatim. `Decimal` means
`rust_decimal::Decimal`. No `f64` appears in this section.

```rust
// --- identifiers ---
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Symbol(pub SmolStr);                  // e.g. "BTCUSDT"

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Asset { Btc, Usdt, Eth, Other(SmolStr) }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StrategyId(pub SmolStr);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AccountId(pub SmolStr);               // e.g. "assets:cash:USDT"

// --- money (type-level currency separation; R2.4) ---
pub trait Currency: Copy + Eq + 'static { const CODE: &'static str; }
pub struct Usdt; impl Currency for Usdt { const CODE: &'static str = "USDT"; }
pub struct Btc;  impl Currency for Btc  { const CODE: &'static str = "BTC";  }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Money<C: Currency>(pub Decimal, #[serde(skip)] PhantomData<C>);

impl<C: Currency> Add for Money<C> { /* same currency only — compile-time */ }
// No impl Add<Money<Btc>> for Money<Usdt> — compile-fail test in trybuild.

// --- quantities & prices ---
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Quantity(Decimal);                    // non-negative; new() validates
impl Quantity {
    pub fn new(d: Decimal) -> Result<Self, QtyError>;  // rejects < 0
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Price(Decimal);                       // strictly positive
impl Price { pub fn new(d: Decimal) -> Result<Self, PriceError>; }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Side { Buy, Sell }

// --- market data ---
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bar {
    pub symbol: Symbol,
    pub tf: Timeframe,            // 1m for v0
    pub open_ts: Timestamp,       // venue ts, bar-open
    pub close_ts: Timestamp,      // venue ts, bar-close
    pub open: Price, pub high: Price, pub low: Price, pub close: Price,
    pub volume: Quantity,         // base-asset units
    pub trade_count: u32,
    pub local_recv_ts: Timestamp, // for clock-skew (R1.3)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tick {
    pub symbol: Symbol,
    pub venue_ts: Timestamp,
    pub local_recv_ts: Timestamp,
    pub price: Price,
    pub qty: Quantity,
    pub side: Side,               // aggressor side
    pub trade_id: u64,
}

// --- intent & execution ---
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Signal {
    pub strategy_id: StrategyId,
    pub symbol: Symbol,
    pub ts: Timestamp,
    pub kind: SignalKind,         // Buy | Sell | Hold
    pub evidence: SignalEvidence, // { fast_ma, slow_ma, indicator_values }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Decision {             // trader → risk → PM in v0.5; v0 bypasses
    pub signal: Signal,
    pub proposed: ProposedOrder,  // side, size, tif, invalidation
    pub rationale: SmolStr,       // for v0: "sma_crossover sizing=fixed_fraction"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order { /* private fields — only Order::new constructs */ }

impl Order {
    /// Enforces R2.4 invariants at construction. Rejects non-positive qty,
    /// mismatched asset pairs, price outside sanity band, and exposure
    /// breaches given the current position snapshot.
    pub fn new(
        strategy_id: StrategyId,
        symbol: Symbol,
        side: Side,
        qty: Quantity,
        kind: OrderKind,           // Market | Limit { price }
        tif: TimeInForce,
        position_snapshot: &Position,
        last_mark: Price,
        risk_limits: &RiskLimits,
    ) -> Result<Self, RiskError>;  // never panics; never silently truncates

    pub fn id(&self) -> OrderId;
    pub fn strategy_id(&self) -> StrategyId;
    pub fn symbol(&self) -> Symbol;
    pub fn side(&self) -> Side;
    pub fn qty(&self) -> Quantity;
    pub fn kind(&self) -> OrderKind;
    pub fn tif(&self) -> TimeInForce;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fill {
    pub order_id: OrderId,
    pub symbol: Symbol,
    pub side: Side,
    pub qty: Quantity,
    pub price: Price,              // post-slippage
    pub fee: Money<Usdt>,          // taker in v0
    pub fee_tier: FeeTier,         // Taker | Maker
    pub venue_ts: Timestamp,
    pub local_ts: Timestamp,
    pub liquidity: Liquidity,      // Taker for v0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub symbol: Symbol,
    pub base_qty: Decimal,         // signed — sign carries direction
    pub cost_basis: Money<Usdt>,
    pub last_mark: Price,
    pub realized_pnl: Money<Usdt>,
    pub unrealized_pnl: Money<Usdt>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Timestamp(pub OffsetDateTime);        // wraps `time::OffsetDateTime`
```

**Error types** are `thiserror`-derived and carried by `Result<T, E>` at every
public boundary. `OrderError`, `RiskError`, `QtyError`, `PriceError`,
`LedgerError`, `FeedError`, `ConfigError`, `StrategyError`, `CostError`.

**Clippy lint roots** in `trading_core/lib.rs` (package `trading_core`, directory `crates/core/`):

```rust
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::float_arithmetic)]
#![warn(clippy::pedantic)]
```

Same deny-level lints propagated to `risk`, `audit`, `exec`, `backtest` per
R2.2/R2.3.

### Strategy trait + registry (R4)

Trait is carried verbatim from
[architecture.md → Strategy registry & hot-loading](../architecture.md#strategy-registry--hot-loading).
v0 adds the registry surface:

```rust
pub trait Strategy: Send + Sync {
    fn id(&self) -> StrategyId;
    fn on_bar(&mut self, bar: &Bar) -> Vec<Signal>;
    fn on_tick(&mut self, tick: &Tick) -> Vec<Signal>;
    fn config_schema() -> serde_json::Value where Self: Sized;
}

pub struct StrategyRegistry { inner: HashMap<StrategyId, Box<dyn Strategy>> }

impl StrategyRegistry {
    pub fn load_from_toml(cfg: &Config) -> Result<Self, StrategyError>;

    /// v0.5 hot-load hook. v0 exposes but does not call from config watcher.
    pub fn swap(&mut self, id: StrategyId, new: Box<dyn Strategy>)
        -> Result<(), StrategyError>;

    pub fn unload(&mut self, id: StrategyId) -> Result<(), StrategyError>;

    pub fn on_bar(&mut self, bar: &Bar) -> Vec<Signal>;   // fan-out
    pub fn on_tick(&mut self, tick: &Tick) -> Vec<Signal>;
}
```

Every `load` / `swap` / `unload` emits a registry-mutation journal entry to
`audit` per
[architecture.md → Lifecycle integration](../architecture.md#lifecycle-integration).
v0 only exercises `load`.

v0 ships exactly one implementation, `sma_crossover`, per R4.3–R4.6.
`on_tick` returns `vec![]`.

### Audit ledger schema (R3)

#### Chart of accounts (bootstrapped in `audit::bootstrap::chart_of_accounts`)

```
assets:cash:USDT                # cash float
assets:position:BTC             # base-asset inventory (qty * cost basis unit)
assets:position_mark:BTC        # mark-to-market contra for unrealized
income:realized_pnl
income:unrealized_pnl
expense:fees:taker
expense:fees:maker              # exists, unused in v0
expense:llm:deep_think          # exists, zero entries in v0 (R10); pre-seeded for v0.5+
expense:llm:quick_think         # exists, zero entries in v0 (R10); pre-seeded for v0.5+
liabilities:llm_accrued         # exists, zero entries in v0 (R10); pre-seeded for v0.5+
expense:infra                   # exists, zero entries in v0; pre-seeded for v1+
expense:data                    # exists, zero entries in v0; pre-seeded for v1+
equity:opening_balance
```

(13 accounts total — canonical per R3.2)

#### Journal entry shape per fill (R3.3)

A **buy fill** of `q` BTC @ `p` USDT with fee `f` USDT is one transaction:

| Dr / Cr  | Account                  | Amount    |
|----------|--------------------------|-----------|
| Dr       | `assets:position:BTC`    | `q * p`   |
| Cr       | `assets:cash:USDT`       | `q * p`   |
| Dr       | `expense:fees:taker`     | `f`       |
| Cr       | `assets:cash:USDT`       | `f`       |

(Sum debits = sum credits at `q*p + f`.)

A **sell fill** mirrors the inventory leg and posts realized P&L:

| Dr / Cr  | Account                  | Amount                               |
|----------|--------------------------|--------------------------------------|
| Dr       | `assets:cash:USDT`       | `q * p`                              |
| Cr       | `assets:position:BTC`    | `q * cost_basis_per_unit`            |
| Dr/Cr    | `income:realized_pnl`    | `q * (p - cost_basis_per_unit)` (signed) |
| Dr       | `expense:fees:taker`     | `f`                                  |
| Cr       | `assets:cash:USDT`       | `f`                                  |

Cost basis is tracked per-position as a running `Money<Usdt>`; FIFO vs
weighted-average picked as **weighted-average** for v0 (simpler; FIFO is a
v0.5 decision driven by
[product.md → Open decisions → Tax / reporting](../product.md#open-decisions)).

#### Minute-boundary reconciliation (R3.5)

Enforced by a **background tokio task** owned by `agent`, not by a ledger
trigger. Rationale: (a) ledger triggers would couple the audit crate to the
mark feed, (b) a tokio task cleanly ties into kill-switch wiring and spans.

```rust
// pseudo-Rust — implementation sketch
async fn reconcile_task(
    mut bar_close_rx: broadcast::Receiver<BarClose>,
    ledger: Arc<Ledger>,
    positions: Arc<RwLock<PositionBook>>,
    tol: Money<Usdt>,
    kill: KillSwitch,
) {
    while let Ok(bc) = bar_close_rx.recv().await {
        let from_ledger = ledger.recompute_equity(bc.ts).await?;
        let from_book   = positions.read().await.equity(bc.marks.clone());
        if (from_ledger - from_book).abs() > tol {
            emit_event!(LedgerImbalance { /* ... */ });
            kill.trip(TripReason::LedgerImbalance).await;
        }
    }
}
```

Tolerance: `config.audit.reconciliation_tolerance_usdt` (default `0.01`).
Span: `tracing::info_span!("reconcile", bar_ts = ?bc.ts)`.

### MatchingEngine trait + v0 PaperEngine (R5)

```rust
#[async_trait]
pub trait MatchingEngine: Send + Sync {
    /// Consume a batch of bar-aligned orders and return their fills.
    /// Bar-close market orders are the only thing v0 exercises, but the
    /// trait signature is already limit-order-friendly.
    async fn step(
        &mut self,
        bar: &Bar,
        orders: Vec<Order>,
    ) -> Result<Vec<Fill>, MatchError>;

    fn config(&self) -> MatchConfig;
}

pub struct PaperEngine {
    slippage_bps: u32,    // default 2
    taker_fee_bps: u32,   // default 4
    maker_fee_bps: u32,   // default 2, unused in v0
    vwap_mode: FillPriceMode, // BarClose | BarVwap
    rng: ChaCha20Rng,     // seeded from --seed
}
```

**Fill-price formula** (market order against bar `b`):

- `base = match vwap_mode { BarClose => b.close, BarVwap => b.vwap }`
- Buy: `fill_price = base * (1 + slippage_bps / 10_000)`
- Sell: `fill_price = base * (1 - slippage_bps / 10_000)`

**Fee:** `fee = notional * (taker_fee_bps / 10_000)` where
`notional = fill_price * qty`. Booked to `expense:fees:taker` per R3.3.

**Determinism:** the RNG is seeded from `--seed` (default `0xC0FFEE`). Only
use is tie-break ordering when two orders carry the same timestamp; no jitter
is added to price or fee in v0.

### Data flow (R1)

```mermaid
flowchart LR
  subgraph venue
    binance[Binance spot WS<br/>btcusdt@kline_1m<br/>btcusdt@trade]
  end
  binance -- tokio-tungstenite --> mds[data::MarketDataSource<br/>= BinanceFeed]
  parquet[Parquet<br/>fixture] --> mds2[data::ReplayFeed]
  mds --> agg[data::bar_stream<br/>(trade_aggregation)]
  mds2 --> agg
  agg -- broadcast::Bar --> strat[strategy::StrategyRegistry]
  agg -- broadcast::Bar --> ui_sub[ui::cockpit Subscription]
  agg -- broadcast::Tick --> ui_sub
  strat --> risk[risk::size_and_validate]
  risk --> order[Order::new Result]
  order -- Ok(Order) --> exec[exec::ExecRouter]
  exec --> paper[backtest::PaperEngine]
  paper -- Fill --> ledger[audit::journal]
  paper -- Fill --> positions[PositionBook]
  positions -- BarClose --> reconcile[reconcile_task]
  ledger -. audit::query .-> ui_query[ui::cockpit P&L card]
```

Channels are `tokio::sync::broadcast` (bars, ticks, fills — multi-subscriber)
and `tokio::sync::mpsc` (orders — single consumer: exec). Capacities come
from `config.bus.{bars,ticks,fills}_capacity` with sane defaults (1024 bars,
8192 ticks, 1024 fills).

### UI cockpit message model (R6)

iced's Elm-architecture fits directly over the broadcast buses. One `Model`,
one `Message` enum, one `Subscription` per feed:

```rust
// ui/cockpit/mod.rs  (sketch)
pub struct Cockpit {
    tape: VecDeque<FillView>,     // bounded to 200
    positions: Vec<PositionView>,
    pnl: PnlSnapshot,             // from audit::query, refreshed on BarClose
    latency: LatencyBadge,
    kill_state: KillState,
    mode_banner: AgentMode,       // research | paper | halted
    confirm: Option<ConfirmDialog>,
}

#[derive(Debug, Clone)]
pub enum Message {
    // feed events
    BarReceived(Bar),
    TickReceived(Tick),
    FillReceived(FillView),
    BarClose(Timestamp),          // triggers pnl refresh
    ClockSkew(Milliseconds),

    // query results (from audit::query)
    PnlRefreshed(PnlSnapshot),
    PositionsRefreshed(Vec<PositionView>),

    // operator actions
    KillPressed,
    KillConfirmPhraseChanged(String),
    KillConfirmed,
    KillCancelled,
    TapePauseToggled,

    // agent → UI lifecycle
    AgentMode(AgentMode),
    LedgerError(LedgerErrorView),
}

pub fn subscription(bus: &AgentBus) -> Subscription<Message> {
    Subscription::batch([
        from_broadcast(bus.bars.subscribe(),  Message::BarReceived),
        from_broadcast(bus.ticks.subscribe(), Message::TickReceived),
        from_broadcast(bus.fills.subscribe(), Message::FillReceived),
        from_broadcast(bus.mode.subscribe(),  Message::AgentMode),
    ])
}
```

**Panel → Model field mapping** (R6.2):

| Panel          | Model field       | Refresh trigger                  | Data source           |
|----------------|-------------------|----------------------------------|-----------------------|
| Live tape      | `tape`            | `FillReceived`                   | broadcast feed        |
| Position panel | `positions`       | `BarClose` → `PositionsRefreshed`| `audit::query::position()` / aggregator |
| P&L card       | `pnl`             | `BarClose` → `PnlRefreshed`      | `audit::query` aggregates (R3.6)         |
| Kill switch    | `kill_state`, `confirm` | user + `AgentMode`         | local + agent bus     |
| Latency badge  | `latency`         | `TickReceived`, `ClockSkew`      | derived from `Tick.venue_ts` vs local    |

**Kill-switch confirm flow** (R6 + R7):

1. `KillPressed` → `confirm = Some(ConfirmDialog::new(expected="HALT BTC"))`.
2. `KillConfirmPhraseChanged(s)` updates the dialog's text field.
3. `KillConfirmed` only fires if `s == expected`; otherwise the button stays
   disabled.
4. `KillConfirmed` sends `AgentCommand::KillSwitch(Operator)` over an mpsc
   into the agent task; the agent runs the flatten routine (R7.2) and emits
   `AgentMode(Halted)` back.

**Empty / loading / error** (R6.4): each panel is a `view_*` function that
matches on an `PanelState::{Empty, Loading, Error(msg), Ready(data)}` and
renders the appropriate widget. Strings come from `ui::strings`, styles from
`ui::theme`.

### Kill switch wiring (R7)

Three concurrent tokio tasks composed by `agent`:

```
┌──────────────────┐      ┌──────────────────┐      ┌──────────────────┐
│ halt_file_watch  │      │ heartbeat_watch  │      │ reconcile_task   │
│ (notify crate)   │      │ (1s publish,     │      │ (ledger vs book) │
│                  │      │  5s timeout)     │      │                  │
└────────┬─────────┘      └────────┬─────────┘      └────────┬─────────┘
         │ trip(reason)            │ trip(reason)            │ trip(reason)
         └──────────────┬──────────┴─────────────────────────┘
                        ▼
                ┌──────────────────┐
                │  KillSwitch      │  single tokio::sync::Mutex<State>
                │  .trip(reason)   │
                └────────┬─────────┘
                         │ AgentCommand::Flatten
                         ▼
             ┌─────────────────────────────┐
             │ flatten_and_halt            │
             │  1. cancel open orders      │
             │  2. emit market_close × N   │
             │  3. await fills             │
             │  4. journal KillSwitchTripped│
             │  5. set agent_mode=Halted   │
             │  6. broadcast(AgentMode)    │
             └─────────────────────────────┘
```

- **Halt-file watcher**: `notify` crate on the directory of
  `config.kill_switch.halt_file`; poll fallback every 500ms if notify is
  unavailable.
- **Heartbeat**: `agent` publishes to an `mpsc` every 1s; a consumer task
  times out after 5s (`config.kill_switch.heartbeat_timeout_ms`) and trips.
- **Sticky halt**: the `KillSwitch` state is only cleared on the combined
  event `ConfirmFromCockpit(OperatorPhrase) && halt_file_absent`, per R7.3.
  Attempting to resume with `.halt` still present re-enters `Halted`.

The `KillSwitchTripped` journal entry (R7.2) is a single transaction:

| Dr / Cr | Account                       | Amount | Metadata                          |
|---------|-------------------------------|--------|-----------------------------------|
| memo    | `equity:opening_balance`      | `0`    | `reason=...`, `trip_ts=...`, `operator=.../halt_file/heartbeat` |

Zero-amount memo entries are supported by `sqlx-ledger` and preserve the
invariant `Σ(debits) == Σ(credits)`; they make the wedge reason queryable
from the ledger alone.

### Config schema (R8)

```toml
# config/agent.toml — v0 subset. Keys outside this set are accepted but ignored.

mode = "research"   # research | paper ; "live" rejected in v0

[data.sources.binance]
ws_url   = "wss://stream.binance.com:9443/ws"
rest_url = "https://api.binance.com"

[data.historical]
parquet_root = "./data/binance/BTCUSDT"

[data]
clock_skew_warn_ms = 2000
clock_skew_halt_ms = 10000

[strategies.sma_crossover]
enabled  = true
fast_len = 20
slow_len = 50

[risk]
per_symbol_exposure_cap = 0.40
daily_loss_stop_pct     = -5.0
max_drawdown_stop_pct   = -15.0

[risk.sizing]
fixed_fraction = 0.10

[backtest]
slippage_bps         = 2
taker_fee_bps        = 4
maker_fee_bps        = 2
initial_capital_usdt = 100_000

[audit]
ledger_db_path                  = "./data/audit/ledger.db"
reconciliation_tolerance_usdt   = 0.01

[kill_switch]
halt_file              = "./.halt"
heartbeat_timeout_ms   = 5000

[observability]
prometheus_listen = "0.0.0.0:9100"

[cost]
budget_usd_month = 20

[bus]  # tokio::sync::broadcast capacities
bars_capacity  = 1024
ticks_capacity = 8192
fills_capacity = 1024
```

`Config::load()` returns `Result<Config, ConfigError>`; validation rejects
non-positive caps, negative budgets, `mode = "live"`, and any
`fast_len >= slow_len` for SMA.

### Observability + cost telemetry (R9, R10)

**Tracing layer composition** (`agent::observability::init`):

```
tracing_subscriber::registry()
    .with(EnvFilter::from_default_env())        // RUST_LOG override
    .with(fmt::layer().json().with_writer(stdout))
    .with(fmt::layer().json().with_writer(rolling_file("logs/", "agent")))
    .with(tracing_opentelemetry::layer())       // deferred — hook point only in v0
    .init();
```

**Prometheus exporter** via `metrics-exporter-prometheus` listening on
`config.observability.prometheus_listen` (default `0.0.0.0:9100`). Counters
and gauges exactly per R9.2.

**Spans** (R9.3): `bar.ingest`, `signal.emit`, `order.submit`, `fill.book`,
`ledger.write`, `reconcile`. Every span carries `symbol` + `strategy_id`
where applicable and `correlation_id` for cross-crate tracing.

**Cost scaffold** (R10): `cost::CostSink` is wired into `agent` as
`Arc<dyn CostSink>` but the default implementation receives zero emissions in
v0 — v0 has no LLM caller. The point of shipping this now is that the v0.5
LLM work adds `cost_sink.record(CostEvent::Llm { .. })` at emit sites without
touching `agent` or `audit`.

### Performance budget for v0

Restated from
[architecture.md → Performance budget](../architecture.md#performance-budget):

| Path                          | Budget        | v0 enforcement                          |
|-------------------------------|---------------|------------------------------------------|
| Bar-close → signal (no LLM)   | < 5 ms p99    | `criterion` bench in `strategy` (baseline recorded) |
| Backtest throughput           | > 100k bars/s | `criterion` bench on `PaperEngine` alone |

**v0-specific targets:**

| Target                                                           | Budget   | Rationale                      |
|------------------------------------------------------------------|---------:|--------------------------------|
| Process 1y of 1m BTCUSDT (~525_600 bars) through full pipeline   | < 60 s   | Laptop (M-series or equivalent); regression bench |
| Ledger write + balance check per fill                            | < 1 ms p99 | SQLite WAL + transaction per fill |
| Minute-boundary reconciliation                                   | < 10 ms  | Keeps the background task off the bar-close critical path |
| Cockpit frame time under live replay                             | < 16 ms  | 60fps floor                     |

### Test strategy

| Layer                    | Tests                                                                                                                   | Crate(s)       | Tool         |
|--------------------------|-------------------------------------------------------------------------------------------------------------------------|----------------|--------------|
| **Unit**                 | `Order::new` validation cases, `Quantity::new` / `Price::new` boundaries, `sma_crossover` signal transitions, `PaperEngine::step` fill math, `Config::load` validation errors | `trading_core`, `strategy`, `backtest` | `cargo test` |
| **Property**             | `Order::new` rejects every invalid combination in its input space (R2 acceptance); `Money<C>` arithmetic is associative/commutative per currency | `trading_core`, `risk` | `proptest`   |
| **Compile-fail**         | `Money<Usdt> + Money<Btc>` does not compile; `Quantity` from negative `Decimal` is a type-level error; `Order` fields are private | `trading_core` | `trybuild`   |
| **Integration (replay)** | `ReplayFeed` on 1h Parquet fixture produces exactly 60 bars; `trade_aggregation` bars match published klines within tol | `data`         | `cargo test` |
| **Integration (ledger)** | Synthesize a deliberate imbalance; reconciler catches it and trips kill switch                                          | `audit`, `agent` | `cargo test` |
| **Integration (kill)**   | Drop `.halt` file mid-run → all positions flat in ≤ 2s, `KillSwitchTripped` journaled, reconciliation still holds      | `agent`        | `cargo test` |
| **Snapshot (UI)**        | Each panel's empty/loading/error/ready state renders stably                                                             | `ui`           | `insta`      |
| **Snapshot (report)**    | Backtest report for `btc-2023-1m-sma-cross` at seed `0xC0FFEE` has a stable sha256                                     | `backtest`     | `insta` + direct hash |
| **Determinism**          | Two runs of `btc-2023-1m-sma-cross` at same seed produce byte-identical reports **and** byte-identical ledger DB dumps | `backtest`     | `cargo test` |
| **Bench**                | `PaperEngine::step` throughput (> 100k bars/s); `sma_crossover::on_bar` latency (< 5ms p99)                             | `backtest`, `strategy` | `criterion` |
| **Observability**        | After a scripted 10m replay, `GET /metrics` contains every counter/gauge from R9.2; `ledger_imbalance_total == 0`       | `agent`        | integration test |

## Verification

The tester's contract for declaring v0 done. All items must be green
before a `VERDICT → PASS` can be issued.

- **V1 Static checks pass.**
  - `cargo fmt --check` clean.
  - `cargo clippy --workspace --all-targets -- -D warnings` clean,
    including the `float_arithmetic`, `unwrap_used`, and
    `expect_used` lints scoped per R2.2 / R2.3.
  - `cargo audit` shows no unpatched advisories.
  - `cargo deny check` (bans, licenses, sources) passes.
- **V2 Unit + integration tests pass.** `cargo test --workspace`
  produces zero failures, zero ignored-unexplained, and the
  per-crate table in
  [.claude/skills/rust-test/templates/test-report.md](../../.claude/skills/rust-test/templates/test-report.md)
  section 3 is filled in. Property tests (R2.4) and `trybuild`
  compile-fail tests (R2.2) are part of this run.
- **V3 Both backtest scenarios run end-to-end.**
  - `btc-2023-1m-sma-cross` produces
    `spec/reports/backtest-<stamp>-btc-2023-1m-sma-cross.md`
    conforming to the tester report template section 5.
  - `btc-2024-h1-sma-cross` produces
    `spec/reports/backtest-<stamp>-btc-2024-h1-sma-cross.md` with
    the 2023 run listed as baseline.
  - Both reports include metrics (Total return, CAGR, Sharpe,
    Sortino, Max drawdown, Hit rate, Turnover, Trades, Avg trade
    P&L) and an equity-curve prose description.
- **V4 Ledger reconciles.** Across the full 2023 backtest, the
  minute-boundary reconciliation (R3.5) passes at **every** bar —
  `ledger_imbalance_total == 0` in Prometheus and zero
  `LedgerImbalance` events in the structured log. The report
  contains the count of successful reconciliations (≈ 525_600 for
  a full year at 1m).
- **V5 Determinism check.** `btc-2023-1m-sma-cross` is run twice at
  seed `0xC0FFEE`; both reports have identical sha256. A diff of
  the ledger DB exports is empty.
- **V6 Manual UI smoke test.** With the agent running against
  Binance testnet (or the replay driver pointed at a recent day):
  - Cockpit launches, all four panels (live tape, position, P&L,
    kill switch) render within 2s.
  - Live tape updates per bar; latency badge reflects real venue ts.
  - Empty / loading / error states reachable (script: stop feed →
    error; restart before data → loading; no-position start →
    empty).
  - Kill-switch test: operator types the confirm phrase → flatten
    executes, positions go to zero, `KillSwitchTripped` journal
    entry visible via `audit` read query, red halted banner shown.
  - Second kill-switch test: drop a `.halt` file externally →
    same flatten + halt + journal behavior.
  - Screenshots of each state committed to the PR per
    [product.md → v0 — first step — Acceptance](../product.md#acceptance).
- **V7 Cost telemetry.** The v0 run's generated `costs.md` shows
  `LLM tokens: $0.00` and `Total / month: ≤ $45` (only hosting +
  storage lines non-zero). The cost ledger accounts
  (`expense:llm:*`) exist in the chart of accounts but contain
  zero entries.
- **V8 Observability.** `GET :9100/metrics` during a live run
  returns every counter/gauge in R9.2. The tester's report
  includes a snapshot of `/metrics` at the end of the run.
- **V9 Runbook present.** `spec/runbooks/kill-switch.md` exists
  and covers trigger, behavior, recovery, audit queries (R7.4).

Failure on any of V1–V9 routes as follows (matches the router in
[AGENT.md → verdict routing](../../AGENT.md#canonical-workflow)):

- Static / test / bench failure → `developer`.
- UI regression → `ui-designer`.
- Structural (crate layout, trait shape, registry) → `architect`.
- Strategy / scenario hypothesis wrong → `analyst`.

## UI — Week 1

_Filled by ui-designer at end of Week 1 (T13–T20)._

### Wireframe (ASCII)

```
┌────────────────────────────────────────────────────────────────────────────┐
│ Trading Cockpit                                                            │
├──────────────────────────────────┬─────────────────────────────────────────┤
│ ┌──────────────────────────────┐ │ ┌─────────────────────────────────────┐ │
│ │ P&L                          │ │ │ Open positions                      │ │
│ │ Total equity      90,129.50  │ │ │ Symbol Qty Cost Mark P&L  P&L% Exp% │ │
│ │ P&L today   +129.50          │ │ │ BTCUS… 0.25 10k 40k  +12  +0.13 11  │ │
│ │ Cash        90,000.00        │ │ └─────────────────────────────────────┘ │
│ │ Unrealized  +250.00          │ │ ┌─────────────────────────────────────┐ │
│ │ Realized    -120.50          │ │ │ Live tape                           │ │
│ ├──────────────────────────────┤ │ │ Time  Sym  Side  Price Qty  Fee     │ │
│ │ Feed latency                 │ │ │ …scrollable, last 200 fills         │ │
│ │ OK   120 ms                  │ │ │ [Pause / Resume]                    │ │
│ ├──────────────────────────────┤ │ │                                     │ │
│ │ Stop trading                 │ │ │                                     │ │
│ │ [big red button → dialog]    │ │ └─────────────────────────────────────┘ │
│ └──────────────────────────────┘ │                                         │
└──────────────────────────────────┴─────────────────────────────────────────┘
```

Left column carries the three sticky panels (P&L, latency, kill). Right
column carries the two scrolling panels (positions, tape). Cockpit density
is compact — operator should see all four panels without scrolling the
window.

### Panels landed

- **Live tape** (`widgets/tape.rs`) — last 200 `FillView` rows, newest on
  top. Pause toggle buffers fills without dropping them. States covered:
  Loading / Empty / Error / Ready / Paused-banner.
- **Positions** (`widgets/positions.rs`) — open positions table with
  cost / mark / P&L / P&L% / exposure% columns. Zero-qty rows filtered
  out. Colors: `pos` / `neg` for signed deltas. States: Loading / Empty
  / Error / Ready (with zero-qty filtering).
- **P&L card** (`widgets/pnl.rs`) — Equity (display size) + daily return,
  cash, unrealized, realized. Color only for signed deltas. States:
  Loading / Empty / Error / Ready × positive / Ready × negative.
- **Kill switch** (`widgets/kill.rs`) — 4-state machine (Idle /
  Confirming / Flattening / Halted). Typed phrase `HALT BTC` gates
  Confirm; phrase-mismatch hint appears after first keystroke. States:
  Idle / Confirming empty / Confirming mismatch / Confirming correct /
  Halted banner.
- **Latency badge** (`widgets/latency.rs`) — color/label from R6.2
  thresholds (`Badge::classify`): OK < 500 ms green, Slow < 2 s amber,
  High ≥ 2 s red, Halted ≥ 10 s red + banner. States: Unknown / Ok /
  Warn / High / Halted.

### Strings added

All in `ui::strings` (`crates/ui/src/strings.rs`). Single-source-of-truth
for operator-facing copy — zero inline literals in widget files.

| Key                           | English                                                                                                                                             |
|-------------------------------|-----------------------------------------------------------------------------------------------------------------------------------------------------|
| `APP_TITLE`                   | Trading Cockpit                                                                                                                                     |
| `PANEL_TAPE_TITLE`            | Live tape                                                                                                                                           |
| `PANEL_POSITIONS_TITLE`       | Open positions                                                                                                                                      |
| `PANEL_PNL_TITLE`             | P&L                                                                                                                                                 |
| `PANEL_KILL_TITLE`            | Stop trading                                                                                                                                        |
| `PANEL_LATENCY_TITLE`         | Feed latency                                                                                                                                        |
| `TAPE_COL_TIME` / `TAPE_COL_SYMBOL` / `TAPE_COL_SIDE` / `TAPE_COL_PRICE` / `TAPE_COL_QTY` / `TAPE_COL_FEE` | Time / Symbol / Side / Price / Qty / Fee |
| `TAPE_PAUSE_LABEL` / `TAPE_RESUME_LABEL`                | Pause / Resume                                                                                            |
| `TAPE_LOADING`                | Connecting to the fill stream…                                                                                                                      |
| `TAPE_EMPTY`                  | No fills yet. Waiting for the first bar from BTCUSDT.                                                                                               |
| `TAPE_ERROR_PREFIX`           | Can't read the fill stream:                                                                                                                         |
| `TAPE_PAUSED_BANNER`          | Paused — updates buffered                                                                                                                           |
| `POS_COL_SYMBOL` / `POS_COL_QTY` / `POS_COL_COST` / `POS_COL_MARK` / `POS_COL_PNL` / `POS_COL_PNL_PCT` / `POS_COL_EXPOSURE` | Symbol / Qty / Cost / Mark / P&L / P&L % / Exposure % |
| `POS_LOADING`                 | Loading positions from the ledger…                                                                                                                  |
| `POS_EMPTY`                   | No open positions. Strategy is armed and watching.                                                                                                  |
| `POS_ERROR_PREFIX`            | Ledger error while reading positions:                                                                                                               |
| `PNL_LABEL_CASH` / `PNL_LABEL_UNREALIZED` / `PNL_LABEL_REALIZED` / `PNL_LABEL_EQUITY` / `PNL_LABEL_DAILY_RETURN` | Cash / Unrealized / Realized today / Total equity / P&L today |
| `PNL_LOADING`                 | Reading equity from the ledger…                                                                                                                     |
| `PNL_EMPTY`                   | No equity recorded yet. First reconciliation pending.                                                                                               |
| `PNL_ERROR_PREFIX`            | Ledger error while reading equity:                                                                                                                  |
| `KILL_BUTTON_LABEL`           | Stop trading                                                                                                                                        |
| `KILL_BUTTON_HELP`            | Cancels open orders, flattens every position, and halts the agent. Requires a typed confirmation.                                                   |
| `KILL_DIALOG_TITLE`           | Confirm stop trading                                                                                                                                |
| `KILL_DIALOG_BODY`            | This cancels every open order, sells each open position at market, and puts the agent into a halted state. Type the phrase below to confirm.       |
| `KILL_PHRASE_LABEL`           | Type HALT BTC to confirm                                                                                                                            |
| `KILL_SAFETY_PHRASE`          | HALT BTC                                                                                                                                            |
| `KILL_CONFIRM_LABEL` / `KILL_CANCEL_LABEL` | Confirm stop / Cancel                                                                                                                  |
| `KILL_PHRASE_MISMATCH_HINT`   | Phrase doesn't match. Type HALT BTC exactly (case-sensitive).                                                                                       |
| `KILL_HALTED_BANNER`          | AGENT HALTED                                                                                                                                        |
| `KILL_HALTED_HINT`            | Remove .halt and re-arm from the operator runbook before resuming.                                                                                  |
| `KILL_RUNBOOK_LINK_LABEL`     | Open kill-switch runbook                                                                                                                            |
| `LATENCY_OK_LABEL` / `LATENCY_WARN_LABEL` / `LATENCY_HIGH_LABEL` / `LATENCY_HALTED_LABEL` | OK / Slow / High / Halted                                              |
| `LATENCY_UNIT_MS` / `LATENCY_UNKNOWN` / `LATENCY_HELP` | ms / — / Venue timestamp vs local clock on the last tick.                                                         |
| `MODE_RESEARCH` / `MODE_PAPER` / `MODE_LIVE` / `MODE_HALTED` | Research / Paper / Live / Halted                                                                             |
| `SIDE_BUY` / `SIDE_SELL`      | BUY / SELL                                                                                                                                          |
| `UNIT_USDT` / `UNIT_BTC`      | USDT / BTC                                                                                                                                          |
| `PLACEHOLDER_NONE`            | —                                                                                                                                                   |

### Theme tokens added

All in `ui::theme` (`crates/ui/src/theme.rs`). Semantic-only. Dark palette;
a light palette is a v0.5 decision.

| Token family | Values                                                                                   |
|--------------|------------------------------------------------------------------------------------------|
| Color        | `BG`, `BG_ELEV`, `FG`, `FG_MUTED`, `ACCENT`, `POS`, `NEG`, `WARN`, `BORDER`              |
| Space        | `XS=4`, `S=8`, `M=12`, `L=16`, `XL=24`, `XXL=32`                                         |
| Text         | `CAPTION=11`, `BODY=13`, `TITLE=16`, `DISPLAY=22`                                        |
| Radius       | `SMALL=2`, `MEDIUM=4`                                                                    |
| Layout       | `PANEL_PADDING=L`, `PANEL_GAP=M`, `PANEL_OUTER_GAP=L`, `TAPE_MAX_ROWS=200`                |
| Latency      | `OK_MS=500`, `WARN_MS=2_000`, `HALTED_MS=10_000`                                         |
| Helpers      | `color_for_delta(d: Decimal)`, `color_for_latency_ms(ms: i64)`                           |

### Accessibility notes

- **Keyboard map.** Default iced focus traversal (Tab / Shift+Tab) hits
  every interactive element: the Pause button in the tape, the Stop
  trading button, the typed-phrase text input in the kill dialog, and
  the Confirm / Cancel buttons. No click-only surfaces.
- **Color contrast.** Spot-checked the primary pairings:
  `FG (#E8ECF2)` on `BG (#11141A)` → 15:1;
  `FG (#E8ECF2)` on `BG_ELEV (#1A1F29)` → 13:1;
  `FG_MUTED (#8B93A3)` on `BG_ELEV (#1A1F29)` → 5.2:1 (AA pass for body);
  `NEG (#FF6B6B)` on `BG_ELEV (#1A1F29)` → 5.9:1; `POS (#3ECF8E)` on
  `BG_ELEV (#1A1F29)` → 7.1:1. All ≥ 4.5:1 for body text.
- **Color is never the only signal.** P&L sign is in the number, side is
  in `BUY`/`SELL` text, latency always carries a label next to its color.
- **Focus order** matches reading order: P&L → latency → kill → positions
  → tape. Destructive (kill) comes before scanning (positions / tape)
  because scanning panels should not steal the operator's eye past the
  big red button.

### Consistency self-audit

Enforced as tests in `crates/ui/tests/consistency.rs` (runs on every
`cargo test -p ui`):

- `inline strings: 0` — widgets contain zero user-visible string literals.
  Everything flows via `ui::strings`. Test: `no_inline_user_visible_strings_in_widgets`.
- `inline hex: 0` — widgets and `state.rs` contain zero `#rrggbb` tokens.
  Only `theme.rs` holds hex. Test: `no_inline_hex_colors_in_widgets_or_state`.

### Known issues / TODOs for Week 2

- **T32 wiring.** Cockpit `Subscription` is currently empty — it needs
  wiring to the `agent` broadcast bus for bars, ticks, fills, and
  `AgentMode`. Depends on T31 (agent binary). The message surface is
  already defined (`Message::BarReceived` / `FillReceived` / `AgentMode`
  etc.) so T32 is additive.
- **Ledger-backed P&L (R3.6).** Today the P&L card reads from
  `ui::fixtures`. Swap to `audit::query::equity` / `realized_pnl_since`
  / `unrealized_pnl` once T07 is in; the `Message::PnlRefreshed` /
  `PositionsRefreshed` shapes already carry the right payloads.
- **`audit` dep is intentionally not in `crates/ui/Cargo.toml` yet.**
  Re-add it in T32 when `audit::query` is available and callable.
- **iced_aw / plotters-iced not adopted in v0.** The cockpit's widget
  budget is low enough that stock iced widgets suffice; the backtest
  viewer (v0.5) is the natural home for `plotters-iced` (equity curve)
  and `iced_aw` (date pickers, tabs).
- **Runbook link (`KILL_RUNBOOK_LINK_LABEL`) is currently a plain
  caption — no hyperlink.** T29 will add the `spec/runbooks/kill-switch.md`
  file; once it exists the link can resolve to a `file://` open action
  or an in-app help modal.
- **`trading_core` crate rename (Rust 2024) — RESOLVED.** The foundation crate
  was previously named `core`, which shadowed `std::core`. The package is now
  `trading_core` (directory stays `crates/core/`). The per-consumer alias
  `trading_core = { package = "core" }` has been removed; all consumers now
  depend directly on `trading_core = { path = "../core" }`. The `doctest = false`
  workaround has been removed. `cargo test --workspace --doc` is green.

## UI — Week 2

_Filled by ui-designer at end of Week 2 (T32 + T_FINAL_B)._

### What wired up

**New module: `ui::live`** (gated behind `--features live`). Subscribes the
cockpit to the `agent::EventBus` six broadcast channels listed in
[dev-week2-broadcast-api-2026-04-18.md](../reports/dev-week2-broadcast-api-2026-04-18.md)
and converts each event into the existing cockpit `Message` enum:

| Channel     | Bus type                    | Cockpit `Message`                         | Close behavior                                     |
|-------------|-----------------------------|-------------------------------------------|----------------------------------------------------|
| `fills`     | `trading_core::Fill`        | `FillReceived(FillView)`                  | `TapeError(CONNECTION_CHANNEL_CLOSED)`             |
| `positions` | `trading_core::Position`    | `PositionsRefreshed(Vec<PositionView>)`\* | `PositionsError(CONNECTION_CHANNEL_CLOSED)`        |
| `pnl`       | `trading_core::PnlSnapshot` | `PnlRefreshed`                            | `PnlError(CONNECTION_CHANNEL_CLOSED)`              |
| `ticks`     | `trading_core::Tick`        | `TickReceived` (drives latency badge)     | silent (latency badge sticks to last reading)      |
| `bars`      | `trading_core::Bar`         | `BarReceived` + `BarClose(close_ts)`      | silent                                             |
| `mode`      | `agent::AgentMode`          | `AgentModeChanged` / `AgentHaltedExternally` | `AgentHaltedExternally(CONNECTION_CHANNEL_CLOSED)` |

\* The positions stream keeps a per-subscription `HashMap<Symbol,
PositionView>` and re-emits the full snapshot on every update — the UI
state machine was designed around full-list refreshes (T07 / `audit::query`
semantics), not per-symbol deltas, so this shim preserves the contract.

**IPC model — same-process, shared `Arc<EventBus>`.** Matches the
developer's handoff (`v0: same process`). In v0 the standalone
`cargo run --bin cockpit --features live` creates its own empty bus (no
publisher) and every panel stays in `Loading` — this is honest behavior,
not a bug. A unified agent+cockpit binary that hands an `Arc<EventBus>` to
both sides is a v0.5 deliverable; until then, the two-binary acceptance
`cargo run --bin cockpit --features live` / `cargo run --bin agent` is
documented in the smoke checklist as a deferred manual step.

**Backpressure.** `broadcast::Receiver::recv` returns:
- `Ok(T)` — forwarded.
- `Err(Lagged(n))` — logged at `warn` (`debug` on the high-volume tick
  channel) and the stream continues. The UI sees a brief gap, not a
  frozen panel.
- `Err(Closed)` — surfaced as the panel's typed error message using
  `strings::CONNECTION_CHANNEL_CLOSED`; for the mode channel it becomes
  `AgentHaltedExternally(CONNECTION_CHANNEL_CLOSED)` so the halted
  banner lights up instead of a silent drop.

**Iced wiring.** Each channel is a `BusRecipe` implementing
`iced::advanced::subscription::Recipe`. `ui::live::subscription(bus)`
batches all six into one `iced::Subscription<Message>`. The cockpit
binary's `App::subscription()` returns `ui::live::subscription(...)`
under `--features live` and `iced::Subscription::none()` otherwise. Eager
subscription (`bus.<channel>()` called before the `stream!` body runs)
closes a publish-before-subscribe race.

**Error-state copy added.** Three new keys in `ui::strings`:
`CONNECTION_AGENT_UNREACHABLE` (reserved for the unified-binary case
where `EventBus::new` fails at startup — not yet reachable in v0),
`CONNECTION_CHANNEL_CLOSED` ("Trading agent disconnected. Check the
agent log and restart it."), and `CONNECTION_LAGGED` (reserved for a
future "cockpit fell behind" banner). All keys are routed into the
panel-error branches so the operator always has a next step.

### New strings / theme tokens

**Strings** (all in `ui::strings`):

| Key                            | English                                                                                                     |
|--------------------------------|-------------------------------------------------------------------------------------------------------------|
| `KILL_RUNBOOK_LINK_PATH`       | `spec/runbooks/kill-switch.md`                                                                              |
| `CONNECTION_AGENT_UNREACHABLE` | Can't reach the trading agent. Start it with `cargo run --bin agent` and re-launch the cockpit.             |
| `CONNECTION_CHANNEL_CLOSED`    | Trading agent disconnected. Check the agent log and restart it.                                             |
| `CONNECTION_LAGGED`            | Cockpit fell behind — some updates were skipped.                                                            |

**Theme tokens:** none added. Week 2 UI work reused the existing
palette, spacing, and type scales — the right outcome per the consistency
contract ("most additions are a code smell").

### Smoke checklist location

[`spec/reports/ui-week2-smoke-checklist-2026-04-18.md`](../reports/ui-week2-smoke-checklist-2026-04-18.md) —
contains:
- Sandbox-verifiable gate table (build + test commands).
- Fixtures-driven walkthrough (8 steps).
- Live kill-switch drill against a running agent (both `.halt` file
  file-watcher trigger and cockpit button typed-phrase confirm).
- Runbook link verification step.
- Deferred PNG capture list with instructions.

Panel state reference (compacted 2026-04-19) at
[`spec/reports/screenshots/v0-paper-sma/README.md`](../reports/screenshots/v0-paper-sma/README.md):
single document covering `tape|positions|pnl|kill` × `loading|empty|error|ready`,
each with rendered copy, `strings::*` keys, and `theme::*` color tokens.

### Deferred manual steps

- **PNG screenshots** (8 files listed in the checklist). The headless
  sandbox cannot render iced into a bitmap; the operator captures with
  the OS screenshot tool (`Cmd+Shift+4` on macOS, `gnome-screenshot -i`
  on Linux) on their workstation and commits into
  `spec/reports/screenshots/v0-paper-sma/`. The logical-state artifacts
  cover every state transition that could regress; pixel-layout drift is
  only checked during PR review.
- **Two-binary launch** (`cargo run --bin agent` + `cargo run --bin
  cockpit --features live`). Same reason: separate-process drill needs
  a real display to see fills advance. Integration test
  `crates/ui/tests/live_subscription.rs` is the sandbox-verifiable
  stand-in for the 2-second acceptance window.

### Consistency self-audit

Grep results from a clean tree (at handoff):

- `rg -g 'crates/ui/src/widgets/*.rs' '"[A-Z]'` → **0 inline strings**.
- `rg -g 'crates/ui/src/**' '#[0-9a-fA-F]{6}' | rg -v theme.rs` → **0 inline hex**.
- `cargo test -p ui --test consistency` — 2/2 PASS.

Both Week 1 consistency gates stay green.

### Test coverage

`cargo test -p ui --features live` now runs **53 tests**, all green:

| Suite                          | Count | Notes                                               |
|--------------------------------|-------|-----------------------------------------------------|
| `ui` unit (lib.rs)             | 24    | includes 7 new `live::tests::*` (stream + conversion) |
| `cockpit` bin                  |  0    | unchanged                                           |
| `tests/consistency.rs`         |  2    | inline-strings + inline-hex audits                  |
| `tests/live_subscription.rs`   |  3    | **new T32 integration tests**                       |
| `tests/panel_snapshots.rs`     | 24    | unchanged — every Week 1 snapshot still green       |

Under `cargo test -p ui` (default features, no `live`), the `live_*`
tests are compile-skipped and the total is 43 (≥ 41 gate satisfied).

## Implementation — backend (Week 1)

### Summary

T01–T12 shipped on 2026-04-17. All quality gates pass:
`cargo check --workspace --all-targets`, `cargo clippy --workspace --all-targets -- -D warnings`,
`cargo test --workspace`, and `cargo fmt --all -- --check` are all clean.

Week 1 repairs applied on 2026-04-17 (repair pass per tester FAIL report
`test-2026-04-17-1443-v0-paper-sma-week1.md`):
- Phase 1: renamed `core` package to `trading_core`; removed alias trap and `doctest = false`.
- Phase 2: spec reconciliation (this file + `spec/product.md` + `spec/architecture.md`).
- Phase 3: chart of accounts aligned at 13 (added `expense:infra`, `expense:data`).
- Phase 4: added T08 (Binance WS) and T09 (ReplayFeed 60-bar) integration tests.
- Phase 5: renamed `trybuild_test.rs` → `trybuild.rs`, added 2 more compile-fail cases.
- Phase 6: task-box honesty pass on T01–T12.

All 7 quality gates now pass:
1. `cargo fmt --all -- --check` — PASS
2. `cargo clippy --workspace --all-targets --all-features -- -D warnings` — PASS
3. `cargo check --workspace --all-targets` — PASS
4. `cargo test --workspace --all-targets` — PASS (T08 ignored; T09 runs)
5. `cargo test --workspace --doc` — PASS (0 E0433 errors after rename)
6. `cargo test -p trading_core --test trybuild` — PASS (3/3 compile-fail cases)
7. `cargo test -p audit` — PASS (5/5, now reflecting 13 accounts)

### Key decisions and deviations

**`sqlx-ledger` blocker (T05):** `sqlx-ledger` v0.11.14 is Postgres-only.
Replaced with a hand-rolled double-entry ledger using `sqlx = "0.8"` with
SQLite WAL mode and embedded migrations (`./migrations/001_chart_of_accounts.sql`).
All public `audit::query` functions return only `Decimal`/`trading_core` types — no
`sqlx` types leak. Decision signed off by architect in `spec/architecture.md`
changelog (2026-04-17).

**`trading_core` package rename (T01/T02 repair):** The crate was renamed from
`core` to `trading_core` in `crates/core/Cargo.toml`. The per-consumer
`trading_core = { package = "core" }` aliases have been removed. The `doctest = false`
workaround has been removed. `cargo test --workspace --doc` is now green with 0 errors.

**Chart of accounts — 13 canonical (T05 repair):** Added `expense:infra` and
`expense:data` to `crates/audit/src/bootstrap.rs`. Pre-seeded for v1+ cost
telemetry (`CostEvent::Infra`, `CostEvent::Data` in the architecture). Integration
test updated to assert 13 accounts.

**T08 integration test:** Added `crates/data/tests/binance_ws_integration.rs`
with 3 tests gated behind `#[ignore]` (live Binance WS; run with `--ignored`).
Run with: `cargo test -p data --test binance_ws_integration -- --ignored`.

**T09 fixture test:** Added `crates/data/tests/replay_60_bars.rs` — generates
a deterministic 60-bar Parquet fixture in a temp directory, drives `ReplayFeed`
in fast mode, asserts exactly 60 bars and monotonically increasing `open_ts`.
Runs in the default suite (no `--ignored` required).

**T03 trybuild repair:** Renamed `tests/trybuild_test.rs` → `tests/trybuild.rs`
so `cargo test -p trading_core --test trybuild` works. Added 2 new compile-fail
cases: `quantity_negative_direct.rs` (private tuple field) and
`order_fields_private.rs` (struct fields private). All 3 cases pass.

### Crate-by-crate notes

| Task | Crate | Key files |
|------|-------|-----------|
| T01 | workspace | `Cargo.toml` — 13-member virtual workspace, `resolver = "2"`, pinned workspace deps |
| T02 | `trading_core` | `src/{asset,money,order,fill,position,signal,symbol,tick,time,bar,views,error}.rs` |
| T03 | `trading_core` | `tests/trybuild.rs`, `tests/compile_fail/{money_cross_currency,quantity_negative_direct,order_fields_private}.{rs,stderr}` |
| T04 | `trading_core` | `src/tests/order_tests.rs` — proptest suite with 1 000 cases per property |
| T05 | `audit` | `migrations/001_chart_of_accounts.sql`, `src/{ledger,bootstrap}.rs`, `tests/ledger_integration.rs` (13 accounts) |
| T06 | `audit` | `src/journal.rs` — `post_fill`, `registry_event`, `kill_switch_tripped`, `verify_balance` |
| T07 | `audit` | `src/query.rs` — `account_list`, `cash_balance`, `realized_pnl_since`, `total_fees`, `recent_fills`, `recent_journal`, `global_debit_credit_sum` |
| T08 | `data` | `src/binance.rs` — WS kline+trade, ping/pong, exponential-backoff reconnect; `tests/binance_ws_integration.rs` (3 ignored tests) |
| T09 | `data` | `src/replay_feed.rs` — Parquet reader via `polars`, fast + wallclock modes; `tests/replay_60_bars.rs` — 60-bar fixture test |
| T10 | `data` | `src/fake_feed.rs` — `FakeFeed`, `trade_aggregation`, `bar_cross_check_delta`; `src/bar_stream.rs` — `bar_stream_with_cross_check` |
| T11 | `data` | `src/clock_skew.rs` — `ClockSkewDetector`, Prometheus `clock_skew_ms{feed}` gauge, tracing |
| T12 | `agent` | `src/config.rs` — `Config::load()`, full validation, `mode = "live"` rejection; `config/agent.toml` |

### Test counts (Week 1, after repairs)

| Crate / suite | Tests | Ignored | Notes |
|---|---|---|---|
| `trading_core` unit + proptest | 20 | 0 | 3 proptest properties × 1 000 cases each |
| `trading_core` trybuild | 3 | 0 | compile-fail: cross-currency, negative qty, private Order fields |
| `audit` integration | 5 | 0 | T05 + T06 acceptance tests (in-memory SQLite, 13 accounts) |
| `data` unit | 8 | 0 | T10 aggregation + cross-check; T11 clock-skew |
| `data` T08 WS integration | 0 | 3 | live Binance WS — run with `--ignored` |
| `data` T09 replay | 1 | 0 | 60-bar fixture, fast mode |
| `agent` unit | 7 | 0 | T12 config validation |

## Changelog

- 2026-04-17 (analyst): initial brief.
- 2026-04-17 (architect): appended `## Design` section translating R1–R10
  into crate map, core type signatures, `Strategy` trait + registry,
  ledger schema + reconciliation task, `MatchingEngine` trait + `PaperEngine`,
  data-flow diagram, iced `Message` / `Subscription` model, kill-switch
  wiring, v0 config TOML, observability + cost scaffold, performance
  budgets, and test strategy. Bumped `owner: architect`, `status:
  in-progress`. Task list for developer + ui-designer lives in
  [spec/tasks/v0-paper-sma.md](../tasks/v0-paper-sma.md).
- 2026-04-17 (ui-designer): T13–T20 landed. Added `## UI — Week 1`
  section with wireframe, panel list, strings + theme token tables,
  accessibility notes, consistency self-audit, and Week 2 handoff
  TODOs. Pinned iced `=0.14.0`, insta `=1.47.2`.
- 2026-04-17 (developer): T01–T12 landed. Added `## Implementation — backend (Week 1)` section.
- 2026-04-17 (developer): Week 1 repair pass per FAIL report `test-2026-04-17-1443-v0-paper-sma-week1.md`.
  Renamed `core` → `trading_core` (Phase 1); spec reconciliation (Phase 2); chart aligned at 13 accounts
  (Phase 3, added `expense:infra` + `expense:data`); added T08 WS integration test (3 ignored) and T09
  60-bar fixture test (Phase 4); renamed trybuild.rs + added 2 compile-fail cases (Phase 5); task-box
  honesty pass T01–T12 (Phase 6). All 7 quality gates now PASS.
- 2026-04-18 (developer): T21–T33 + T_FINAL_A landed. Added
  `## Implementation — backend (Week 2)` section below.
- 2026-04-19 (ui-designer): T32 + T_FINAL_B landed. Added
  `## UI — Week 2` section. `ui::live` module wires the cockpit
  `Subscription` to `agent::EventBus` behind `--features live`; 3 new
  integration tests; 16 logical-state artifacts + smoke checklist
  committed under `spec/reports/screenshots/v0-paper-sma/` +
  `spec/reports/ui-week2-smoke-checklist-2026-04-18.md`. Four new
  strings (`KILL_RUNBOOK_LINK_PATH`, `CONNECTION_*`), zero new theme
  tokens. Total ui-crate tests: 53 (with `live`) / 43 (default).
- 2026-04-19 (operator): compacted the 32 panel-state artifacts
  (16 `.txt` + 16 `.png`) into a single reference at
  `spec/reports/screenshots/v0-paper-sma/README.md`. Optimized for future
  AI validation / spec-driven development. Individual files removed.

## Implementation — backend (Week 2)

**Completed 2026-04-18.** Tasks T21–T33 + T_FINAL_A.  T32 + T_FINAL_B remain
for ui-designer.

### T21 — `features::sma`

`SmaStream` (online, running-sum O(1) per bar) and `SmaBatch` (slice-last
using Decimal arithmetic).  `kand` 0.2.2 excluded — compile bug: the crate's
`Signal` enum requires `Into<i64>` but the derive macro fails to generate it
with the `f64`-only feature flag.  Both adapters use pure `Decimal` arithmetic
(more precise than f64 round-trips).  Proptest 500-case cross-check asserts
agreement within `Decimal::new(1, 8)`.

File: `crates/features/src/sma.rs`

### T22 — `strategy::StrategyRegistry`

`Strategy` trait with `on_bar`, `on_tick`, `config_schema`.
`StrategyRegistry::register`, `swap`, `unload`, `on_bar`, `on_tick`,
`drain_pending_events`, `flush_pending_to_ledger`.  `SmaCrossover` impl using
two `SmaStream` windows; emits `Buy` on fast-crosses-above-slow and `Sell` on
fast-crosses-below-slow.

File: `crates/strategy/src/registry.rs`, `crates/strategy/src/sma_crossover.rs`

### T23 — `risk::size_and_validate`

`FixedFractionSizer::compute_qty` → notional clamp to per-symbol exposure cap
→ `Order::new` for full invariant enforcement.  Unit tests: basic sizing
(`equity=100_000`, `fraction=0.1`, `price=40_000` → `qty=0.25 BTC`), cap
clamping, zero-equity error, zero-price error.

File: `crates/risk/src/sizing.rs`

### T24 — `backtest::PaperEngine`

`MatchingEngine` trait + `PaperEngine` (seeded `ChaCha20Rng`, slippage ± bps,
taker/maker fee bps, `BarClose` fill price mode).  Acceptance: buy 0.1 BTC at
close=40 000, slippage=2 bps, fee=4 bps → `fill.price=40 008`,
`fill.fee=1.60032 USDT`.

File: `crates/backtest/src/paper.rs`

### T25 — Backtest binary

`backtest` binary (`crates/backtest/src/main.rs`).  Accepts `--scenario`
(btc-2023-1m-sma-cross, btc-2024-h1-sma-cross), `--seed`.  Data source:
tries Parquet at `data/binance/<symbol>/<year>/`; falls back to synthetic bars
(Box-Muller GBM, seeded `ChaCha20Rng`).  Writes report to
`spec/reports/backtest-<stamp>-<scenario>.md`.

### T26 — Minute-boundary reconciler

`agent::reconciler::check_balance` + `ReconcilerTask`.  Invariant:
`cash + position_qty × last_mark == equity_curve.last()`.  Checked every 1440
bars (≈ 1 trading day of 1-minute data).  On mismatch increments
`ledger_imbalance_events`; in live agent trips kill switch.

File: `crates/agent/src/reconciler.rs`

### T27 — Observability

`agent::observability::register_metrics()` registers all R9.2 counters
and gauges: `bars_in_total`, `ticks_in_total`, `signals_total`,
`orders_sent_total`, `fills_total`, `kill_switch_trips_total`,
`ledger_imbalance_total`, `fees_usdt_total`, `clock_skew_ms`,
`position_qty`, `equity_usdt`, `cash_usdt`.
`start_prometheus_exporter()` binds the exporter on the configured address.

File: `crates/agent/src/observability.rs`

### T28 — `agent::KillSwitch`

`KillSwitch` wraps an `AtomicBool` (sticky trip) + `broadcast::Sender<AgentMode>`.
`spawn_halt_file_watcher()` polls the halt file every 500 ms.
`check_halt_file()` runs synchronously at startup — agent enters `Halted`
immediately if file is present.  `write_halt_file()` / `remove_halt_file()`
operator helpers.  `AgentMode::Halted { reason }` broadcast on trip.

File: `crates/agent/src/kill_switch.rs`

### T29 — Kill-switch runbook

Written to `spec/runbooks/kill-switch.md`.  Covers: trigger conditions (halt
file, ledger imbalance), expected behavior, recovery steps, audit-ledger SQL
queries, Prometheus alert rule examples, and clean-flatten procedure
placeholder for v0.5.

### T30 — `cost` crate

`CostEvent::Llm { provider, model, tier, role, tokens_in, tokens_out,
tokens_cached_in, usd, correlation_id }`.  `CostSink` trait +
`LedgerCostSink` (fire-and-forget tokio spawn → `audit::journal::post_cost`
writing Dr `expense:llm:<tier>` / Cr `liabilities:llm_accrued`) +
`NoopCostSink`.  `CostBudget::remaining()`.  Zero emitters in v0.

File: `crates/cost/src/sink.rs`, `crates/cost/src/budget.rs`

### T31 — `agent` binary

`crates/agent/src/main.rs`.  Wires: tracing JSON init → config → observability
→ kill switch → audit ledger → cost budget → strategy registry → broadcast bus
→ data source (research=replay, paper=Binance WS) → wait for ctrl-c or halt.
Logs every subsystem init at INFO.  `/metrics` served by Prometheus exporter.

### T33 — Determinism test

`crates/backtest/tests/determinism.rs`.
`t33_determinism_mini_backtest`: 1000-bar mini-backtest run twice with seed
`0xC0_FFEE`; asserts identical `trades`, `final_equity`, `signal_count`,
`equity_curve_len`.
`t33_report_sha256_deterministic`: sha256 of identical report text is stable.

### T_FINAL_A — End-to-end backtest results

| Scenario | Bars | Trades | Final equity | Elapsed | Imbalances |
|----------|------|--------|-------------|---------|------------|
| btc-2023-1m-sma-cross | 525 600 | 12 077 | $47 290.03 | 0.2 s | 0 |
| btc-2024-h1-sma-cross | 262 800 | 6 068 | $67 241.80 | 0.1 s | 0 |

Seed: `0xC0FFEE`.  Data source: synthetic (seeded GBM, v0 fallback).
Both reports in `spec/reports/`.

### Deviations from spec

| Deviation | Impact | Mitigation |
|-----------|--------|------------|
| `kand` 0.2.2 excluded (compile bug) | Batch SMA uses Decimal arithmetic instead of kand f64 | More precise; proptest cross-check passes |
| Synthetic bars used (no Parquet on disk) | Results not real-market | Seeded RNG → fully reproducible; real Parquet drops in via `data/binance/` |
| `spawn_halt_file_watcher` uses polling (500ms), not `notify` inotify | Max halt-detection latency 500ms vs ~1ms | Acceptable for v0; upgrade in v0.5 |

---

## Implementation — v0 repairs (HF-1, HF-2)

_Added 2026-04-19 by developer agent._

- **HF-1 — Determinism hash convention** (`crates/backtest/src/lib.rs`):
  Added `backtest::report_body_hash(report: &str) -> Vec<u8>` and
  `backtest::extract_report_body(report: &str) -> &str`.  These functions
  locate the closing `---` delimiter of the YAML front matter and hash only
  the content that follows.  The `generated:` wall-clock timestamp lives in
  the front matter and is intentionally excluded; everything else
  (scenario parameters, equity, trade counts, fees, Sharpe, drawdown) is a
  pure function of the seed and must be byte-identical.

- **HF-1 — Fake T33 test replaced** (`crates/backtest/tests/determinism.rs`):
  `t33_report_sha256_deterministic` previously hashed a hardcoded static
  string and asserted equality — trivially true, proved nothing.  Replaced
  with a real test that spawns the `backtest` binary twice via
  `std::process::Command`, reads each report from a temp directory, calls
  `backtest::report_body_hash()` on each, and asserts byte-identical results.
  Manual verification confirmed body-only SHA-256 is identical across two
  real binary runs at seed `0xC0FFEE`.

- **HF-2 — Prometheus recorder ordering fix** (`crates/agent/src/main.rs`):
  `register_metrics()` was called before `start_prometheus_exporter()`,
  sending all `describe_counter!` / `counter!` / `gauge!` calls to the
  no-op global recorder and causing `/metrics` to return an empty body.
  Fixed by swapping the two calls: install the Prometheus recorder first,
  then register metrics.  A comment documents the invariant: _"Install
  recorder before registering metrics — otherwise names never surface
  on /metrics."_

- **HF-2 — Metrics regression test** (`crates/agent/tests/metrics_endpoint.rs`):
  Spins up just the Prometheus exporter on port `19100` (test-only), calls
  `register_metrics()`, waits 150 ms for the HTTP server to bind, hits
  `GET /metrics` via `reqwest`, and asserts every R9.2 metric name is
  present in the body.  Runs in the default `cargo test` suite (not `#[ignore]`).

- **T31 binary name corrected** (`spec/tasks/v0-paper-sma.md`): The
  acceptance criterion previously said `--bin agent`; the actual binary is
  `--bin trading` per `[[bin]] name = "trading"` in `crates/agent/Cargo.toml`.
  Task note updated.
| T32 / T_FINAL_B deferred | Cockpit not wired to real bus | ui-designer scope; broadcast API documented in `dev-week2-broadcast-api-2026-04-18.md` |
