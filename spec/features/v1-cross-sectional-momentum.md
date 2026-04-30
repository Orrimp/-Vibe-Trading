---
slug: v1-cross-sectional-momentum
status: in-progress
owner: ui-designer
updated: 2026-04-30
---

# v1 — Cross-Sectional Momentum (Top-N)

## Why

v1 is the first feature on the [strategy roadmap](../product.md#strategy-library--roadmap)
that is not deliberately edge-free. v0's `sma_crossover` and v0.5's three
composed recipes (`btc_macd_trend`, `btc_rsi_reversion`,
`btc_bbands_mean_revert`) were tracer bullets — their analyst hypotheses
explicitly expected weak or negative Sharpe and "any positive number is a
red flag to recheck the fee model" (v0.5 Scenario 2 hypothesis,
[v05-composed-strategies.md](v05-composed-strategies.md)). Cross-sectional
momentum is the first **plausible edge candidate** on the roadmap and the
first feature that genuinely requires the harness to run **multiple symbols
at once**. Per [product.md → Universe & data fidelity ladder](../product.md#universe--data-fidelity-ladder),
v1's universe is "Top-10 USDT spot, 1m + L2 + funding context, +Kraken" —
this brief takes that ladder entry as binding for the universe size and
defers the L2 / funding / multi-venue questions explicitly to the architect
(see Notes at the bottom). The point of v1 is to stretch every implicitly
single-symbol code path — `BinanceFeed`, `ReplayFeed`, the backtest
interleave order, the strategy registry's signal fan-out, the position
book, the risk sizer, the cockpit's positions panel — and prove they
honestly handle a universe.

The locked moat bet — **persistent memory + double-entry audit** per
[product.md → Differentiator](../product.md#differentiator) — is the bet
v1 finally lets pay off in a measurable way. Per-symbol P&L attribution
becomes meaningful for the first time: the audit ledger's `Position` rows
are already keyed by `Symbol` (v0 R3.2, R3.3 — see
[v0-paper-sma.md](v0-paper-sma.md)) so the schema needs no change, but
v0 only ever exercised a single key (`BTC`). v1 turns that single row
into ten and makes a query like "ETH momentum signal explained 40% of
last week's realized P&L" a real ledger slice rather than a hypothetical.
The same broadening turns v1+ reflection memory from a thought experiment
into a system that has signal: lesson cards can now correlate per-symbol
outcomes ("long ETH on a negative funding-flip in a positive-momentum
regime ended in chop with fee drag") in a way single-symbol BTC could
not. The strategies panel that v0.5 added to the cockpit
([v05-composed-strategies.md → R5](v05-composed-strategies.md), tasks
T522–T528) already accepts multi-strategy state but in v1 sees its
first multi-symbol position roster from a single strategy.

v1 is **not** validating edge confirmation — paper backtest only, no live
data trading promotion. v1 is **not** introducing LLMs, DL forecasters, or
RL — those land on the v2 / v2.5 / v3 tiers per the strategy roadmap
([product.md → Strategy library — roadmap](../product.md#strategy-library--roadmap)).
v1 is **not** introducing perp/leverage/short execution — spot-only,
and per R4 the v1 strategy ships **long-only** because spot crypto cannot
naturally short. v1 is **not** introducing macro overlays (DXY, US10Y,
SPX) — those are deferred per the roadmap. v1 stays inside paper mode
on real data per [product.md → Project scope boundary](../product.md#project-scope-boundary);
real-money execution is a follow-up project, not v1, not v2, not v3.
LLM cost stays at $0.00 for this feature; the only cost line that may
move is the data feed (10× the symbol stream) — see R10.

## Requirements

Numbered, testable, derived from
[product.md → Strategy library — roadmap](../product.md#strategy-library--roadmap),
[product.md → Universe & data fidelity ladder](../product.md#universe--data-fidelity-ladder),
[architecture.md → Strategy registry & hot-loading](../architecture.md#strategy-registry--hot-loading),
and the v0 + v0.5 ship state in
[spec/reports/screenshots/v0-paper-sma/README.md](../reports/screenshots/v0-paper-sma/README.md).
Each ends with a one-line **acceptance** the tester can verify. All
requirements preserve the v0 `Strategy` trait shape (no trait changes)
and the v0.5 audit / broadcast / strategies-panel surfaces (no schema
changes).

### R1 — Universe spec

- **R1.1** v1 universe is **fixed at strategy load time** to the
  Top-10 USDT spot pairs by **30-day notional volume** on Binance,
  resolved as the universe of the backtest period. Universe membership
  is **frozen** for the entire backtest run — no rebalance on listing
  changes, no addition/removal mid-run. This eliminates the
  rebalancing-on-listing complications that contaminate an edge claim.
- **R1.2** Universe is configurable via TOML in the strategy config
  file ([architecture.md → v0.5 strategy registry](../architecture.md#strategy-registry--hot-loading)
  pattern). The strategy file accepts a `universe = [...]` array of
  symbol strings; missing or malformed entries fail load with a
  `StrategyLoadError` (v0.5 R2.5).
- **R1.3** v1 default universe (research mode):
  `["BTCUSDT", "ETHUSDT", "BNBUSDT", "SOLUSDT", "XRPUSDT", "ADAUSDT",
  "DOGEUSDT", "AVAXUSDT", "DOTUSDT", "LINKUSDT"]` — chosen as a
  representative top-10 USDT-spot snapshot for the 2023 backtest period.
  [ASSUMPTION] this list is the analyst's pick of the 2023 top-10 USDT
  notional snapshot; architect to confirm by querying actual
  30-day-notional ranks against Binance Vision archives, or push back
  with the actual top-10 if it differs.
- **R1.4** All v1 universe symbols are **USDT-quoted spot pairs** —
  spot only per [product.md → Non-goals](../product.md#non-goals)
  (no perps, no leverage). The settlement currency is USDT for all
  ten, so the existing `Money<Usdt>` type machinery (v0 R2.1)
  applies unchanged.
- **Acceptance:** loading a strategy TOML with a `universe = [...]`
  array of ten symbols populates `ComposedStrategy.universe` (or the
  new `MomentumStrategy.universe`, see R7) with exactly those ten
  `Symbol` values; loading the same file with a malformed universe
  (eight symbols, an unknown symbol, an empty array) returns
  `StrategyLoadError { error_code: "invalid_universe" }` and leaves
  the prior strategy untouched per v0.5 R8.

### R2 — Multi-symbol data ingest

- **R2.1** The existing `MarketDataSource` trait
  ([architecture.md → v0 hand-rolled Binance WS](../architecture.md#v0-decision--hand-rolled-binance-ws--marketdatasource-trait--confirmed-2026-04-17))
  is **extended or composed** so the agent runtime ingests N symbols
  in parallel. Concrete plumbing pattern (multiplexed single
  WebSocket vs. one connection per symbol vs. an `mpsc::merge` of
  per-symbol streams) is the architect's call. Analyst's
  requirement is just: at v1 universe size, multi-symbol ingest
  works without per-symbol-per-bar lag explosion.
- **R2.2** Bounded memory: the agent's bus capacities (`bars`,
  `ticks`, `fills` per v0 `[bus]` config table) must scale to 10×
  symbols without growing without bound. Either capacities are
  multiplied by symbol count at startup, or the bars/ticks channels
  are sharded per-symbol — implementation choice deferred to architect.
- **R2.3** Per-symbol clock-skew detection: v0 R1.3 currently emits
  `ClockSkew` warnings + a `clock_skew_ms{feed}` Prometheus gauge
  scoped per feed. v1 extends the gauge label set so skew is reported
  **per symbol** (`clock_skew_ms{feed,symbol}`). A persistent
  per-symbol skew breach still trips the kill switch (v0 R7) — it is
  **not** scoped down to "halt this symbol only" in v1; one bad
  symbol halts the whole agent, matching the v0 conservatism.
- **R2.4** The replay driver (v0 R1.4 `ReplayFeed`) is extended so a
  single backtest run can replay N symbols from N parquet roots
  (one per symbol, conventional `data/binance/<symbol>/<year>/*.parquet`)
  and merge into the strategy fan-out in **deterministic interleaved
  order** — see R12.
- **R2.5** Live `BinanceFeed`-backed paper mode subscribes to N
  `<symbol>@kline_1m` streams. v1 does **not** require live trade
  streams for all ten symbols (kline-only is sufficient at the v1
  rebalance cadence per R6). [ASSUMPTION] kline-only is sufficient;
  architect may push back if multi-symbol L2 / trade feeds are
  required by the universe-ladder entry's "L2 + funding context"
  language — see Notes.
- **Acceptance:** an integration test starts the replay driver on a
  10-symbol fixture for a 1-hour window (60 bars × 10 = 600 bars
  total) and asserts (a) all 600 bars surface to the strategy
  registry, (b) bars from any single symbol arrive in monotonically
  increasing `venue_ts` order, (c) the merged stream sorts by
  `(venue_ts, symbol)` deterministically (R12), (d) memory
  high-water-mark stays bounded at < 64MiB during a year-long
  replay across all ten symbols.

### R3 — Momentum score

- **R3.1** Per-bar, per-symbol momentum score using a configurable
  lookback `n` (default `n = 60` minutes / 1 hour at the 1m bar
  granularity). Recommended formula:
  `score(s, t) = log(close[s, t] / close[s, t-n]) / realized_vol(close[s], n)`
  — vol-adjusted log return.
- **R3.2** `realized_vol(close[s], n)` is the standard deviation of
  log returns over the same `n`-bar window
  (`std(log(close[s, t-i] / close[s, t-i-1]) for i in 0..n)`). Floor
  at a small `vol_floor = 1e-6` (in log-return units) to avoid
  divide-by-zero on stalled tape. [ASSUMPTION] this floor is the
  analyst's pick; architect may pin a different value or move the
  floor into the `risk` crate as a universal numerical-safety
  constant.
- **R3.3** Implementation reuses the v0.5 indicator-tree pattern
  ([v05-composed-strategies.md → R1](v05-composed-strategies.md))
  where economical, but the cross-sectional reduction (rank N
  symbols by score per bar) is **not** expressible in the v0.5 rule
  DSL (the DSL is per-symbol, scalar-comparison-shaped). v1 ships a
  new `features::cross_sectional` module providing
  `score_vol_adjusted_return(symbol_close_history, n) -> Decimal`
  built on the same `RingBuffer<Decimal>` primitives the v0.5
  indicator nodes use. No new TA-crate dependency; pure-Decimal Rust.
- **R3.4** Score values are `Decimal` end-to-end — no `f64` per the
  v0 R2.2 clippy `float_arithmetic` deny lint. `log` and `sqrt`
  on `Decimal` use a workspace-fixed implementation in
  `features::math` (see R3.5).
- **R3.5** `features::math` adds `decimal_ln(Decimal) -> Decimal`
  and `decimal_sqrt(Decimal) -> Decimal` thin wrappers over a chosen
  algorithm (Taylor / Newton iteration to a fixed precision). Both
  are deterministic across runs (no platform-dependent libm).
  [ASSUMPTION] precision target is 10 decimal places; architect to
  confirm or pin a different precision in the `features` crate's
  numeric-policy doc.
- **Acceptance:** a unit test feeds a synthetic 200-bar series with
  hand-computed expected scores and asserts
  `score_vol_adjusted_return` returns the expected value within
  `Decimal::new(1, 9)` (1e-9) tolerance; a property test asserts
  monotonicity (a strictly increasing close series produces
  monotonically increasing scores given fixed vol).

### R4 — Top-N selector

- **R4.1** At each rebalance bar (R6), the strategy ranks every
  symbol in the universe by its current momentum score (R3) and
  selects:
  - **Top-K longs** — the `K_long` symbols with the highest score.
  - **Bottom-K shorts** — the `K_short` symbols with the lowest score.
- **R4.2** v1 default: `K_long = 3`, `K_short = 0`. v1.5 may explore
  `K_long = K_short = 5` per the roadmap; not in v1 scope.
- **R4.3** [ASSUMPTION] **v1 ships long-only** (`K_short = 0`) because
  spot crypto cannot naturally go short — there is no spot
  short-sell mechanism on Binance / Coinbase / Kraken USDT-spot
  pairs. Shorting via perps lives on the v2 universe-ladder tier
  (`+ Top-25 perps (signal only, not exec)` per
  [product.md → Universe & data fidelity ladder](../product.md#universe--data-fidelity-ladder));
  perp execution itself is even later. Architect to confirm or push
  back: if `K_short > 0` is wanted in v1, the strategy must reduce
  to "exclude these from longs" rather than "open a short position",
  and that change should land here, not in v2.
- **R4.4** Tie-break on equal score: alphabetical `Symbol` ordering
  (`BTCUSDT < ETHUSDT < ...`). Deterministic by R12.
- **R4.5** Symbols without a fully-warmed-up score (insufficient
  history at the start of replay or after a feed reconnect) are
  excluded from ranking — they cannot be selected as either long or
  short. The strategy emits no orders for symbols still warming up.
- **Acceptance:** a unit test feeds ten synthetic per-symbol score
  series, asserts the selector returns the alphabetically-first 3
  symbols on the high tail; a second test inserts two symbols with
  identical (tied) top scores and asserts alphabetical tie-break;
  a third test marks two symbols as "warming up" and asserts they
  are excluded.

### R5 — Position sizing

- **R5.1** Equal-weight across the K positions: each open position
  receives `equity * exposure_cap / K_long` of notional at rebalance,
  where `exposure_cap = 0.5` is the **per-strategy** total-exposure
  cap (sum of long notionals as a fraction of equity) — separate
  from the existing v0 `risk.per_symbol_exposure_cap` (default
  `0.4`).
- **R5.2** `risk::size_and_validate` (v0 R4.5) is extended to
  handle **vector orders**: instead of a single `Order`, the
  strategy emits a `Vec<ProposedOrder>` (one per universe symbol
  needing change), and the risk path validates the **portfolio
  exposure** (`Σ notional_long` ≤ `equity * exposure_cap`) plus
  per-symbol caps (existing v0 invariant) atomically. Either the
  whole vector is accepted, or the whole vector is rejected with
  `RiskError::PortfolioExposureBreach`. Partial acceptance would
  leave the strategy in an inconsistent rebalance state and is
  rejected as a design choice.
- **R5.3** New `risk` config keys:
  - `risk.cross_sectional.exposure_cap` (default `0.50`).
  - `risk.cross_sectional.k_long` (default `3`).
  - `risk.cross_sectional.k_short` (default `0`).
  Validation rules: `k_long + k_short <= len(universe)`,
  `0.0 < exposure_cap <= 1.0`, `k_long >= 1`, `k_short >= 0`.
- **R5.4** Rebalance never increases per-symbol exposure beyond the
  v0 per-symbol cap (`risk.per_symbol_exposure_cap`, default `0.4`).
  At v1 default `exposure_cap=0.5 / K_long=3`, each leg's
  exposure is `~0.167` — well inside the per-symbol cap. The two
  invariants compose: per-symbol cap binds for K=1 degenerate case;
  portfolio cap binds for K large.
- **R5.5** `risk::RiskLimits` gains an optional
  `portfolio_exposure_cap: Option<Decimal>` field. If `Some`, the
  vector validator enforces it; if `None`, only the per-symbol cap
  applies (v0 backward-compat).
- **Acceptance:** a unit test on `risk::size_and_validate` accepts a
  3-leg `Vec<ProposedOrder>` totalling `0.45 * equity` notional and
  rejects the same set with one leg pushed to make total `0.55 *
  equity` (above the `0.50` cap), with `PortfolioExposureBreach`;
  a property test asserts no acceptance ever leaves the portfolio
  total over the cap.

### R6 — Rebalance cadence

- **R6.1** **Hourly default** rebalance (every 60 1m bars). At each
  rebalance bar, the strategy:
  1. Computes per-symbol momentum scores (R3) using bars strictly
     prior to the rebalance bar's open (no look-ahead).
  2. Selects top-K longs (R4).
  3. Generates a `Vec<ProposedOrder>`:
     - **Close**: open positions whose symbol fell out of the new
       top-K → market sell-to-flat (size = current position).
     - **Open**: new top-K members not currently held → market buy
       to target weight.
     - **Hold**: top-K members already held — adjust size if
       equity / target weight has drifted by more than a configurable
       threshold; otherwise leave alone.
  4. Emits a single `Decision` row to the audit ledger summarizing
     the rebalance (universe, scores, selected, action), with the
     per-symbol `Order` rows fanning out from it (each `Order`
     references the parent `Decision` id).
- **R6.2** **Drift threshold** for the hold case: rebalance only if
  `|current_weight - target_weight| / target_weight > 0.10`
  (10% relative drift). Avoids generating fee-drag-only round-trips
  on bars where nothing meaningful changed. Configurable via
  `strategy.cross_sectional.drift_rebalance_threshold`. [ASSUMPTION]
  10% is the analyst's first pick; this is the kind of knob that
  v1.5 will tune from the v1 OOS baseline.
- **R6.3** **Cadence config**:
  `strategy.cross_sectional.rebalance_minutes` (default `60`).
  Validated `>= 1`. Hourly == every 60 bars at 1m granularity; v1
  could in principle accept any positive integer.
- **R6.4** Between rebalance bars, the strategy emits no orders.
  `on_bar` for non-rebalance bars updates score state and returns
  `vec![]`. This is consistent with the v0 / v0.5 edge-triggered
  signal model — no per-bar trading on signal flicker.
- **R6.5** A rebalance is **all-or-nothing** at the risk layer (R5.2)
  — if the portfolio-exposure validator rejects the vector, no
  orders fire and a `RebalanceRejected` ledger entry is written
  (single row, no debits/credits, on the v0.5 `strategy_events`
  table or a sibling `decision_events` table — architect to choose
  the surface; the analyst requires that the rejection is queryable
  from `audit::query`).
- **Acceptance:** a backtest replay over 60 minutes (one rebalance
  bar) with a fresh universe asserts (a) exactly one `Decision`
  row written, (b) exactly `K_long` `Order` rows referencing it,
  (c) no orders fired on the 59 non-rebalance bars; a second test
  with a deliberately over-leveraged synthetic universe asserts a
  `RebalanceRejected` row appears and **zero** `Order` rows fan
  out from it.

### R7 — Strategy plug-in

- **R7.1** v1 strategy ships as a fresh `Strategy` impl in
  `crates/strategy/src/cross_sectional/momentum.rs` —
  **not** as a `ComposedStrategy` recipe. The v0.5 rule DSL
  (per [v05-composed-strategies.md → R2](v05-composed-strategies.md))
  is per-symbol scalar-comparison shaped; cross-sectional ranking
  ("rank these N symbols by score, take top K") does not fit the
  grammar without a redesign. v1 adds a third `Strategy`
  implementation alongside `sma_crossover` (v0) and
  `ComposedStrategy` (v0.5).
- **R7.2** Implementation lives in `strategy::cross_sectional`
  module; the file `momentum.rs` defines `MomentumStrategy` which
  implements the existing v0 `Strategy` trait verbatim (`id`,
  `on_bar`, `on_tick`, `config_schema`). `on_tick` returns
  `vec![]` — momentum is bar-close.
- **R7.3** `MomentumStrategy::on_bar` is called once per bar per
  agent dispatch (the agent's existing `StrategyRegistry::on_bar`
  fan-out, v0.5 R3.4). Because v1 strategies are universe-aware
  (one `MomentumStrategy` instance handles 10 symbols), `on_bar`
  must be called for **every** bar in the universe, not just bars
  whose `Bar.symbol` matches a strategy attribute. The architect
  picks the routing pattern: either `MomentumStrategy` declares its
  universe and the registry only forwards in-universe bars, or
  `MomentumStrategy` filters internally. Analyst's preference is
  the former (registry filters) so a strategy never sees bars
  outside its declared universe.
- **R7.4** `StrategyRegistry` in v0.5 is keyed by `StrategyId` only
  — the `Strategy::on_bar(&mut self, bar: &Bar)` signature does not
  identify which strategy should see which symbol's bars. v1 may
  require a small registry change: a per-strategy
  `interested_in(symbol: Symbol) -> bool` predicate, or a
  `Vec<Symbol>` declared at construction. **No change to the
  `Strategy` trait shape itself** — interest is metadata next to
  the trait object, not a new trait method, to keep the v0.5 hot-load
  surface untouched.
- **R7.5** Strategy load via TOML uses the same v0.5 surface
  (`config/strategies/<id>.toml`, file watcher, hot-swap). New
  TOML schema fields specific to v1:
  ```toml
  id     = "top10_momentum_h1"
  kind   = "cross_sectional_momentum"     # new kind discriminator
  stage  = "research"
  universe = ["BTCUSDT", "ETHUSDT", ...]  # exactly 10 symbols
  lookback_minutes = 60
  k_long  = 3
  k_short = 0
  rebalance_minutes = 60
  drift_rebalance_threshold = 0.10
  size = "equal_weight"                   # new sizing kind for v1
  ```
- **R7.6** The new `kind = "cross_sectional_momentum"` discriminator
  routes the loader to a `CrossSectionalMomentumConfig` deserialize
  path (sibling to `ComposedStrategyConfig`). Unknown `kind` values
  fail load with `unsupported_kind` error code — additive to the
  v0.5 error-code table.
- **R7.7** Hot-swap (v0.5 R3) works identically — editing the v1
  TOML triggers debounced reload, atomic registry swap, journal
  entry, and broadcast bus event. The `strategy_events` table
  ([architecture.md → v0.5 strategy-event journal](../architecture.md#v05--strategy-event-journal-schema-q1--confirmed-2026-04-19))
  carries v1 swaps with no schema change.
- **Acceptance:** loading the v1 TOML at agent startup populates
  the registry with one `MomentumStrategy`; a hot-swap (rewriting
  the file with a new `lookback_minutes`) within a replay produces
  a single `Swap` row in `strategy_events` with new content hash,
  and the next rebalance bar uses the new lookback.

### R8 — Per-symbol P&L attribution

- **R8.1** **Schema confirmation:** the existing 13-account v0
  chart of accounts (v0 R3.2) supports per-symbol attribution out
  of the box. `assets:position:BTC` is one of `assets:position:<asset>`
  — v1 adds `assets:position:ETH`, `assets:position:BNB`,
  `assets:position:SOL`, etc. as nine new sub-accounts on first
  use. The `audit::bootstrap::chart_of_accounts` (v0 R3.2) extends
  to seed all v1 universe asset accounts at startup.
  **No schema change.**
- **R8.2** New `audit::query` reader API:
  ```rust
  pub fn pnl_by_symbol(
      since: Timestamp,
      until: Timestamp,
  ) -> Result<Vec<(Symbol, Money<Usdt>)>, QueryError>;
  ```
  Returns one row per symbol with non-zero realized P&L in the
  window, sorted by `Symbol`. Lives in `crates/audit/src/query.rs`
  next to the existing `realized_pnl_since`. Implementation uses
  `assets:position:<asset>` join with the symbol → asset mapping
  recorded at universe load time.
- **R8.3** [ASSUMPTION] Symbol → base-asset mapping is a static
  table seeded from `MarketDataSource::exchange_info` (v0 R1
  `BinanceFeed::exchange_info`) at strategy load time and stored
  on the strategy struct. `BTCUSDT → BTC`, `ETHUSDT → ETH`, etc.
  Architect to decide whether this mapping lives on `Symbol`
  itself (parsed from the `<base><quote>` convention) or in a
  side table — the analyst's preference is a side table because
  exchange-native symbol parsing is brittle (e.g. `1000SHIBUSDT`).
- **R8.4** The new query is the foundation for the v1+
  [operator success reports](../product.md#operator-success-reports)
  per-strategy + per-symbol attribution slice. v1 does **not**
  ship a cockpit panel for it — the API is locked here so v1.5+
  can add the panel without a query refactor.
- **R8.5** Determinism property: across the v1 backtest, the sum of
  `pnl_by_symbol` rows for the full backtest window must equal the
  scalar `realized_pnl_since(start)` to the satoshi (i.e.
  `Σ pnl_by_symbol(start, end) == realized_pnl_since(start)` at
  `Timestamp(end)`). Asserted as part of V5 below.
- **Acceptance:** at the end of the v1 2023 backtest, querying
  `pnl_by_symbol(2023-01-01, 2024-01-01)` returns up to 10 rows;
  the sum equals `realized_pnl_since(2023-01-01)` exactly; rows
  with zero realized P&L are omitted (e.g. a symbol that was
  never selected); rows are sorted alphabetically by `Symbol`.

### R9 — Multi-symbol backtest harness

- **R9.1** The v0 `backtest` binary's `--scenario` flag (v0 R5.5)
  accepts a v1 scenario that loads a 10-symbol Parquet replay path.
  Scenario config carries a `universe: [...]` array and a
  `parquet_root_template` (e.g. `./data/binance/{symbol}/{year}`)
  that the backtest engine expands per symbol.
- **R9.2** The backtest engine runs all 10 streams in **deterministic
  interleaved order** — bars from different symbols are merged into
  a single sorted-by-`venue_ts` event stream. When two bars share
  the same `venue_ts` (which happens at every minute boundary for
  10 symbols), tie-break is by `Symbol` (alphabetical) per R12. The
  same merged stream feeds the strategy registry's `on_bar` calls,
  so the strategy sees one bar at a time, ordered.
- **R9.3** The backtest report writer (v0.5 R9.3 added a `Strategy`
  section with id + content hash + source path) is extended to
  surface a multi-symbol summary section listing per-symbol metrics:
  total return, trade count, hit rate, avg trade P&L, contribution
  to total Sharpe. Format matches the existing report template's
  metrics table style.
- **R9.4** Backward compatibility: the v0 + v0.5 single-symbol
  scenarios (`btc-2023-1m-sma-cross`, `btc-2024-h1-sma-cross`,
  `btc-2023-1m-sma-baseline-refresh`, `btc-2023-1m-macd-trend`,
  `btc-2023-1m-rsi-reversion`, `btc-2023-1m-bbands-mean-revert`)
  continue to run unchanged and produce **byte-identical reports**
  — anchor SHA `fc2e3b4a04055e60209fe85541173aa8883df226d2756352dfd101597168649c`
  for the v0 sma-cross run per
  [spec/reports/screenshots/v0-paper-sma/README.md](../reports/screenshots/v0-paper-sma/README.md)
  must not move.
- **R9.5** The backtest's `MatchingEngine` (v0 `PaperEngine` —
  bps slippage + taker fee + bar-VWAP, ChaCha20Rng-seeded) is
  unchanged in v1. Slippage is computed per-fill against per-symbol
  bar close — the engine already takes a `Bar` parameter, so v1
  passes the symbol's bar; no refactor needed.
- **Acceptance:** running the v1 2023 scenario produces a report
  with per-symbol metrics rows; running v0's `btc-2023-1m-sma-cross`
  scenario at seed `0xC0FFEE` produces a body-SHA256 byte-identical
  to the locked anchor (regression gate).

### R10 — Cost telemetry

- **R10.1** **LLM cost stays at $0.00.** v1 invokes no LLM. The
  `cost::CostSink` v0 scaffold (v0 R10) is untouched.
- **R10.2** Infra / data feed cost: the live paper-trading mode
  now ingests 10× the WebSocket data vs v0/v0.5. The
  [product.md → Cost economics](../product.md#cost-economics--monthly-ceiling)
  v1 column allocates `$40/month hosting` and `$15/month storage`
  (the LLM line at v1 is `$80` but unused by this feature). v1's
  hosting line is unchanged from "single VM" — Binance WebSocket
  fan-out for 10 spot kline streams is bandwidth-trivial. The
  storage line allows for 10× the parquet replay archive once
  v1 paper-trades for any sustained period.
- **R10.3** v1 introduces a `costs.md` v1 column row in the
  monthly cost rollup report (the tester's auto-generated cost
  report per [product.md → Cost economics](../product.md#cost-economics--monthly-ceiling)).
  Total monthly target ≤ `$135` per the locked v1 ceiling. v1
  feature itself adds zero LLM line items; if the agent runs v0/v0.5
  composed strategies + v1 momentum simultaneously in paper mode,
  the LLM line is still $0 because none of those strategy classes
  call an LLM.
- **R10.4** A regression gate: the tester's bench step
  ([architecture.md → Performance budget](../architecture.md#performance-budget))
  asserts that bar-close → signal stays under `5ms p99` even with
  the 10× symbol load. See V7 below.
- **Acceptance:** the v1 backtest run's auto-generated `costs.md`
  shows `LLM tokens: $0.00`, `Total / month ≤ $135` (v1 ceiling),
  with no infra / data line crossing its budget cell.

### R11 — Cockpit positions panel

- **R11.1** The v0 cockpit positions panel (v0 R6.2) already supports
  multi-row rendering — its column layout is `Symbol | Qty | Cost |
  Mark | P&L | P&L% | Exposure %` with arbitrary row count. **Zero
  UI changes are required for v1.** The operator should now see up
  to `K_long = 3` rows in steady state (one per held symbol)
  rather than the v0/v0.5 single BTC row.
- **R11.2** Empty / loading / error states (v0 R6.4) keep working
  — empty before the first rebalance, ready with up to 3 rows
  after, error if `audit::query::position()` fails for any symbol.
- **R11.3** [ASSUMPTION] zero new ui-designer work for v1 on the
  positions panel; this requirement is a **negative confirmation**
  — the architect / ui-designer signs off that v1 is purely
  data-flow (strategy emits orders for ETH/BNB/SOL/etc. as well as
  BTC; the existing widget renders them) rather than a UI feature.
  The strategies panel (v0.5 R5) likewise needs no schema change —
  one strategy row, three open positions held.
- **R11.4** Operator confirms acceptance: launch cockpit against
  the v1 replay driver, observe up to 3 rows after the first
  rebalance, observe rows mutate as universe selection rotates.
  Manual smoke-test step in V8 below.
- **Acceptance:** running cockpit (`cargo run --bin cockpit
  --features fixtures`) against a v1-fixtures fake bus with a
  preloaded 3-position roster renders three rows with correct
  P&L coloring; no widget code changed.

### R12 — Determinism

- **R12.1** Same gate as v0/v0.5: report body-SHA256 byte-identical
  across two runs of the v1 backtest at seed `0xC0FFEE`. Gate
  applies to the report only — the SQLite ledger DB binary is
  not byte-stable across runs (timestamps, ROWIDs), but the
  `pnl_by_symbol` query results and the report-rendered numbers
  must be.
- **R12.2** Multi-symbol interleave order is deterministic: the
  merged event stream sorts by `(venue_ts ASC, symbol ASC)`, where
  `symbol ASC` is alphabetical on the exchange-native symbol
  string (`Symbol(SmolStr)` lexicographic ordering). This is
  **not just `venue_ts ASC`** — adding the symbol secondary key is
  load-bearing for determinism because at minute boundaries the
  10 bars share `venue_ts`.
- **R12.3** The momentum-score warmup (R4.5) for each symbol is
  deterministic given the same input bar stream — first
  `n + 1` bars produce no score; bar `n + 2` produces the first
  score. Warmup boundaries for all 10 symbols are simultaneously
  reached because they all start from the same fixture bar 0.
- **R12.4** Tie-break on equal momentum scores (R4.4) is
  alphabetical `Symbol` order — already deterministic per R12.2.
- **R12.5** The `risk::size_and_validate` vector path (R5.2) does
  not introduce non-determinism — it iterates the input
  `Vec<ProposedOrder>` in the order the strategy emitted (which
  is alphabetical per R12.4) and accepts/rejects atomically.
- **Acceptance:** two runs of the v1 2023 backtest at seed
  `0xC0FFEE` produce reports with identical body-SHA256 (V5
  below).

## Backtest Scenarios

Two scenarios. Scenario 1 is the v1 in-sample run on a full year of 2023
data — the **first scenario in this project where positive Sharpe is
not automatically a red flag**. Scenario 2 is the v1 OOS baseline on H1
2024 — establishes the v1.5+ regression floor.

### Scenario: `top10-2023-1h-momentum`

- **Universe:** `BTCUSDT, ETHUSDT, BNBUSDT, SOLUSDT, XRPUSDT, ADAUSDT,
  DOGEUSDT, AVAXUSDT, DOTUSDT, LINKUSDT`
- **Period:** `2023-01-01` → `2023-12-31`
- **Granularity:** `1m` bars (rebalance hourly per R6 — every 60 bars)
- **Data source:** `binance-spot` (via
  `data/binance/{symbol}/2023/*.parquet`, one Parquet root per
  universe symbol)
- **Fees:** `0.04%` taker, `0.02%` maker (maker unused — market
  orders only at rebalance)
- **Slippage model:** `bps: 2` (per-fill against per-symbol bar close)
- **Initial capital:** `100_000 USDT`
- **Position sizing:** equal-weight across `K_long = 3` legs;
  per-strategy exposure cap `0.50` of equity; v0 per-symbol cap
  `0.40` still applies (binds in degenerate `K_long = 1` case, not
  here)
- **Risk limits:**
  - Max leverage: `1x` (spot, no margin)
  - Max drawdown stop: `-15%` (v0 default)
  - Per-symbol exposure cap: `40%` (v0 default)
  - Portfolio exposure cap (v1 new): `50%`
- **Strategy params:** `top10_momentum_h1` v1 TOML —
  `kind = "cross_sectional_momentum"`, `lookback_minutes = 60`,
  `k_long = 3`, `k_short = 0`, `rebalance_minutes = 60`,
  `drift_rebalance_threshold = 0.10`, `size = "equal_weight"`
- **Seed:** `0xC0FFEE`
- **Baseline report:** none (this is the in-sample run that
  establishes the v1 baseline).

**Expected outcome (analyst hypothesis):** **This is the first
scenario in the project where a positive Sharpe is not automatically
a red flag.** Cross-sectional momentum on top-10 USDT spot at hourly
rebalance has historically produced Sharpe in the **0.3 – 0.8** range
out-of-sample on similar universes in academic and practitioner
literature ([Jegadeesh & Titman 1993](https://www.jstor.org/stable/2328882)
established the canonical effect; crypto-specific replications (e.g.
[Liu, Tsyvinski, Wu 2022, RFS](https://academic.oup.com/rfs/article-abstract/35/6/2689))
report similar magnitudes for top-N USDT pairs in the 2017–2021
window). v1 acceptance does **not** require beating that — v1
acceptance requires the harness to produce a defensible number we can
**argue with**. A Sharpe in `[-0.2, 1.0]` for the in-sample 2023 run
is plausibly explainable (2023 was a recovery-trend year — momentum
plausibly works); a Sharpe outside that range is a signal to recheck
data alignment, fee model, or score formula before celebrating or
panicking. State the prior plainly: 60-min vol-adjusted return
momentum on top-10 spot at hourly rebalance with `0.04%` taker fee
+ `2bps` slippage **probably** prints positive Sharpe in 2023; we
don't bet money on it; we lock the OOS baseline (Scenario 2) before
making any judgment about edge.

### Scenario: `top10-2024-h1-momentum`

- **Universe:** `BTCUSDT, ETHUSDT, BNBUSDT, SOLUSDT, XRPUSDT, ADAUSDT,
  DOGEUSDT, AVAXUSDT, DOTUSDT, LINKUSDT`
  (identical to Scenario 1 — frozen membership)
- **Period:** `2024-01-01` → `2024-06-30`
- **Granularity:** `1m` bars (rebalance hourly)
- **Data source:** `binance-spot` (via
  `data/binance/{symbol}/2024/*.parquet`)
- **Fees:** `0.04%` taker, `0.02%` maker (maker unused)
- **Slippage model:** `bps: 2`
- **Initial capital:** `100_000 USDT`
- **Position sizing:** identical to Scenario 1 (equal-weight,
  `K_long = 3`, per-strategy exposure `0.50`)
- **Risk limits:** identical to Scenario 1.
- **Strategy params:** identical to Scenario 1
  (`top10_momentum_h1`).
- **Seed:** `0xC0FFEE`
- **Baseline report:** Scenario 1's
  `spec/reports/backtest-<stamp>-top10-2023-1h-momentum.md`.

**Expected outcome (analyst hypothesis):** This is the v1
**out-of-sample baseline**. We expect Sharpe directionally similar
to Scenario 1 — large divergence (Scenario 1 prints `Sharpe = 0.6`
and Scenario 2 prints `Sharpe = -1.2`, or vice versa) is a strong
signal of in-sample overfit at the `lookback_minutes = 60` /
`rebalance_minutes = 60` choice, and v1.5 should sweep those knobs
before promoting to `paper`. The numerical output of this run
becomes the **v1 OOS regression floor** that every later strategy
(v1.5 mean-reversion, v2 LLM overlay, v2.5 DL forecaster, v3 RL
policy) must beat on `top10-2024-h1` per the
[product.md → Strategy lifecycle — promotion gates](../product.md#strategy-lifecycle--promotion-gates)
`research → paper` gate (`Sharpe > 1.0 on 2y OOS data`). v1's
momentum strategy stays in `research` stage at the close of v1 —
promotion is the analyst's next loop after a clean tester report,
contingent on Scenario 2's metrics. v1 itself is shipped as soon
as the harness verifications below pass; the Sharpe number is a
result, not a gate.

## Design

Translates R1–R12 into crate / module additions, Rust types, TOML schema,
new audit/broadcast surfaces, and test strategy. All decisions anchor to
[architecture.md → v1 cross-sectional momentum resolutions (Q1–Q6)](../architecture.md#v1--cross-sectional-momentum-resolutions-q1q6--confirmed-2026-04-29)
and the v0/v0.5 Design sections in
[v0-paper-sma.md → Design](v0-paper-sma.md#design) +
[v05-composed-strategies.md → Design](v05-composed-strategies.md#design).
This section is **strategy + multi-symbol plumbing + per-symbol P&L
attribution + funding observation-only**; v0 / v0.5 crate surfaces stay
untouched except for additive extensions.

### Crate map delta from v0.5

| Crate          | Change in v1                                                                                                                                                                                                                              |
|----------------|------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `trading_core` | **+** `Universe` newtype (`SymbolSet` = `BTreeSet<Symbol>` for deterministic iteration). **+** `FundingObs` message type (Q2). **+** New `RebalanceRejected` variant on `StrategyEventKind`. **+** `RiskLimits.portfolio_exposure_cap: Option<Decimal>`. **No `Strategy` trait changes.** |
| `features`     | **+** New submodule `features::cross_sectional` with `score_vol_adjusted_return(close_history: &RingBuffer<Decimal>, n: u32) -> Result<Decimal, ScoreError>`. **+** New submodule `features::math` with `decimal_ln` / `decimal_sqrt` (deterministic Taylor / Newton, 10 dp precision). |
| `strategy`     | **+** New submodule `strategy::cross_sectional` with `momentum.rs` (`MomentumStrategy`), `score.rs` (per-symbol score cache thin shim over `features`), `selector.rs` (top-K + warmup filter), `mod.rs` (config + factory). **No registry change** — Q5 picks strategy-side filtering. |
| `risk`         | **+** New `size_portfolio_target(target_weights, equity, prices, position_book, limits) -> Result<Vec<Order>, RiskError>` vector-order sizer. Existing scalar `size_and_validate` stays. **+** `RiskLimits` consumer reads the new `portfolio_exposure_cap` field. |
| `audit`        | **+** Migration `003_funding_rates.sql` (Q2). **+** `audit::journal::rebalance_rejected(..)` writer (Q6 — uses existing `strategy_events` table, no schema migration). **+** `audit::query::pnl_by_symbol` reader. **+** `audit::query::funding_rate_history` reader. **+** Chart-of-accounts extension: `assets:position:<asset>` for nine new universe symbols (additive; bootstrap seeds them on first agent startup with the v1 universe). |
| `data`         | **+** New `data::funding::FundingPoller` (REST + tokio task). **+** `ReplayFeed::merge_symbols(roots: &[(Symbol, PathBuf)]) -> impl Stream<Bar>` deterministic interleave by `(venue_ts ASC, symbol ASC)`. **+** Multi-symbol `BinanceFeed::subscribe_bars_multi(symbols: &[Symbol])` thin wrapper that fans out per-symbol `subscribe_bars` calls into a merged `BoxStream` (no protocol change — Binance multi-stream is one WS connection per symbol or a `?streams=` combined endpoint; developer's call). |
| `agent`        | **+** Multi-symbol ingest plumbing — universe-aware bus capacity scaling. **+** `funding_obs` broadcast channel (capacity 32). **+** `agent::funding_poller_task` wires the poller into the orchestrator. **+** Per-symbol `clock_skew_ms{feed,symbol}` Prometheus label (R2.3). |
| `backtest`     | **+** New `--scenario top10-2023-1h-momentum` and `top10-2024-h1-momentum`. **+** Multi-symbol replay driver (uses `ReplayFeed::merge_symbols`). **+** Report writer per-symbol metrics section (R9.3). |
| `ui`          | Unchanged — R11 is a negative confirmation. The existing positions panel renders multiple rows; the strategies panel (v0.5) absorbs one new strategy row. v1 ui-designer task is smoke-only (V8). |
| `cost`, `exec`, `models`, `llm` | Unchanged in v1. |

**Dependency edges (additive):**

```
trading_core ← strategy::cross_sectional, data::funding,
               risk (portfolio sizer), audit (rebalance_rejected),
               agent (funding bus)
audit        ← agent::funding_poller_task (writes funding_rates rows)
data         ← agent (funding poller), backtest (multi-symbol replay)
features     ← strategy::cross_sectional (score module)
risk         ← strategy::cross_sectional (sizer call site is in agent,
               but risk depends only on trading_core)
```

No new crate is introduced. No edge reverses. The v0/v0.5
audit/broadcast/strategies-panel surfaces are unchanged.

### Universe types (R1)

```rust
// crates/core/src/universe.rs (new)
use std::collections::BTreeSet;

/// Deterministic, sorted set of `Symbol`s — alphabetical iteration order.
/// Used as the v1 universe spec; iteration order is the determinism gate
/// that R12.2 / R12.4 pin.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SymbolSet(BTreeSet<Symbol>);

impl SymbolSet {
    pub fn new(symbols: impl IntoIterator<Item = Symbol>) -> Result<Self, UniverseError> {
        let set: BTreeSet<_> = symbols.into_iter().collect();
        if set.is_empty() {
            return Err(UniverseError::Empty);
        }
        Ok(Self(set))
    }
    pub fn contains(&self, s: &Symbol) -> bool { self.0.contains(s) }
    pub fn iter(&self) -> impl Iterator<Item = &Symbol> { self.0.iter() }
    pub fn len(&self) -> usize { self.0.len() }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Universe {
    pub symbols:   SymbolSet,
    /// Symbol → base-asset mapping captured at load time from
    /// `MarketDataSource::exchange_info` (per R8.3 — side table, not
    /// parsed from the symbol string). Stable for the life of the
    /// strategy instance.
    pub base_asset: BTreeMap<Symbol, Asset>,
}

#[derive(Debug, thiserror::Error)]
pub enum UniverseError {
    #[error("universe is empty")]
    Empty,
    #[error("universe contains unknown symbol: {0}")]
    UnknownSymbol(Symbol),
    #[error("universe must contain at least 2 symbols for cross-sectional ranking, got {0}")]
    TooSmall(usize),
}
```

`SymbolSet` is `BTreeSet`-backed deliberately so iteration is
alphabetical without an explicit sort — the Q5 strategy-side filter
and the R12.2 `(venue_ts, symbol ASC)` interleave both rely on that
property.

### `MomentumStrategy` (R3, R4, R6, R7)

```rust
// crates/strategy/src/cross_sectional/momentum.rs (new)
use trading_core::{Bar, Decimal, Signal, Strategy, StrategyId, Symbol, Tick, Timestamp,
                   Universe, RiskError};

pub struct MomentumStrategy {
    id:                StrategyId,
    universe:          Universe,
    lookback_minutes:  u32,                 // R3.1 — n; default 60
    rebalance_minutes: u32,                 // R6.3 — default 60
    k_long:            u32,                 // R4.2 — default 3
    k_short:           u32,                 // R4.2 — must be 0 in v1 (Q3)
    vol_floor:         Decimal,             // R3.2 — default 1e-6
    drift_threshold:   Decimal,             // R6.2 — default 0.10
    exposure_cap:      Decimal,             // R5.1 — default 0.50

    // Per-symbol price ring buffers, sized to lookback_minutes + 1.
    // Allocated at construction; no allocation on the hot path.
    histories:         BTreeMap<Symbol, RingBuffer<Decimal>>,

    // Per-symbol latest score cache, recomputed on the symbol's bar
    // and read at rebalance time.
    scores:            BTreeMap<Symbol, Option<Decimal>>,

    // Last rebalance bar's close timestamp. None until first rebalance.
    last_rebalance_ts: Option<Timestamp>,

    // Content hash of (universe, lookback, rebalance, k_long, k_short,
    // vol_floor, drift_threshold, exposure_cap) — sha256 hex,
    // stable across runs. Same shape as v0.5 ComposedStrategy.hash.
    hash:              [u8; 32],
    source_path:       SmolStr,
}

impl Strategy for MomentumStrategy {
    fn id(&self) -> StrategyId { self.id.clone() }

    fn on_bar(&mut self, bar: &Bar) -> Vec<Signal> {
        // Q5 strategy-side filtering — out-of-universe bar is a no-op.
        if !self.universe.symbols.contains(&bar.symbol) {
            return Vec::new();
        }

        // 1. Push close into the symbol's ring buffer.
        if let Some(rb) = self.histories.get_mut(&bar.symbol) {
            rb.push(bar.close.get());
        }

        // 2. Recompute the symbol's score (R3) from its ring buffer.
        //    Returns None until ring buffer is fully warmed (R4.5).
        let score = features::cross_sectional::score_vol_adjusted_return(
            self.histories.get(&bar.symbol).expect("symbol in universe"),
            self.lookback_minutes,
            self.vol_floor,
        ).ok();
        self.scores.insert(bar.symbol.clone(), score);

        // 3. Decide whether this is a rebalance bar (R6.1).
        //    Triggered when `(bar.close_ts - last_rebalance_ts) >= rebalance_minutes`,
        //    OR on first warm bar after all symbols are warmed.
        if !self.is_rebalance_bar(bar) {
            return Vec::new();
        }

        // 4. Rank universe symbols by score (warmed only — R4.5),
        //    select top-K (R4), tie-break alphabetically (R4.4).
        let target_weights = self.compute_target_weights();

        // 5. Construct rebalance signals — one Signal per symbol that
        //    needs an action (open / close / size adjust above
        //    drift threshold per R6.2). All Signals share the same
        //    Decision parent_id so the audit ledger can attribute fills
        //    to the rebalance event.
        let signals = self.build_rebalance_signals(bar, &target_weights);

        self.last_rebalance_ts = Some(bar.close_ts);
        signals
    }

    fn on_tick(&mut self, _tick: &Tick) -> Vec<Signal> { Vec::new() }

    fn config_schema() -> serde_json::Value where Self: Sized {
        CrossSectionalMomentumConfig::json_schema()
    }
}

impl MomentumStrategy {
    /// Inherent method — registry does not consume this (Q5).
    /// Operator success reports + cockpit introspection only.
    pub fn universe(&self) -> &Universe { &self.universe }

    fn is_rebalance_bar(&self, bar: &Bar) -> bool {
        match self.last_rebalance_ts {
            None => self.all_warmed(),
            Some(prev) => {
                let elapsed = bar.close_ts.minutes_since(prev);
                elapsed >= i64::from(self.rebalance_minutes)
            }
        }
    }

    fn all_warmed(&self) -> bool {
        self.histories.values().all(|rb| rb.is_full())
    }

    fn compute_target_weights(&self) -> BTreeMap<Symbol, Decimal> {
        // selector::top_k_long iterates self.scores in alphabetical
        // order (BTreeMap), filters None / NaN warmup-incomplete
        // entries, sorts descending by score with alphabetical
        // tie-break, takes first k_long. Each gets weight
        // exposure_cap / k_long.
        crate::cross_sectional::selector::top_k_long(
            &self.scores,
            self.k_long,
            self.exposure_cap,
        )
    }

    fn build_rebalance_signals(
        &self,
        bar: &Bar,
        target_weights: &BTreeMap<Symbol, Decimal>,
    ) -> Vec<Signal> {
        // Iterates universe in alphabetical order (BTreeSet). For each
        // symbol decides Hold / Open / Close / Resize per R6.1, applies
        // the drift threshold per R6.2, emits one Signal per action.
        // Uses bar.close_ts for the Signal.ts so all rebalance children
        // share the same timestamp (audit reconciliation friendly).
        unimplemented!("see selector module")
    }
}
```

**Score module** (`features::cross_sectional`, R3):

```rust
// crates/features/src/cross_sectional.rs (new)
pub fn score_vol_adjusted_return(
    history: &RingBuffer<Decimal>,
    n: u32,
    vol_floor: Decimal,
) -> Result<Decimal, ScoreError> {
    if history.len() < (n as usize + 1) {
        return Err(ScoreError::InsufficientHistory);
    }
    let close_now = history.last().ok_or(ScoreError::Empty)?;
    let close_back = history.get_back(n as usize).ok_or(ScoreError::Empty)?;

    // Vol-adjusted log return (R3.1).
    let log_return = features::math::decimal_ln(close_now / close_back)?;

    // Realized vol = std of log returns over the same n-bar window (R3.2).
    let mut log_rets = Vec::with_capacity(n as usize);
    for i in 0..n as usize {
        let now  = history.get_back(i)
                          .ok_or(ScoreError::Empty)?;
        let prev = history.get_back(i + 1)
                          .ok_or(ScoreError::Empty)?;
        log_rets.push(features::math::decimal_ln(now / prev)?);
    }
    let realized_vol = decimal_std(&log_rets)?.max(vol_floor);

    Ok(log_return / realized_vol)
}
```

**Selector module** (`strategy::cross_sectional::selector`, R4):

```rust
pub fn top_k_long(
    scores: &BTreeMap<Symbol, Option<Decimal>>,
    k: u32,
    exposure_cap: Decimal,
) -> BTreeMap<Symbol, Decimal> {
    // 1. Filter out None / warmup-incomplete entries (R4.5).
    // 2. Sort descending by score, alphabetical tie-break (R4.4).
    //    Iterating a BTreeMap is alphabetical, so a stable sort by
    //    score (descending) preserves alphabetical tie-break.
    // 3. Take first k. Each weight = exposure_cap / k.
    let leg_weight = exposure_cap / Decimal::from(k);
    scores.iter()
        .filter_map(|(s, v)| v.map(|score| (s.clone(), score)))
        .sorted_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0))) // stable alpha tie-break
        .take(k as usize)
        .map(|(s, _)| (s, leg_weight))
        .collect()
}
```

**Allocation discipline:** identical to v0.5 — ring buffers are sized
at construction; `on_bar` performs no `Vec::push` on warmup-only bars.
Rebalance bars allocate at most `2 * universe.len()` `Signal`s (one
close + one open per universe symbol in worst case); this bound is
load-tested in V7 against the 5ms p99 budget.

### Vector-order sizer (R5)

```rust
// crates/risk/src/portfolio.rs (new)
use trading_core::{Decimal, Money, Order, Position, RiskError, RiskLimits, Symbol, Usdt};

/// One leg of a portfolio target.
#[derive(Debug, Clone)]
pub struct TargetLeg {
    pub symbol:        Symbol,
    pub target_weight: Decimal,    // 0 == close; otherwise in (0, exposure_cap]
    pub mark_price:    Price,
}

/// Diff one symbol's current position vs the target weight.
#[derive(Debug, Clone)]
enum LegAction {
    Open  { qty: Quantity, side: Side },        // Side::Buy in v1
    Close { qty: Quantity },                    // Side::Sell to flat
    Resize{ qty: Quantity, side: Side },        // size adjustment > drift threshold
    Hold,                                       // within drift threshold — no order
}

/// Compute, validate, and atomically accept-or-reject a portfolio
/// rebalance. R5.2: all-or-nothing — partial acceptance is rejected as
/// a design choice. Returns `Vec<Order>` ready for `exec` on success.
///
/// Inputs:
///   - `targets`:     desired weights per universe symbol (0 == close).
///   - `equity`:      current portfolio equity in USDT.
///   - `position_book`: current per-symbol position snapshot.
///   - `drift_threshold`: R6.2 — relative drift below which Hold (no order).
///   - `limits`:      RiskLimits including per-symbol cap and
///                    portfolio_exposure_cap.
///   - `strategy_id`: rebalance attribution.
///
/// Returns:
///   - `Ok(Vec<Order>)` — vector of validated orders, alphabetically
///     ordered by symbol (R12.5).
///   - `Err(RiskError::PortfolioExposureBreach)` — Σ proposed long
///     notional > equity * portfolio_exposure_cap. No orders fire.
///   - `Err(RiskError::PerSymbolExposureBreach)` — any leg violates
///     the existing v0 per-symbol cap.
pub fn size_portfolio_target(
    strategy_id: StrategyId,
    targets:     &BTreeMap<Symbol, Decimal>,
    equity:      Money<Usdt>,
    position_book: &PositionBook,
    drift_threshold: Decimal,
    limits:      &RiskLimits,
) -> Result<Vec<Order>, RiskError>;
```

**Validation order** (mirrors v0 `Order::new` invariants):

1. Per-leg: compute `LegAction` from `(current_position, target_weight,
   mark_price, drift_threshold)`. `Hold` legs are dropped.
2. Per-leg: each non-Hold leg's notional ≤ `equity *
   limits.per_symbol_exposure_cap` (existing v0 invariant — preserved).
3. Aggregate: `Σ open_notional + Σ resize_notional` (longs only in v1
   per Q3) ≤ `equity * limits.portfolio_exposure_cap.unwrap_or(ONE)`.
4. If any check fails: emit no orders, return the appropriate
   `RiskError`. The agent then calls
   `audit::journal::rebalance_rejected(..)` (Q6) before returning to
   `on_bar`'s caller.
5. On success: construct each `Order` via existing `Order::new`,
   collect into `Vec<Order>` sorted alphabetically by symbol.

**`RiskLimits` extension** (additive in `trading_core`):

```rust
pub struct RiskLimits {
    pub per_symbol_exposure_cap: Decimal,        // existing v0 — 0.40 default
    pub max_drawdown_stop_pct:   Decimal,        // existing v0
    pub daily_loss_stop_pct:     Decimal,        // existing v0
    pub portfolio_exposure_cap:  Option<Decimal>, // NEW v1 — 0.50 default when set
}
```

When `portfolio_exposure_cap = None` the validator skips the aggregate
check (v0 backward-compat).

**Determinism** (R12.5): the input `targets` is a `BTreeMap`
(alphabetical). The output `Vec<Order>` is constructed in the same
iteration order. No HashMap, no `time::SystemTime::now()`, no RNG.

### Multi-symbol bar interleave (R2, R12)

**Sort key:** `(bar.venue_ts ASC, bar.symbol ASC)`. Symbol ordering is
the lexicographic order of `Symbol(SmolStr)` — `BTCUSDT < ETHUSDT <
LINKUSDT < ...`. R12.2 makes this load-bearing: at every minute
boundary all 10 symbols share `venue_ts`, and the secondary key fixes
the order across runs.

**`ReplayFeed::merge_symbols`** (in `crates/data/src/replay_feed.rs`,
new method):

```rust
impl ReplayFeed {
    /// Build a single deterministic bar stream from N per-symbol Parquet
    /// roots. Internally:
    ///   1. Spawn one `subscribe_bars(symbol, tf)` stream per symbol.
    ///   2. Use `futures::stream::select_all` to merge.
    ///   3. Buffer the front of each per-symbol stream; emit the bar
    ///      whose `(venue_ts, symbol)` is smallest. (k-way merge.)
    /// The implementation owns a small per-symbol head buffer so the
    /// merge is single-pass O(N_bars * log(N_symbols)). Not a
    /// performance bottleneck at v1 scale (10 symbols).
    pub async fn merge_symbols(
        &self,
        symbols: &[Symbol],
        tf:      Timeframe,
    ) -> Result<BoxStream<'static, Result<Bar, FeedError>>, FeedError>;
}
```

**Live `BinanceFeed::subscribe_bars_multi`**: at v1 the analyst's R2.5
"kline-only is sufficient" guidance lets us subscribe to 10 single-stream
WebSockets (one per symbol) and merge with the same k-way merge logic
on `(venue_ts, symbol)`. Alternatively, Binance's combined-stream
endpoint (`/ws/<sym1>@kline_1m/<sym2>@kline_1m/...`) can deliver all 10
on a single WS connection — developer's call. Either way the consumer
sees a `Stream<Item = Bar>` sorted by `(venue_ts, symbol)`.

**Determinism guard** (V5 / R12.2): the multi-symbol replay emits a
structured-log artifact (first 1000 events) containing
`(venue_ts, symbol)` tuples. Two runs at seed `0xC0FFEE` produce
identical artifacts. Asserted by a new
`crates/backtest/tests/multi_symbol_determinism.rs` integration test.

**Bus capacity scaling (R2.2):** `agent::Config::scale_bus_for_universe`
multiplies `bars_capacity` and `ticks_capacity` by `universe.len()`
when a multi-symbol strategy is loaded. Fixed multipliers in v1
(no auto-tuning). `fills_capacity` left at v0 default — fills are
bounded by rebalance cadence × K_long, well inside v0's 1024 capacity.

### Per-symbol P&L attribution (R8)

**Chart of accounts extension:** the existing 13-account chart
(`assets:position:BTC`, etc.) is parameterized; v1 universe load adds
nine new accounts on first bootstrap:

```
assets:position:ETH
assets:position:BNB
assets:position:SOL
assets:position:XRP
assets:position:ADA
assets:position:DOGE
assets:position:AVAX
assets:position:DOT
assets:position:LINK
assets:position_mark:ETH       # nine more mark-to-market contras
assets:position_mark:BNB
... (per universe)
```

**No SQL migration required** — `chart_of_accounts()` in
`audit::bootstrap` is idempotent (`INSERT OR IGNORE`). v1's
modification is a one-line extension that iterates the universe at
agent startup and seeds any missing rows. The reconciler's
`global_debit_credit_sum` query already filters by account-name pattern
so adding new accounts does not perturb invariants.

**`pnl_by_symbol` reader** (in `crates/audit/src/query.rs`, new):

```rust
pub async fn pnl_by_symbol(
    ledger: &Ledger,
    since:  Timestamp,
    until:  Timestamp,
) -> Result<Vec<(Symbol, Money<Usdt>)>, LedgerError>;
```

Implementation: SQL aggregation over `journal_entries` joined to a
per-fill `symbol` lookup (the `Fill.symbol` is captured in the
`metadata` JSON column today; v1 either promotes it to a real column
via a small migration **or** parses the JSON in the query — developer's
call, but the metadata-JSON path is sufficient at v1 scale and avoids
a migration). Returns one `(Symbol, Money<Usdt>)` per symbol with
non-zero realized P&L in `[since, until)`, sorted alphabetically.

**Sum-equals-scalar invariant (R8.5):**
`Σ pnl_by_symbol(since, until)` exactly equals
`realized_pnl_since(since)` evaluated at `until`. Verified by a new
property test in `crates/audit/tests/pnl_by_symbol.rs`:
generates random fill sequences across 10 symbols, asserts the sum
invariant holds to the satoshi.

### `RebalanceRejected` audit surface (R6.5, Q6)

Per [architecture.md → v1 Q6](../architecture.md#v1-q6--rebalancerejected-ledger-surface-extend-strategy_events):
new `kind = "rebalance_rejected"` value on the existing
`strategy_events.kind` column. **No SQL migration.**

**Writer** (in `crates/audit/src/journal.rs`, new helper):

```rust
pub async fn rebalance_rejected(
    ledger:      &Ledger,
    strategy_id: StrategyId,
    error_code:  &str,                  // e.g. "portfolio_exposure_breach"
    error_summary: &str,                // e.g. "proposed 0.55 > cap 0.50"
    ts:          Timestamp,
) -> Result<(), LedgerError>;
```

Internally builds a `StrategyEventWrite` with
`kind = StrategyEventKind::RebalanceRejected`, `error_code`,
`error_summary`, `strategy_id`, `ts`. `old_hash` / `new_hash` are
`None` (no swap is happening).

**Reader:** `audit::query::strategy_history(id)` returns all events
including rebalance rejections; the caller filters on `kind` if needed.
No new reader method required.

**Reconciler invariant unchanged:** `strategy_events` rows carry no
money; the v0 `journal_entries` reconciliation
(`Σ debits == Σ credits`) is unaffected.

### Funding-rate observation-only ingest (Q2)

Per [architecture.md → v1 Q2](../architecture.md#v1-q2--funding-rate-ingest-observation-only-at-v1):
v1 ships the path; `MomentumStrategy` does **not** consume it.

**Crate map:**

- `crates/data/src/funding.rs` — `FundingPoller` task: REST GET
  `https://fapi.binance.com/fapi/v1/premiumIndex` (USDT perp funding
  rate + next-funding ts) once per hour per symbol; emits
  `FundingObs` on each successful poll; persists to `funding_rates`
  table inside the same SQLite ledger.
- `crates/audit/migrations/003_funding_rates.sql` — schema as in
  architecture.md Q2.
- `crates/agent/src/bus.rs` — `funding_obs` broadcast channel
  (capacity 32, lagged-drop + log backpressure matches the v0
  pattern).
- `crates/audit/src/query.rs` — `funding_rate_history(symbol, since)`
  reader.

**Wiring in agent:** the orchestrator spawns
`agent::funding_poller_task(universe, ledger, bus, cancel)` alongside
the v0 kill-switch and v0.5 strategy-watcher tasks. Mode gating: runs
in `paper` and `research`; in `research` mode the poller is replaced
by `agent::funding_replay_task` that reads pre-recorded `funding_rates`
rows from a backtest fixture (so backtests stay deterministic and do
not hit the network).

**The `MomentumStrategy` does not subscribe** to `funding_obs`. The
channel exists for v1.5+ strategies and the (future) cockpit funding
column.

### TOML schema for v1 strategy config (R7.5)

```toml
# config/strategies/top10_momentum_h1.toml
id     = "top10_momentum_h1"               # MUST equal filename stem
kind   = "cross_sectional_momentum"        # NEW v1 discriminator
stage  = "research"                        # research | paper

# v1 universe — exactly 10 symbols for default v1; min 2, max 32 enforced
# at parse time. Re-validated against MarketDataSource::exchange_info.
universe = [
    "BTCUSDT", "ETHUSDT", "BNBUSDT", "SOLUSDT", "XRPUSDT",
    "ADAUSDT", "DOGEUSDT", "AVAXUSDT", "DOTUSDT", "LINKUSDT",
]

# Score / rebalance knobs (R3, R6).
lookback_minutes  = 60
rebalance_minutes = 60
vol_floor         = "0.000001"             # Decimal as TEXT — R3.4 no f64
drift_rebalance_threshold = "0.10"

# Top-K selection (R4).
k_long  = 3
k_short = 0                                 # MUST be 0 in v1 (Q3)

# Sizing (R5).
size         = "equal_weight"               # NEW v1 sizing kind
exposure_cap = "0.50"                       # per-strategy portfolio cap (R5.1)
```

**Serde struct** (in `strategy::cross_sectional::config`):

```rust
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CrossSectionalMomentumConfig {
    pub id:                        SmolStr,
    pub kind:                      StrategyKind,    // = CrossSectionalMomentum
    pub stage:                     Stage,
    pub universe:                  Vec<SmolStr>,    // 2..=32 symbols
    pub lookback_minutes:          u32,             // ≥ 2
    pub rebalance_minutes:         u32,             // ≥ 1
    #[serde(default = "default_vol_floor")]
    pub vol_floor:                 Decimal,
    #[serde(default = "default_drift")]
    pub drift_rebalance_threshold: Decimal,
    pub k_long:                    u32,             // ≥ 1
    #[serde(default)]
    pub k_short:                   u32,             // MUST be 0 in v1
    pub size:                      SizingKind,      // = EqualWeight
    pub exposure_cap:              Decimal,         // (0, 1]
}
```

**Validation rules** (typecheck path, mirrors v0.5 error-code pattern):

| code                          | cause                                                            |
|-------------------------------|------------------------------------------------------------------|
| `invalid_universe`            | `< 2` symbols, `> 32` symbols, or duplicate symbols              |
| `unknown_symbol`              | symbol not present in `MarketDataSource::exchange_info`         |
| `invalid_lookback`            | `lookback_minutes < 2`                                          |
| `invalid_rebalance`           | `rebalance_minutes < 1`                                         |
| `invalid_k_long`              | `k_long < 1` or `k_long > universe.len()`                       |
| `unsupported_short_sizing`    | `k_short > 0` (Q3 — v1 ships long-only)                          |
| `invalid_exposure_cap`        | `exposure_cap` not in `(0, 1]`                                  |
| `invalid_drift_threshold`     | drift threshold not in `[0, 1]`                                 |
| `unsupported_sizing`          | `size` not in `{ "equal_weight" }` for v1                        |
| `unsupported_kind`            | `kind` not in supported set (additive to v0.5 error table)       |

Errors flow through the same `StrategyLoadError` (v0.5 R2.5) +
`strategy_events.kind = "Reject"` path. **No new error-handling
plumbing.**

**Hot-swap (R7.7):** identical to v0.5 — the file watcher reloads,
parses, typechecks, constructs a new `MomentumStrategy`, swaps
atomically. The `strategy_events` table records the swap with the new
content hash. Per-symbol ring buffers reset on swap (the new strategy
re-warms from bar 0). Open positions persist (per v0.5 R3.3).

### Performance plan (R10.4, V7)

Per-bar work scales **O(N_symbols)** for the ring-buffer push +
score recompute on the symbol's own bar (constant per bar, `N` bars
per minute boundary). Rebalance work scales **O(N_symbols · lookback)**
amortized over the rebalance interval (60 minutes × 10 symbols × 60
score samples = 36 000 cell touches per hour, ~600 per minute).

**Budget table** (one-paragraph, derived from
[architecture.md → Performance budget](../architecture.md#performance-budget)
and v0/v0.5 measured baselines):

| Path                                          | Budget        | v1 expectation                                    |
|-----------------------------------------------|---------------|---------------------------------------------------|
| Bar-close → signal (no rebalance, 1 strategy) | < 5 ms p99    | ~50 µs at v1 scale (1 ring push + 1 score recomp) |
| Bar-close → signal (rebalance bar)            | < 5 ms p99    | ~1.5 ms p99 estimate (sort 10 scores, build vec)  |
| `size_portfolio_target` (10-leg target)       | < 1 ms p99    | ~200 µs estimate (per-leg validate + aggregate)   |
| Backtest throughput (10 symbols, 1 thread)    | > 10k bars/symbol/s | meets the v0 single-symbol budget divided 10× |

Bench home: `crates/strategy/benches/cross_sectional.rs`. Three cases:
warmup-only bar (no rebalance), rebalance bar with no diff (all hold),
rebalance bar with full rotation (close 3 + open 3). Criterion
baseline committed to
`criterion_baselines/v1-cross-sectional-momentum/`. Regression > 10%
fails the bench step (V7).

### Determinism plan (R12)

| Source of non-determinism (potential)            | Mitigation                                                                                                  |
|--------------------------------------------------|-------------------------------------------------------------------------------------------------------------|
| `HashMap` iteration                              | All universe-keyed maps are `BTreeMap` / `BTreeSet` (alphabetical, R12.2 / R12.4)                            |
| `f64` `log` / `sqrt` platform variance           | `features::math::decimal_ln` / `decimal_sqrt` — Decimal-only, fixed precision (R3.4 / R3.5; 10 dp pinned)   |
| Multi-symbol stream merge order                  | k-way merge with `(venue_ts, symbol)` key (R2 / R12.2); replay artifact diffed across runs (V5)              |
| Vector-order sizing iteration                    | Input `BTreeMap` → output `Vec<Order>` sorted alphabetically (R12.5)                                         |
| Rebalance trigger jitter                         | Driven off `bar.close_ts` arithmetic only — no `SystemTime::now()`                                          |
| Funding poller wall-clock                        | Disabled in `research` mode; replaced with `funding_replay_task` reading a pre-recorded fixture              |

### Test strategy

| Layer                          | Tests                                                                                                                                    | Crate(s)          | Tool        |
|--------------------------------|------------------------------------------------------------------------------------------------------------------------------------------|-------------------|-------------|
| **Unit — universe**            | `Universe::new` rejects empty / size-1 / duplicates; `SymbolSet::iter` order is alphabetical                                             | `trading_core`    | `cargo test` |
| **Unit — score**               | Hand-computed expected scores for 200-bar synthetic series; tolerance `Decimal::new(1, 9)`                                                | `features`        | `cargo test` |
| **Unit — decimal_ln/sqrt**     | Reference values to 10 dp; deterministic across runs                                                                                     | `features::math`  | `cargo test` |
| **Unit — selector**            | Top-K with synthetic scores; tie-break alphabetical; warmup exclusion                                                                    | `strategy`        | `cargo test` |
| **Unit — sizer**               | Vector accept (3 legs at 0.45 equity); reject at 0.55 with `PortfolioExposureBreach`; per-symbol cap binds at K=1                       | `risk`            | `cargo test` |
| **Property — score**           | Strictly increasing close → monotonically increasing score given fixed vol; non-negative under positive vol                              | `features`        | `proptest`   |
| **Property — sizer**           | No accepted vector ever leaves portfolio total over the cap, across 1 000 random target / equity / price tuples                          | `risk`            | `proptest`   |
| **Property — pnl_by_symbol**   | `Σ pnl_by_symbol(since, until) == realized_pnl_since(since) at until` across random fill sequences over 10 symbols                       | `audit`           | `proptest`   |
| **Integration — multi-symbol replay** | 10-symbol fixture, 1h window, 600 bars: every bar arrives, alphabetical interleave at minute boundaries, `<64 MiB` memory high-water | `data`, `backtest`| `cargo test` |
| **Integration — rebalance cadence** | Replay over 60 min (1 rebalance): exactly 1 `Decision`, `K_long` `Order` rows, no orders on the 59 non-rebalance bars                   | `agent`           | `cargo test` |
| **Integration — rebalance reject** | Synthetic over-leveraged universe → `RebalanceRejected` strategy_event row, zero `Order` rows                                            | `agent`, `audit`  | `cargo test` |
| **Integration — hot-swap**     | Edit `top10_momentum_h1.toml` mid-replay (new `lookback_minutes`); swap within 2s; new hash in `strategy_events`; next rebalance uses new lookback | `agent`         | `cargo test` |
| **Integration — funding ingest** | Mock REST endpoint; one poll per hour produces one `FundingObs` event + one `funding_rates` row; `audit::query::funding_rate_history` returns it | `data`, `audit` | `cargo test` |
| **Determinism (V5)**           | Two runs of `top10-2023-1h-momentum` at seed `0xC0FFEE` produce byte-identical reports; first 1000 merged events identical line-for-line | `backtest`        | `cargo test` |
| **Snapshot — report**          | Per-symbol metrics section format stable across runs                                                                                    | `backtest`        | `insta`      |
| **Bench (V7)**                 | `MomentumStrategy::on_bar` p99 < 5ms at 10 symbols; multi-symbol backtest > 10k bars/symbol/s                                            | `strategy`, `backtest` | `criterion` |
| **Regression (V9)**            | All v0 + v0.5 backtest scenarios produce byte-identical reports — anchor SHA stays at `fc2e3b4a…` for `btc-2023-1m-sma-cross`            | `backtest`        | `cargo test` |
| **Snapshot — UI (V8)**         | Cockpit fixtures with 3-position roster renders three rows; strategies panel shows one v1 row                                            | `ui`              | `insta`      |

## Implementation — v1 backend

### Crates / modules landed

| Crate | Module / file | What was added |
|---|---|---|
| `trading_core` | `universe.rs`, `funding.rs` | `Universe`, `SymbolSet`, `FundingObs`, `RebalanceRejected`, `RiskLimits.portfolio_exposure_cap` |
| `features` | `math.rs` | `decimal_ln`, `decimal_sqrt` (10 dp precision) |
| `features` | `cross_sectional.rs` | `score_vol_adjusted_return` — `InsufficientHistory` on empty buffer |
| `strategy` | `cross_sectional/momentum.rs` | `MomentumStrategy::on_bar` — 200-bar warmup + rebalance; K=3 deterministic alphabetical output |
| `strategy` | `cross_sectional/selector.rs` | `top_k_long` — 8 tests; alphabetical tie-break, warmup exclusion |
| `strategy` | `cross_sectional/config.rs` | `CrossSectionalMomentumConfig` TOML schema; `from_str` parser |
| `risk` | `portfolio.rs` | `size_portfolio_target` — per-symbol cap + portfolio cap; 3-leg accept/reject + proptest(1000) |
| `audit` | `journal.rs` | `rebalance_rejected` write path; integration test (kind/fields; `ledger_imbalance=0`) |
| `audit` | `query.rs` | `pnl_by_symbol` — 50-fill integration test; Σ invariant; proptest(200) |
| `audit` | `bootstrap.rs` | `seed_universe_accounts` — idempotent; restart no-op |
| `data` | `replay.rs` | `ReplayFeed::merge_symbols`, `merge_synthetic` — alphabetical interleave, monotonic ts, memory bound |
| `data` | `funding.rs` | `FundingPoller` struct + `BinanceFundingClient`; `funding_obs` EventBus channel wired |
| `agent` | `watcher.rs` | `MomentumStrategy` loaded; hot-swap on `top10_momentum_h1.toml` change |
| `backtest` | `main.rs` | `write_momentum_report`; `merge_synthetic` path; 10 independent `ChaCha20Rng` streams |
| `config` | `strategies/top10_momentum_h1.toml` | v1 strategy config; `content_hash d41f391...` |
| `ui` | `widgets/strategies.rs` | `Reject \| RebalanceRejected` merged match arm |

### New dependencies

No new external crates added. `ChaCha20Rng` is from existing `rand_chacha` workspace dep (T616).

### Deviations from Design

| Deviation | Design intent | Actual |
|---|---|---|
| T612 — multi-symbol live BinanceFeed | Combined-stream WS endpoint or N per-symbol connections | Single-symbol WS only; multi-symbol live not implemented |
| T613 — FundingPoller integration test | Mock-REST test + `003_funding_rates.sql` migration + `funding_rate_history` query | Struct + channel wired; test/migration/query absent |
| T614 — funding_poller_task orchestration | `funding_poller_task` spawned in `main.rs`; "funding_poller started" log line | Not spawned; agent boots without live funding data |
| T621 — criterion bench runtime budget | `MomentumStrategy::on_bar` p99 < 5ms measured | `--no-run` build gate only; runtime not measured |
| Determinism fix | `write_momentum_report` body to be stable across runs | Initial implementation included `Wall-clock time` row in body (elapsed varied: 4.3s vs 4.2s); fixed by moving timing to YAML front-matter only |

### Phase 1 task audit (T601–T_FINAL_A_v1)

| Task | Status | Summary |
|---|---|---|
| T601 | PASS | `cargo test -p trading_core` — 23 tests |
| T602 | PASS | `features::math` — `decimal_ln`, `decimal_sqrt` precision verified |
| T603 | PASS | `features::cross_sectional` — 4 tests + proptest |
| T604 | PASS | `strategy::cross_sectional::selector::top_k_long` — 8 tests |
| T605 | PASS | 11 bad-fixture tests pass |
| T606 | PASS | `MomentumStrategy::on_bar` — warmup, K=3, alphabetical |
| T607 | PASS | `risk::size_portfolio_target` — accept/reject + proptest(1000) |
| T608 | PASS | `audit::journal::rebalance_rejected` integration test |
| T609 | PASS | `audit::query::pnl_by_symbol` — Σ invariant + proptest(200) |
| T610 | PASS | `audit::bootstrap::seed_universe_accounts` — idempotent |
| T611 | PASS | `data::ReplayFeed::merge_symbols` + `merge_synthetic` |
| T612 | FAIL | Multi-symbol live WS; per-symbol `clock_skew_ms` label; testnet — all absent — deferred to v1.5 |
| T613 | PASS | mock-REST integration test (wiremock, 3 tests); `insert_funding_obs` + `funding_rate_history` added (6 audit tests) |
| T614 | PASS | Poller spawned in `main.rs`; `FundingConfig` default-off; `funding_poller_started` / `funding_poller_disabled` boot logs |
| T615 | PASS | `top10_momentum_h1.toml` parses; content hash verified |
| T616 | PASS | 10 `ChaCha20Rng` streams seeded from master seed documented |
| T617 | PASS | Both v1 scenarios run end-to-end; exit 0; `ledger_imbalance=0` |
| T618 | PASS | `multi_symbol_determinism` 5/5; alphabetical merge order |
| T619 | PASS | `v1_hot_swap` 4/4; new content hash in strategy_events |
| T620 | PASS | `v1_rebalance_reject` 3/3; `portfolio_exposure_breach`; `ledger_imbalance=0` |
| T621 | PASS | `--no-run` build gate; runtime budget deferred |
| T622 | PASS | All 5 v0/v0.5 anchors verified |
| T_FINAL_A_v1 | PASS | All criteria met; 7 anchor hashes preserved; 306 tests green |

**Ticked: 22/23.** T612 deferred to v1.5 (multi-symbol live BinanceFeed). T613 + T614 + T_FINAL_A_v1 complete.

### v0/v0.5 anchor regression gate (T622)

All 5 v0/v0.5 scenarios produce exact locked body-SHA256 hashes. No regression.

| Scenario | Body SHA-256 |
|---|---|
| `btc-2023-1m-sma-cross` | `fc2e3b4a04055e60209fe85541173aa8883df226d2756352dfd101597168649c` |
| `btc-2023-1m-sma-baseline-refresh` | `fc2e3b4a04055e60209fe85541173aa8883df226d2756352dfd101597168649c` |
| `btc-2023-1m-macd-trend` | `ef9c5e483fa079f670a7aa15671643fce3b39a5ce35df8cb6d797887053f8805` |
| `btc-2023-1m-rsi-reversion` | `bc56d20d608c680e534bf6764ce8e0e568f0d4ffdf847a539c53fef65170d7aa` |
| `btc-2023-1m-bbands-mean-revert` | `d8a08a23d3629556c5fca39d6af89d7e0f99418e642af0b86fce22ff4d2792e3` |

### v1 backtest scenario hashes

Data source: **synthetic (seeded RNG, v1 multi-symbol)** — 10 independent `ChaCha20Rng`
streams, each seeded from `master_seed + idx * 0x9E3779B9`. Determinism verified: each
scenario run twice at seed `0xC0FFEE`, body hashes identical.

| Scenario | Body SHA-256 | Final equity | Trades | Ledger imbalance |
|---|---|---|---|---|
| `top10-2023-1h-momentum` | `3b60ef0743f006867b9e52f9de154869ee170987b27560e288b2d9597d3ecf97` | $56,282.81 USDT | 4,809 | 0 |
| `top10-2024-h1-momentum` | `1f33534fc7c6af1c04330564bec77aac620ecf6f1058f11ff90dfb66adcf05c6` | $46,401.41 USDT | 2,490 | 0 |

Reports written to:
- `spec/reports/backtest-20260429-195148-top10-2023-1h-momentum.md`
- `spec/reports/backtest-20260429-195243-top10-2024-h1-momentum.md`

### Build and test stats

| Gate | Result |
|---|---|
| `cargo fmt --all -- --check` | PASS (formatting fixed before gate) |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | PASS (11 issues found and fixed) |
| `cargo check --workspace --all-targets` | PASS |
| `cargo test --workspace --all-targets` | PASS — 306 tests (9 new: 3 data + 6 audit) |
| `cargo test --workspace --doc` | PASS |
| `cargo test -p trading_core --test trybuild` | PASS |
| `cargo test -p audit` | PASS |
| `cargo build --workspace --release` | PASS — 13s |

Closeout notes: [spec/reports/dev-v1-closeout-notes-2026-04-29.md](../reports/dev-v1-closeout-notes-2026-04-29.md)

### v1 funding-poller close (T613 + T614 + T_FINAL_A_v1)

**Completed 2026-04-29.**

#### What landed

**T613 — FundingPoller integration test + migration + query:**

- `audit::journal::insert_funding_obs(&Ledger, &FundingObs) -> Result<()>` — appends
  a row to `funding_rates` table (not a double-entry entry; reconciler invariant unaffected).
- `audit::query::funding_rate_history(ledger, symbol, since, until) -> Result<Vec<FundingObs>>` —
  read-only, no `sqlx` types in return; sorted by `funding_ts ASC`.
- `FundingPoller::poll_once_for_test()` — public test helper exposing one poll cycle
  so tests drive timing without depending on wall-clock sleep.
- `crates/data/tests/funding_poller_integration.rs` — 3 wiremock-backed tests:
  - `t613_poll_three_symbols_persists_rows` — happy path: mock server returns canned
    `premiumIndex` JSON; 3 `FundingObs` events broadcast; 3 rows persisted; `funding_rate_history`
    returns them.
  - `t613_poller_skips_on_connection_refused` — server dropped before poll; poller skips
    gracefully (no panic).
  - `t613_poller_skips_on_5xx` — server returns 500; poller skips gracefully.
- `crates/audit/tests/funding_rate_history_test.rs` — 6 tests: table existence,
  chronological order, symbol exclusion, window filter, empty result, ledger balance invariant.
- `003_funding_rates.sql` migration verified — `CREATE TABLE IF NOT EXISTS funding_rates`
  with correct columns and two indices; applies cleanly on a fresh in-memory DB.
- `wiremock = "0.6.2"` added as workspace dev-dependency (pinned exact version; no new runtime dep).

**T614 — Spawn the poller in agent main:**

- `FundingConfig` struct added to `agent::config` with fields: `enabled: bool` (default
  `false`), `interval_secs: u64` (default 3600), `universe: Vec<String>` (default empty).
- `[funding]` section added to `config/agent.toml` — `enabled = false` by default with a
  comment explaining the default-off policy.
- `EventBus::funding_obs_sender()` — new method returning a `broadcast::Sender<FundingObs>`
  clone for direct handoff to the poller task.
- `crates/agent/src/main.rs` — after the strategy watcher spawn:
  - If `cfg.funding.enabled`: constructs `FundingPoller` from config, spawns it with
    `tokio::spawn` + `CancellationToken`; spawns a persistence sidecar that subscribes to
    `funding_obs` and calls `audit::journal::insert_funding_obs`; logs
    `INFO funding_poller_started universe_size=N`.
  - If disabled: logs `INFO funding_poller_disabled`.
  - Non-essential: poller and persistence sidecar panic do NOT crash the agent.
  - `CancellationToken` is held in `_funding_cancel` which is dropped on agent shutdown,
    cancelling both tasks.

#### Mock test pattern

The integration test implements `FundingRestClient` for a `MockRestClient` struct that points
at the wiremock `MockServer` base URL. This avoids the `BinanceFundingClient` hard-coded
`fapi.binance.com` URL without modifying production code. The pattern follows the
"all external I/O behind a trait" rule from the coding standards.

#### Config-disable default

`funding.enabled = false` in `config/agent.toml` means:
- All tests and CI runs never hit Binance fapi.
- Research-mode backtests are unaffected.
- Setting `enabled = true` in a paper-mode deployment activates the hourly REST poller.

#### 7 anchor hashes preserved

All 7 hashes verified green after T613 + T614 landed (no shared code paths touched):

| Scenario | Body SHA-256 |
|---|---|
| `btc-2023-1m-sma-cross` | `fc2e3b4a04055e60209fe85541173aa8883df226d2756352dfd101597168649c` |
| `btc-2023-1m-sma-baseline-refresh` | `fc2e3b4a04055e60209fe85541173aa8883df226d2756352dfd101597168649c` |
| `btc-2023-1m-macd-trend` | `ef9c5e483fa079f670a7aa15671643fce3b39a5ce35df8cb6d797887053f8805` |
| `btc-2023-1m-rsi-reversion` | `bc56d20d608c680e534bf6764ce8e0e568f0d4ffdf847a539c53fef65170d7aa` |
| `btc-2023-1m-bbands-mean-revert` | `d8a08a23d3629556c5fca39d6af89d7e0f99418e642af0b86fce22ff4d2792e3` |
| `top10-2023-1h-momentum` | `3b60ef0743f006867b9e52f9de154869ee170987b27560e288b2d9597d3ecf97` |
| `top10-2024-h1-momentum` | `1f33534fc7c6af1c04330564bec77aac620ecf6f1058f11ff90dfb66adcf05c6` |

#### T612 status

T612 (multi-symbol live BinanceFeed) remains `[ ]`. Note in tasks file:
**deferred to v1.5** — single-symbol WS only; no per-symbol `clock_skew_ms{feed,symbol}` label;
no testnet smoke test.

## Verification

The tester's contract for declaring v1 done. All items must be green
before a `VERDICT → PASS` can be issued. Mapping to R-numbered
requirements is explicit so the tester's report can cross-reference.

- **V1 Static checks pass.** `cargo fmt --check` clean,
  `cargo clippy --workspace --all-targets -- -D warnings` clean
  (including the v0 R2.2 / R2.3 deny lints), `cargo audit` shows no
  unpatched advisories, `cargo deny check` (bans, licenses, sources)
  passes. Maps to v0 V1.
- **V2 `cargo test --workspace` green.** Zero failures, zero
  unexplained `#[ignore]`. Includes:
  - R1 universe-load unit + property tests.
  - R2 multi-symbol replay integration test (10-symbol fixture, 1h).
  - R3 momentum-score unit + property tests, `decimal_ln` /
    `decimal_sqrt` precision tests.
  - R4 top-N selector unit tests (tie-break, warmup exclusion).
  - R5 vector `risk::size_and_validate` unit + property tests.
  - R6 rebalance cadence integration test (hourly trigger, drift
    threshold, all-or-nothing rejection).
  - R7 strategy load + hot-swap integration test (new
    `kind = "cross_sectional_momentum"` discriminator).
  - R8 `pnl_by_symbol` unit test (sum-equals-scalar invariant) +
    integration over a full backtest tail.
  - R12 determinism tests (alphabetical interleave, score warmup
    boundaries).
- **V3 Both backtest scenarios run end-to-end.**
  - `top10-2023-1h-momentum` produces
    `spec/reports/backtest-<stamp>-top10-2023-1h-momentum.md`
    conforming to the v0/v0.5 report template plus the new
    per-symbol summary section (R9.3).
  - `top10-2024-h1-momentum` produces
    `spec/reports/backtest-<stamp>-top10-2024-h1-momentum.md`
    with Scenario 1 listed as baseline.
  - Both reports include metrics: Total return, CAGR, Sharpe,
    Sortino, Max drawdown, Hit rate, Turnover, Trades, Avg trade
    P&L, **per-symbol metrics rows** (10 rows in the v1 universe).
- **V4 Per-symbol P&L attribution (R8).** The post-Scenario-1
  `audit::query::pnl_by_symbol(2023-01-01, 2024-01-01)` returns up
  to 10 rows; `Σ rows` exactly equals
  `realized_pnl_since(2023-01-01)` at the end timestamp;
  alphabetical ordering verified.
- **V5 Determinism (R12).** `top10-2023-1h-momentum` runs twice
  at seed `0xC0FFEE`; both reports have identical body-SHA256.
  The merged-event-stream order is captured in a structured-log
  artifact (first 1000 events per run) and is identical line-for-line
  across the two runs — confirms `(venue_ts ASC, symbol ASC)`
  determinism is real, not a coincidence.
- **V6 Multi-symbol interleave smoke (R2 + R12).** Tester runs the
  10-symbol replay over a 1-hour fixture and inspects the structured
  log: at every minute boundary, exactly 10 bars surface (one per
  symbol), in alphabetical order. No bar arrives out of `venue_ts`.
- **V7 Performance budget (R10).**
  - `cargo bench -p strategy --bench cross_sectional` shows
    `MomentumStrategy::on_bar` p99 latency `< 5ms` even at the v1
    universe size (10 symbols). Same `< 5ms p99` budget the v0/v0.5
    benches enforce per
    [architecture.md → Performance budget](../architecture.md#performance-budget).
  - `cargo bench -p backtest` shows multi-symbol throughput
    `> 100k bars/s` divided across 10 symbols (i.e.
    `> 10k bars/symbol/s` aggregated; the v0 single-symbol target
    is `> 100k bars/s`, which a 10-symbol run divides into).
- **V8 Cockpit positions panel smoke (R11).**
  - `cargo run --bin cockpit --features fixtures` against a
    preloaded 3-position roster renders three rows. Manual smoke
    by the operator (capture screenshot under
    `spec/reports/screenshots/v1-cross-sectional-momentum/` per
    the v0 pattern in
    [spec/reports/screenshots/v0-paper-sma/README.md](../reports/screenshots/v0-paper-sma/README.md)).
  - Strategies panel (v0.5 R5) shows one row for the v1 strategy;
    `Holds position = yes`; `Signals / 60s = 3` immediately after
    a rebalance bar.
- **V9 v0 + v0.5 regression-free.** All v0 + v0.5 test reports
  pass at the locked anchor hashes. Specifically:
  - `btc-2023-1m-sma-cross` body-SHA256 stays at
    `fc2e3b4a04055e60209fe85541173aa8883df226d2756352dfd101597168649c`
    per [v0 ship reference](../reports/screenshots/v0-paper-sma/README.md).
  - `btc-2023-1m-sma-baseline-refresh` matches the same anchor
    (it's the same code path).
  - `btc-2023-1m-macd-trend`, `btc-2023-1m-rsi-reversion`,
    `btc-2023-1m-bbands-mean-revert` body-SHA256s match the v0.5
    locked values (`ef9c5e48…`, `bc56d20d…`, `d8a08a23…`).
- **V10 Cost telemetry (R10).** v1 run's auto-generated
  `costs.md` shows `LLM tokens: $0.00`, `Total / month ≤ $135`
  (v1 ceiling); cost ledger accounts (`expense:llm:*`) still
  contain zero entries.
- **V11 Audit reconciliation (v0 R3.5).** Across the full v1 2023
  backtest, the minute-boundary reconciliation passes at every bar
  — `ledger_imbalance_total == 0`, zero `LedgerImbalance` events
  in the structured log, zero `RebalanceRejected` events except
  the deliberately-induced ones in R6 / R5 unit tests. Cross-symbol
  reconciliation: `cash + Σ_symbol(positions[symbol] × mark[symbol])
  = equity` holds for the multi-symbol position book.

Failure on any of V1–V11 routes per the v0 / v0.5 verdict-routing
contract:
- Static / test / bench failure → `developer`.
- UI regression → `ui-designer` (v1's UI is a negative
  confirmation per R11; a regression here is unexpected).
- Structural (crate layout, trait shape, registry, vector risk
  surface, multi-symbol bus pattern) → `architect`.
- Strategy / scenario hypothesis wrong (Scenario 1 prints
  Sharpe < -1 or > +1.5 — outside the analyst's defensible-range
  prior) → `analyst`.

## UI — v1

R11 is a **negative confirmation**: the v0 positions widget already
renders N rows; v1 ships **no widget code**. The ui-designer's tail
is fixtures + smoke + snapshot only.

### What landed

- **T623** — `ui::fixtures` v1 extension. New deterministic
  generators for the top-3 long momentum portfolio:
  `fake_v1_position_btc()` (`POS`), `fake_v1_position_eth()`
  (`NEG`), `fake_v1_position_sol()` (`FG_MUTED`),
  `fake_v1_three_symbol_portfolio()`,
  `fake_v1_strategy_row_momentum()` (id `top10_momentum_h1`),
  `fake_v1_recent_events()`, and
  `fake_cockpit_v1_steady_state()`. The cockpit binary's `boot()`
  switches its default fixture from
  `fake_cockpit_with_strategies()` to
  `fake_cockpit_v1_steady_state()` so `cargo run --bin cockpit
  --features fixtures` directly demos the v1 multi-row state. The
  v0/v0.5 fixtures are untouched and still callable from tests.
- **T_FINAL_B_v1** — V8 smoke. Appended `## v1 — multi-symbol
  positions smoke` to
  [`spec/reports/ui-week2-smoke-checklist-2026-04-18.md`](../reports/ui-week2-smoke-checklist-2026-04-18.md);
  added the new snapshot test
  `panel_snapshots__positions_v1_three_rows` that pins the
  three-row layout and the per-row color tokens; updated
  [`screenshots/v0-paper-sma/README.md` §4.2](../reports/screenshots/v0-paper-sma/README.md#42-positions--open-positions)
  to note the v1 up-to-3-rows steady state; queued the deferred
  PNG `screenshot-v1-positions-three-rows.png` for the operator
  capture.

### Strings added

- 0. The positions panel headers / empty / error / loading copy from
  v0 already cover the v1 multi-row case (R11); multi-row is data,
  not new copy. The `top10_momentum_h1` id is a fixture string
  (not user-facing chrome) and lives in `ui::fixtures`, not
  `ui::strings`.

### Theme tokens added

- 0. The v0 token set (`POS` / `NEG` / `FG_MUTED` / `FG` / `BG` /
  `BG_ELEV` / `ACCENT` / `WARN` / `BORDER`) already covers all
  three P&L sign branches the new fixture exercises. Per the
  three-goal contract, "near-zero" was the ship-target; v1
  achieved zero.

### Accessibility notes

- Keyboard / focus order unchanged from v0. The positions panel is
  read-only; tab order is governed by the cockpit container in
  `bin/cockpit.rs`, untouched in v1.
- Color is never the only signal — the `pnl` and `pnl_pct`
  numerals carry an explicit sign character (`+` / `-` / no
  prefix for zero) via `fmt_usdt_signed` / `fmt_pct`, so an
  operator with a color-vision deficiency can still tell the rows
  apart.
- Numbers stay right-aligned monospaced (per the v0 contract).
  The v1 fixture's three different magnitudes (BTC ~40k mark, ETH
  ~2.4k mark, SOL ~100 mark) deliberately stress that
  right-alignment so a future regression that switches to
  left-alignment is visually obvious in the snapshot diff.

### Consistency self-audit

- inline strings: 0 / inline hex: 0. `cargo test -p ui` still
  passes the
  `no_inline_user_visible_strings_in_widgets` and
  `no_inline_hex_colors_in_widgets_or_state` gates. No widget
  files were touched in v1; only `fixtures.rs`, `bin/cockpit.rs`,
  and the snapshot test.

### Test coverage delta

- default suite (`cargo test -p ui`): was 30 → now 31 panel
  snapshots (added `positions_v1_three_rows`). Total default-build
  test count (across the three test binaries) was 57 → now 58.
- live suite (`cargo test -p ui --features live`): unchanged at
  71 — v1 introduces no new live subscribers (R11.3 explicit:
  multi-symbol is a positions-bus payload change, not a new
  channel).

### Deferred manual

- `screenshot-v1-positions-three-rows.png` — operator captures via
  `cargo run --bin cockpit --features fixtures` on a desktop
  display (sandbox is headless). Capture instruction is in the
  smoke checklist's "Deferred PNG list — v0.5 additions" table
  (the v1 row is appended there to keep one canonical list).

## Changelog

- 2026-04-20 (analyst): initial brief.
- 2026-04-29 (developer): appended `## Implementation — v1 backend`; ticked 20/23 T6xx tasks; T612/T613/T614 deferred to next sprint (live ingest path); v1 scenario hashes locked; v0/v0.5 anchor regression PASS; 297 tests green; all quality gates PASS. Ownership transferred to developer; status remains `in-progress` pending T612–T614.
- 2026-04-29 (developer): T613 + T614 + T_FINAL_A_v1 completed; appended `### v1 funding-poller close` subsection; ticked 22/23 tasks; T612 deferred to v1.5; 306 tests green; 7 anchor hashes preserved.
- 2026-04-30 (ui-designer): T623 + T_FINAL_B_v1 completed — appended `## UI — v1` section above; ticked both UI tasks in the task list; v1 cockpit fixture (`fake_cockpit_v1_steady_state`) drives `cargo run --bin cockpit --features fixtures` to the multi-row positions steady state; new `panel_snapshots__positions_v1_three_rows` snapshot test pins the three-row layout (`POS` / `NEG` / `FG_MUTED`); zero new strings, zero new theme tokens (R11 negative confirmation honored); `screenshots/v0-paper-sma/README.md` §4.2 updated; smoke checklist appended at `spec/reports/ui-week2-smoke-checklist-2026-04-18.md`. UI test count default 57 → 58; live unchanged at 71. T612 stays `[ ]` deferred to v1.5.
- 2026-04-29 (architect): appended `## Design` section translating R1–R12
  into crate / module additions, traits, message types, TOML schema, and
  test strategy. Resolved the six open analyst questions in
  [architecture.md → v1 cross-sectional momentum resolutions](../architecture.md#v1--cross-sectional-momentum-resolutions-q1q6--confirmed-2026-04-29):
  Q1 L2 deferred to v1.5; Q2 funding observation-only at v1 (poller +
  table + `funding_obs` channel; not consumed by momentum); Q3 long-only
  confirmed (`K_long=3`, `K_short=0`); Q4 single-venue Binance for v1;
  Q5 strategy-side universe filtering (no trait change); Q6 extend
  `strategy_events.kind` with `rebalance_rejected` (no schema migration).
  No change to the v0 `Strategy` trait or v0.5 audit / broadcast
  surfaces. Status `draft → in-progress`; ownership shifts from
  analyst to architect for the duration of T6xx execution. Task list at
  [tasks/v1-cross-sectional-momentum.md](../tasks/v1-cross-sectional-momentum.md).

## Notes — open questions for architect

The analyst defers these architectural decisions to the architect.
The brief is written so each can be answered yes/no without
reshaping the requirements above.

1. **L2 book ingest at v1 vs deferred to v1.5?**
   [product.md → Universe & data fidelity ladder](../product.md#universe--data-fidelity-ladder)
   v1 entry says "1m + L2 + funding context, +Kraken". v0/v0.5
   shipped klines + trades only. The v1 momentum score (R3) does
   **not** consume L2 — it's a close-to-close vol-adjusted return.
   L2 ingest at v1 would be infrastructure for v1.5+ pairs / 1s
   aggregation and the v2 perp signal feed. Analyst's preference:
   **defer L2 to v1.5** to keep this scope tight; architect to
   confirm or push back.

2. **Funding-rate ingest at v1?**
   Same sentence as (1) calls out funding-rate context. v1's
   momentum score does not consume funding either. The reflection
   memory line in `## Why` mentions "long ETH on negative
   funding-flip" only as a v1+ memory richness example — it does
   not require the funding feed to land in v1. Analyst's preference:
   **defer funding ingest to v1.5** alongside L2; architect to
   confirm or push back.

3. **Long-only vs long-short via perps (R4 [ASSUMPTION] re-stated).**
   v1 ships `K_short = 0` because spot crypto has no native short
   mechanism. If the architect wants any kind of short-side signal
   in v1, it must reduce to "exclude these symbols from longs"
   rather than "open a short position". Perp-based shorting belongs
   in v2 per the universe ladder. Confirm.

4. **Multi-venue at v1 (`+Coinbase +Kraken`) or single-venue
   Binance only?**
   The v1 universe-ladder entry reads "Top-10 USDT spot, 1m + L2 +
   funding context, +Kraken" — interpreted strictly that's three
   venues (Binance from v0, Coinbase from v0.5 ladder, Kraken new
   in v1). v0 and v0.5 ship Binance-only — Coinbase ingest is also
   not yet built. Analyst's preference: **single-venue (Binance)
   for v1** to keep scope tight; multi-venue Coinbase + Kraken in
   v1.5 once cross-sectional momentum has a defensible number on
   one venue. The product.md ladder entry is then re-read as the
   roadmap goal for the v1 series, with v1 itself doing the
   universe-size work and v1.5 doing the venue-multiplexing work.
   Architect to confirm or push back; if multi-venue is required at
   v1, R2 grows substantially (per-venue feed adapters, cross-venue
   symbol reconciliation, venue-attribution in the ledger) and the
   tester has a much larger surface.

5. **`MomentumStrategy::interested_in(symbol)` shape (R7.4).**
   Analyst's preference is registry-side filtering (the strategy
   declares its universe; the registry forwards only in-universe
   bars). Alternative is strategy-side filtering. The analyst's
   preference keeps `Strategy::on_bar` callers stateless about
   universe membership; architect picks the cleaner extension to
   the v0.5 `StrategyRegistry::on_bar` fan-out.

6. **`RebalanceRejected` ledger surface (R6.5).** New row type:
   sibling to v0.5 `strategy_events`, or extension to it, or a
   third dedicated `decision_events` table? Analyst's preference
   is **extend `strategy_events` with a new `kind = "RebalanceRejected"`
   variant** because the existing table is already operator-event-shaped
   and carries `error_code` / `error_summary`; architect to decide
   whether the schema fits or wants a separate table for
   per-decision events.
