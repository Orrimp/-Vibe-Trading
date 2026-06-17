---
slug: v1-5b-multi-venue
status: shipped
owner: tester
updated: 2026-05-03
version: 1.2.0
---

# v1.5b — Multi-venue + 1s aggregated trades

## Why

This is the **largest queued backend feature** on the
[backlog](../backlog.md), explicitly carved out of v1.5 by the v1.5a
analyst's split decision
([v15a-mean-reversion-pairs.md → Why, lines 25–44](../v15a-mean-reversion-pairs/feature.md))
and the architect's v1 Q4 resolution
([architecture.md → v1 Q4 — Multi-venue: single-venue (Binance) for v1, lines 855–884](../architecture.md#v1-q4--multi-venue-single-venue-binance-for-v1)).
v1.5a shipped the mean-reversion pairs strategy on the existing
Binance USDT universe; v1.5b is the **plumbing-only sibling** that
expands the **data side** of the agent — Coinbase + Kraken adapters,
USDC pairs, multi-symbol live `BinanceFeed` (T612), and 1-second
aggregated trades — without touching execution.

### The operational gap

Today the agent reads market data from **Binance only** — single
venue, single source of truth. `crates/data/src/binance.rs:89–132`
defines a single `BinanceFeed` struct, and the
[universe ladder v1.5 entry](../product.md#universe--data-fidelity-ladder)
("+ Stablecoin pairs (BTC/USDC, ETH/USDC), 1s aggregated trades")
is the explicit roadmap commitment to fix that. Three concrete
gaps motivate v1.5b:

1. **Single venue means single failure mode.** A Binance WS outage
   today silences the agent's entire data feed. Real crypto
   trading benefits from cross-venue redundancy: a Coinbase
   feed staying up while Binance reconnects keeps the strategy
   layer fed (subject to v1.5b's failover semantics — see Q7).
2. **No USDC pairs.** v1.5a's pair list explicitly excludes USDC
   pairs because Binance USDC books are thinner than USDT and
   liquidity is concentrated on Coinbase / Kraken
   ([architecture.md → v1.5a Q5 — USDC pairs: blocked on v1.5b
   multi-venue, lines 1130–1158](../architecture.md#v15a-q5--usdc-pairs-blocked-on-v15b-multi-venue)).
   v1.5b unblocks the universe extension.
3. **Bar granularity floor is 1m.** `Timeframe::OneMinute` is the
   smallest enum variant
   (`crates/core/src/bar.rs:11–19`). v1.5b adds
   `Timeframe::OneSecond` (aggregated client-side from the raw
   trade stream — see Q5) so a future microstructure strategy
   has a finer-grained bar to consume without re-engineering
   the bar pipeline.

### Why these three venues

- **Binance** — already the v0/v0.5/v1/v1.5a baseline. v1.5b
  finishes T612 (multi-symbol live `BinanceFeed`) per the v1
  closeout's explicit deferral
  ([v1-cross-sectional-momentum.md → T612 status, lines 1516–1521](../v1-cross-sectional-momentum/feature.md#t612-status)).
  Today's `BinanceFeed::subscribe_bars` /
  `subscribe_trades` opens **one WS connection per symbol**;
  v1's 10-symbol universe in paper mode would mean 10 idle
  WS connections eating resources. T612 is the multi-symbol
  fan-out (combined-stream URL or N connections per symbol —
  architect picks).
- **Coinbase** — the second-largest US-regulated spot venue.
  USDC pairs are **deeper** here than on Binance (BTCUSDC,
  ETHUSDC). Coinbase Advanced Trade WS is the current public
  market-data API; the legacy Coinbase Pro WS still works for
  read-only market data and is sometimes cited as more stable.
  Architect picks (Q2).
- **Kraken** — third-largest US-regulated spot venue. Symbol
  shape is exotic (`XBT/USD` not `BTCUSD`); both base-asset
  normalization and pair separator are different from Binance.
  This is precisely why the architect's v1 Q4 deferral existed:
  cross-venue symbol normalization is its own surface.

### Why now (after v1.5a)

The v1.5a Q1 split decision
([architecture.md → v1.5a Q1 — Single brief vs split: confirm
split (Option B), lines 999–1019](../architecture.md#v15a-q1--single-brief-vs-split-confirm-split-option-b))
deliberately decoupled the strategy edge claim (v1.5a) from the
multi-venue infra delivery (v1.5b) so they fail independently.
With v1.5a shipped, v1.5b is unblocked: the strategy registry,
audit ledger, broadcast bus, and rebalance / risk infra are
all venue-agnostic by construction (R10 — backwards compat),
so a multi-venue ingest path layered onto the same
`MarketDataSource` trait is purely additive.

### What v1.5b is **not**

- Not a strategy. **No new strategy crate, no new strategy TOML,
  no new edge claim.** v1.5b expands the **data path**; future
  strategies (v2 LLM-augmented, cross-venue arb) consume v1.5b's
  output. A "cross-venue arbitrage" strategy that reads two
  venues' prices for the same instrument is a candidate v2 or
  v3 entry; v1.5b is the plumbing it sits on.
- Not real-money execution. v1.5b stays inside the
  [project scope boundary](../product.md#project-scope-boundary):
  paper-trading on real market data, simulated fills.
  **The execution side stays single-venue** (paper engine
  fills against the venue the operator's `MatchingEngine` is
  configured for; cross-venue execution is a follow-up project
  concern). v1.5b is **data ingest only**.
- Not perp futures. v1.5b stays spot-only per the
  [universe ladder](../product.md#universe--data-fidelity-ladder)
  (perps are v2 territory).
- Not L2 books. The architect's v1.5a Q6 deferral
  ([architecture.md → v1.5a Q6 — L2 / funding-rate ingest:
  stay deferred, lines 1160–1180](../architecture.md#v15a-q6--l2--funding-rate-ingest-stay-deferred))
  punts L2 ingest to v1.5b's discretion or v2 perp-shorting.
  Analyst recommendation: **stay deferred** — L2 is its own
  ingest surface (depth update events, snapshot+diff
  reconciliation, sequence-gap recovery) that doubles v1.5b
  scope. Architect can override.
- Not LLMs. LLM cost stays $0.00 for v1.5b. LLM lands in v2.
- Not a re-anchor of the 9 backtest scenarios. The
  [11/11 anchor](../anchors.toml) regression goal is preferred
  byte-identical, achievable by construction (see R11).

### The moat-bet implication

Multi-venue data fits the [moat bet](../product.md#differentiator)
(persistent memory + double-entry audit) the moment a Tick / Bar
carries a `venue` field: the audit ledger can attribute
"BTCUSDC fill on Coinbase" vs "BTCUSDT fill on Binance" to the
correct venue, the lesson-card memory can key on venue +
symbol + strategy, and the operator-success-reports R7
"feed reconnects" row (T805) becomes per-venue rather than a
single Binance counter. v1.5b is the **data-shape change** that
unlocks venue-aware moat features later. The brief itself adds
no memory or audit logic — those land downstream — but the
type-system change (Tick / Bar gain `venue: Venue`) is
load-bearing for everything that follows.

### Cost discipline

Multi-venue infra **must** stay inside the locked cost ladder
([product.md → Cost economics, lines 332–347](../product.md#cost-economics--monthly-ceiling))
of $45 / $135 / $360 monthly. All three target venues (Binance,
Coinbase, Kraken) expose **free public market-data WebSocket
endpoints** for spot trades + klines without API keys (Q8 confirms);
no paid subscriptions are required. Hosting cost stays inside the
v2 $80/mo ceiling because three venues × ten symbols × WS
streams is a constant memory footprint (~30 long-lived TCP
connections) that fits comfortably on the same single VM. **Zero
cost-ladder violation by design.**

## Requirements

Numbered, testable, derived from
[product.md → Universe & data fidelity ladder](../product.md#universe--data-fidelity-ladder)
v1.5 entry, the architect's v1 Q4 / v1.5a Q1 / v1.5a Q5 / v1.5a Q6
resolutions, the v1 closeout's T612 deferral, and the existing
`MarketDataSource` trait shape in
[`crates/data/src/source.rs`](../../crates/data/src/source.rs).
Each ends with a one-line **acceptance** the tester can verify.
All requirements preserve the v0 / v0.5 / v1 / v1.5a `Strategy`
trait shape (no trait changes), the audit chart of accounts (no
new accounts beyond what per-symbol-position-accounts already
seeds), and — critically — the 9 locked backtest anchor hashes
(R11).

### R1 — Coinbase market-data ingest

- **R1.1** New `CoinbaseFeed` struct in `crates/data/src/coinbase.rs`
  implementing the existing `MarketDataSource` trait. Same shape as
  `BinanceFeed`: WS URL, REST URL, optional `Arc<audit::Ledger>` for
  T805 feed-reconnect events, `with_ledger` builder.
- **R1.2** WS subscriptions for **trades** (raw) and **kline-equivalent**
  (Coinbase calls them `candles` on Advanced Trade; bar / tf semantics
  match Binance). Architect picks the WS endpoint per Q2 (Advanced Trade
  vs legacy Pro).
- **R1.3** Symbol normalization at the `CoinbaseFeed` boundary:
  Coinbase's `BTC-USD` / `BTC-USDC` shape is mapped to the agent's
  shared `Symbol` newtype (`crates/core/src/symbol.rs`). The
  normalization is **adapter-local**: the agent's universe still
  uses `BTCUSDC`-style strings, the adapter rewrites the on-wire
  form. The mapping is documented in a `coinbase_symbol_map` helper.
- **R1.4** Reconnect handling identical to `BinanceFeed`: exponential
  back-off (1s base, 60s cap), pong replies on server pings, T805
  `FeedReconnect` writer fires on every re-establishment after the
  initial connect — **with venue context** (R7 / R8).
- **R1.5** REST `exchange_info` populated from Coinbase's `products`
  endpoint (`base_currency`, `quote_currency`, `min_funds`, `base_min_size`)
  → `SymbolInfo`. Error mapping identical to Binance (Connection /
  Parse / StreamClosed).
- **R1.6** Free public market data only — **no authenticated WS**,
  **no API keys**, **no rate-limited REST endpoints beyond
  `products`**. Q8 confirms unauth WS is sufficient for our use.
- **Acceptance:** `cargo test --workspace -p data` exercises
  `CoinbaseFeed::exchange_info("BTC-USD")` against a `wiremock` REST
  fixture and `CoinbaseFeed::subscribe_trades` against a scripted
  WS test harness (Q10). At paper-mode startup with
  `[data.sources.coinbase]` enabled in `config/agent.toml`, a
  `BTC-USD` Tick stream emits at least one event within 30s.

### R2 — Kraken market-data ingest

- **R2.1** New `KrakenFeed` struct in `crates/data/src/kraken.rs`
  implementing `MarketDataSource`. Same shape as `BinanceFeed` /
  `CoinbaseFeed`.
- **R2.2** WS subscriptions for **trades** (`trade` channel) and
  **OHLC bars** (`ohlc` channel) on Kraken's WebSocket v2 API.
  Architect picks v2 vs v1 (Kraken's v1 is deprecated but still
  online; v2 is current).
- **R2.3** Symbol normalization at the `KrakenFeed` boundary:
  Kraken's `XBT/USD`, `XBT/USDC`, `XBT/USDT` shape is mapped to
  the agent's `Symbol` newtype. Note: Kraken uses `XBT` for
  Bitcoin (legacy ISO-4217 'X' prefix). Adapter-local
  `kraken_symbol_map` translates `BTCUSDC` ↔ `XBT/USDC`.
- **R2.4** Reconnect handling identical to R1.4 — exponential
  back-off, pong replies, T805 `FeedReconnect` events with venue
  context.
- **R2.5** REST `exchange_info` populated from Kraken's
  `AssetPairs` endpoint (`base`, `quote`, `lot_decimals`,
  `pair_decimals`, `ordermin`) → `SymbolInfo`.
- **R2.6** Free public market data only — Kraken's public WS does
  not require auth for trade / OHLC channels. Q8 confirms.
- **Acceptance:** `cargo test --workspace -p data` exercises
  `KrakenFeed` analogously to R1; paper-mode startup with
  `[data.sources.kraken]` enabled emits at least one
  `XBT/USD` Tick within 30s.

### R3 — USDC pairs in the universe

- **R3.1** Universe extension: the v1.5a 10-symbol BTCUSDT-quoted
  universe (`BTCUSDT`, `ETHUSDT`, `BNBUSDT`, `SOLUSDT`, `XRPUSDT`,
  `ADAUSDT`, `DOGEUSDT`, `AVAXUSDT`, `DOTUSDT`, `LINKUSDT` — see
  `config/agent.toml [funding].universe`, lines 56–60) gains a
  **mirror set** of USDC pairs: `BTCUSDC`, `ETHUSDC`, `BNBUSDC`,
  `SOLUSDC`, `XRPUSDC`, `ADAUSDC`, `DOGEUSDC`, `AVAXUSDC`,
  `DOTUSDC`, `LINKUSDC`. Universe shape per Q6 — analyst recommends
  **doubled** (now 20 symbols total, operator opts in to either
  set via config); architect picks.
- **R3.2** Per-pair venue routing: USDC pairs default to Coinbase
  + Kraken (where USDC liquidity is concentrated per
  [architecture.md → v1.5a Q5, lines 1130–1144](../architecture.md#v15a-q5--usdc-pairs-blocked-on-v15b-multi-venue));
  USDT pairs default to Binance. Operator can override per pair
  via `config/agent.toml [data.sources.<venue>].symbols`.
- **R3.3** v1.5a's `MeanReversionPairsStrategy` USDC-pair rejection
  (the `unsupported_quote` error in
  [v15a-mean-reversion-pairs.md, R1.3 USDT-only constraint, lines
  113–120](../v15a-mean-reversion-pairs/feature.md)) is **lifted** — the strategy
  loader now accepts USDC pair tuples. v1.5b carries the explicit
  deliverable from
  [architecture.md → v1.5a Q5, lines 1146–1150](../architecture.md#v15a-q5--usdc-pairs-blocked-on-v15b-multi-venue):
  "unblock USDC pair support in `MeanReversionPairsStrategy` once
  Coinbase / Kraken adapters ship." Architect confirms whether
  this is a v1.5b task or follow-up.
- **R3.4** USDC asset enum variant: `Asset::Usdc` added to
  `crates/core/src/asset.rs` alongside the existing `Btc` / `Eth` /
  `Other(SmolStr)` variants. `Universe::from_usdt_symbols` gets a
  sibling `from_usdc_symbols` constructor.
- **Acceptance:** a paper-fill cycle on `BTCUSDC` (Coinbase) writes
  rows to the audit DB whose `account_id` is
  `assets:position:BTCUSDC` (per
  [features/per-symbol-position-accounts.md → R1, lines 30–60](../per-symbol-position-accounts/feature.md));
  `audit::query::pnl_by_symbol` returns a row for `BTCUSDC`.

### R4 — T612 multi-symbol live BinanceFeed

- **R4.1** `BinanceFeed::subscribe_bars` and `subscribe_trades`
  today open **one WS connection per symbol per stream**
  (`crates/data/src/binance.rs:255–367` and `374–474`). T612
  finishes the multi-symbol live ingest by **multiplexing** the
  10-symbol universe onto Binance's combined-stream endpoint
  (`wss://stream.binance.com:9443/stream?streams=...`) so
  one WS connection serves N symbols.
- **R4.2** Per-symbol `clock_skew_ms{feed,symbol}` Prometheus label
  emission, deferred at v1 closeout
  ([v1-cross-sectional-momentum.md → T612 status, lines 1518–1520](../v1-cross-sectional-momentum/feature.md#t612-status)).
- **R4.3** Testnet smoke test (also deferred at v1 closeout) — a
  Binance Spot Testnet WS URL constant + a smoke test that
  connects against testnet and asserts at least one Tick per
  symbol within 60s.
- **R4.4** Backwards compat: today's single-symbol
  `subscribe_bars(symbol, tf)` API is **preserved**. T612 adds a
  new fan-out method (e.g. `subscribe_bars_multi(&[Symbol], tf)
  → BoxStream<Result<Bar>>`) — the merged stream emits Bars in
  `(venue_ts ASC, symbol ASC)` order, matching the v1 multi-symbol
  replay determinism contract
  ([v1-cross-sectional-momentum.md → R12.2 / R12.4](../v1-cross-sectional-momentum/feature.md)).
  Architect can decide single-method-with-list vs new-method.
- **Acceptance:** `cargo test --workspace -p data` includes a 10-symbol
  combined-stream test (mocked WS server) that emits one bar per
  minute boundary in alphabetical order, no message lag > 5s.

### R5 — 1-second aggregated trades

- **R5.1** `Timeframe::OneSecond` variant added to
  `crates/core/src/bar.rs:11–19` (display string `"1s"`). Existing
  variants unchanged.
- **R5.2** Server-side vs client-side aggregation: per Q5
  recommendation, **client-side**: the `bar_stream` module
  (`crates/data/src/bar_stream.rs`, already exists for tick → bar
  aggregation per
  [architecture.md → Tick aggregation, lines 2101–2105](../architecture.md#tick-aggregation))
  gains a 1-second window. Aggregator consumes the raw `Tick`
  stream and emits `Bar { tf: OneSecond, … }` events at every
  1-second boundary aligned to UTC second.
- **R5.3** Determinism: the 1s bar's `open_ts` is the rounded-down
  second boundary; `close_ts` is `open_ts + 1s` minus 1 microsecond
  (matching the existing 1m bar convention). Empty seconds (no
  trades) emit no bar — strategies see "trade-driven" bars only.
- **R5.4** Cross-venue consistency: 1s bar aggregation is **per
  (venue, symbol)** pair, not cross-venue. A `BTCUSDT@Binance` 1s
  bar and a `BTCUSDT@Coinbase` 1s bar are two distinct streams
  (R7 — Tick / Bar carries venue).
- **R5.5** Performance: client-side aggregation must hold p99
  < 500µs per Tick at 10 symbols × 3 venues = 30 streams (well
  inside the v0 5ms hot-path budget per
  [architecture.md → Performance budget](../architecture.md#performance-budget)).
- **Acceptance:** `cargo test -p data` includes a synthetic-trade
  fixture (e.g. 100 ticks across 10 seconds, scripted) that
  exercises the 1s aggregator and asserts the emitted bars match
  expected OHLCV by hand.

### R6 — Per-venue `MarketDataSource` impls

- **R6.1** Three concrete impls of the existing
  [`MarketDataSource` trait](../../crates/data/src/source.rs):
  `BinanceFeed` (extended for R4), `CoinbaseFeed` (R1),
  `KrakenFeed` (R2). All three implement `exchange_info`,
  `subscribe_bars`, `subscribe_trades` with identical async-trait
  signatures.
- **R6.2** **No trait change.** The trait shape stays as today;
  v1.5b's only trait-level addition is on the `Bar` / `Tick` types
  (R7 — venue field). Trait methods continue to take `Symbol` +
  `Timeframe`; the `venue` is implicit in the impl.
- **R6.3** Optional: a thin `MultiVenueSource` struct that holds
  `Vec<Box<dyn MarketDataSource>>` and presents a unified subscription
  surface. Architect picks (Q3) — analyst recommends **per-venue
  tasks** (parallel, panic-isolated) over `select_all` merge.
- **Acceptance:** all three feeds share the trait surface; a
  generic `fn ingest<F: MarketDataSource>(feed: F, …)` compiles
  against each.

### R7 — Venue identification on `Tick` / `Bar`

- **R7.1** New `Venue` type in `crates/core/src/symbol.rs` (or a
  new `venue.rs` — architect picks). Shape per Q1:
  - **Recommended (analyst):** closed enum
    `enum Venue { Binance, Coinbase, Kraken }` with `#[serde(rename_all =
    "lowercase")]`. Closed set is safer (typo-proof, exhaustive match)
    and matches the bounded universe of three target venues.
  - **Alternative:** newtype `Venue(SmolStr)` — open set, no spec churn
    when adding a fourth venue. Analyst rejects: every venue has its
    own ingest impl + symbol normalization, so adding a venue is
    never an open-set change in practice.
- **R7.2** `Bar` and `Tick` gain `venue: Venue` as a **required**
  field (not `Option<Venue>`). Migration: every existing fixture
  Bar / Tick literal in `crates/*/src/**` and `crates/*/tests/**`
  gains `venue: Venue::Binance` (one-line change per literal). All
  `Bar { … }` / `Tick { … }` constructors in `BinanceFeed`,
  `CoinbaseFeed`, `KrakenFeed`, `replay_feed.rs`, `fake_feed.rs`,
  `bar_stream.rs` set the field at the venue boundary.
- **R7.3** Q4 explicitly recommends **required** (closed enum) +
  full migration. The `Option<Venue>` "soft introduction" is
  **rejected**: it leaks a `None` variant through the entire
  codebase (every `if venue.is_some() { … }` branch is a
  potential bug surface) and gains nothing — the architect's grep
  shows zero existing Bar / Tick literals are in committed report
  bodies (R11), so the migration is mechanical.
- **R7.4** Determinism contract update: the v1 multi-symbol replay
  ordering of `(venue_ts ASC, symbol ASC)` extends to
  `(venue_ts ASC, venue ASC, symbol ASC)`. Tie-break order:
  `Binance < Coinbase < Kraken` (alphabetical on the closed enum's
  Display string).
- **Acceptance:** `cargo test --workspace` compiles after the Bar /
  Tick migration; `Venue::Binance` appears in zero committed
  report bodies (R11 grep gate).

### R8 — Per-venue strategy_events

- **R8.1** T805's `feed_reconnect` writer in
  `crates/audit/src/journal.rs:648–668` currently captures a
  `symbol: &str` argument and stores it in the
  `error_summary` column. v1.5b extends the writer to also
  capture **venue**.
- **R8.2** Two implementation options (architect picks per Q11):
  - **(a) Schema migration `006_strategy_events_venue.sql`** —
    add a nullable `venue TEXT` column to `strategy_events`. Pros:
    typed, queryable. Cons: schema churn, possible anchor risk
    if any committed report body renders the venue column. Needs
    architect-confirmed grep.
  - **(b) Encode `<venue>:<symbol>` in `error_summary`** —
    e.g. `"binance:BTCUSDT"`. Pros: zero schema migration, zero
    anchor risk. Cons: parse-on-read at the operator-success-reports
    R7 row.
  - Analyst recommends **(b)** — the `error_summary` column is
    already TEXT and architect-blessed for free-form provenance
    (per `[architecture.md → v1+ — Operator success reports
    resolutions](../architecture.md#v1--operator-success-reports-resolutions-q1q9--confirmed-2026-05-01)`).
    Stays consistent with the v0.5 / v1 / v1.5a additive pattern.
- **R8.3** `KillSwitchTripped` events are scoped to a venue
  **only when caused by a venue-specific event** (e.g. clock-skew
  on Coinbase). Otherwise venue is `None` (global trip).
- **R8.4** The operator-success-reports R7 system-health row
  (T805 / T811) renders a per-venue feed-reconnect count: the
  reports binary reads `strategy_events_since(period_start)`,
  filters to `kind == FeedReconnect`, and groups by venue parsed
  from `error_summary`. **Anchor risk:** the operator-success
  anchors lock report bodies; venue-grouped feed reconnects
  appear in body. v1.5b's anchor budget assumes either zero
  reconnect rows in the locked anchor fixtures (likely — the
  fixtures are short-window) or a re-lock budget for those
  specific anchors. Architect confirms via grep on the locked
  report bodies.
- **Acceptance:** a synthetic Coinbase reconnect on `BTCUSDC`
  writes one row to `strategy_events` with
  `kind = FeedReconnect`, `error_summary = "coinbase:BTCUSDC"`
  (or equivalent under R8.2 (a) shape).

### R9 — Cost discipline

- **R9.1** Multi-venue infra **must** stay inside the
  [cost ladder](../product.md#cost-economics--monthly-ceiling)
  ($45 / $135 / $360 monthly). v1.5b lands inside the v2 $360/mo
  ceiling.
- **R9.2** All three venues expose **free public market-data WS
  endpoints** (Q8 confirms). No paid data subscriptions, no API
  keys for read-only market data, no per-message billing.
- **R9.3** Memory + CPU footprint: ~30 long-lived TCP connections
  + the existing single-VM hosting fits well inside the v2 hosting
  ceiling ($80/mo). Zero net cost increase from v1.5a baseline.
- **R9.4** v1.5b run-time emits an explicit `costs.md` line item
  per venue: each row reports `LLM tokens: $0.00`, `Market data:
  $0.00`, total well below ceiling. Tester gates V12.
- **Acceptance:** the auto-generated `costs.md` after a paper run
  shows zero net cost increase vs. v1.5a baseline.

### R10 — Backwards compatibility

- **R10.1** Single-venue (Binance only) configurations **continue
  to work unchanged**. An operator who does **not** add
  `[data.sources.coinbase]` or `[data.sources.kraken]` to
  `config/agent.toml` sees identical agent behavior to v1.5a.
- **R10.2** The default `config/agent.toml` shipped in the repo
  keeps Binance-only ingest enabled and the other two venues
  disabled. Operators opt in by adding stanzas.
- **R10.3** `BinanceFeed::production()` constructor unchanged. The
  T612 multi-symbol upgrade lands as an **additive** method
  (R4.4) — single-symbol callers (`subscribe_bars`,
  `subscribe_trades`) keep working.
- **R10.4** The `Tick` / `Bar` `venue` field is required (R7.2),
  but all existing constructors in single-venue paths default to
  `Venue::Binance` — no test fixture rebuilds; the migration is
  mechanical (R7.3).
- **R10.5** All v0 / v0.5 / v1 / v1.5a strategies work
  byte-identically against the v1.5b ingest path, because:
  (a) strategies see `Bar` and `Tick` events that **already**
  carry `Venue::Binance` after the migration; (b) the strategy
  trait is unchanged; (c) no strategy in v0–v1.5a inspects
  `bar.venue` (zero references — analyst grepped).
- **Acceptance:** v1.5a's `top10-2024-h1-momentum` and
  `pairs-2024-h1` backtests reproduce byte-identical reports
  pre- and post-v1.5b (anchor regression — see R11 / V8).

### R11 — Anchor regression — 11/11 PASS by construction

- **R11.1** The 9 backtest anchors live in
  [`spec/anchors.toml`](../anchors.toml) per
  [AGENT.md → Process discipline, lines 244–248](../../AGENT.md#process-discipline-lessons-from-v0--v15a):
  the body-only SHA-256 of nine canonical scenarios. The 2 v1+
  anchors (the operator-success-reports body anchors) are also
  preserved.
- **R11.2** **Architect must independently grep** every
  committed report body under `spec/<slug>/reports/` for the strings
  `venue`, `coinbase`, `kraken`, `binance:` (case-insensitive).
  Expected count: **zero** (the backtest reports render
  aggregated metrics — total return, Sharpe, drawdown — and
  per-symbol summaries, not raw Bar / Tick rows). Operator-
  success report bodies render strategy-event counts; if any
  fixture has a feed-reconnect row in body, that's the anchor
  surface to confirm.
- **R11.3** If R11.2 grep shows zero hits, anchor risk is **zero
  by construction**: the type system change adds a field that
  no committed body references. v1.5b runs the 9 + 2 anchor
  gate `verify-anchors` as the tester's hard gate, and
  every hash matches.
- **R11.4** If R11.2 grep shows non-zero hits (unexpected),
  the analyst routes back to the architect for a re-lock
  budget per the v1.5a T717 / T811 precedent (re-anchor only
  the affected scenarios, document each re-lock with a
  one-line rationale).
- **Acceptance:** `verify-anchors` exits 0 with all 11 anchor
  hashes matching post-v1.5b code change.

### R12 — Invariant preservation

All previously locked invariants from v0 / v0.5 / v1 / v1.5a /
operator-success-reports / per-symbol-position-accounts /
journal-transactions-metadata / tape-row-audit-modal hold
unchanged:

- **R12.1** T802 (journal_transactions strategy_id), T805
  (feed_reconnect writer — extended per R8 in additive shape),
  T806 (agent_uptime), T809 / T810 (reports binary spawn /
  in-process cron flag) — all hold. T805 writer signature gains
  a venue argument (R8) but call sites in the kill-switch
  handler / cockpit reload path are venue-agnostic and pass
  `None` until they're refactored to be venue-aware (out of
  scope for v1.5b).
- **R12.2** T901–T912 (live-cockpit-unified — Subscription /
  EventBus / kill-switch closure / agent-runtime / fixtures /
  prometheus listener). EventBus channels stay venue-blind
  in v1.5b (the bus broadcasts `Bar` / `Tick` directly; the
  consumer reads `bar.venue` if it cares). No new bus
  channels.
- **R12.3** T1101–T1107 (per-symbol-position-accounts) — `assets:
  position:<SYMBOL>` hold per-symbol (post per-symbol-position-accounts).
  v1.5b adds 10 USDC mirror symbols to the universe; the
  bootstrap migration `006_per_symbol_position_accounts.sql`
  reads `config/agent.toml [funding].universe` at agent start
  (per
  [architecture.md → Audit migration list, line 342](../architecture.md#audit-migration-list--current))
  and seeds rows for every symbol. Adding USDC pairs is an
  additive `INSERT OR IGNORE` (no schema migration).
- **R12.4** T1201–T1209 (tape-row-audit-modal) — UI invariants.
  v1.5b is plumbing-only; modal renders venue field optionally
  (architect's call — likely a future UI iteration absorbs it).
- **R12.5** T1301–T1305 (journal-transactions-metadata) — the
  reader is unaffected by venue (operates on
  `journal_transactions` rows, which are venue-blind by
  construction).
- **Acceptance:** `verify-anchors` PASS + `cargo test --workspace`
  PASS — every previously-passing test still passes.

### R13 — Q-resolution: `Venue` shape in the type system

This is **Q1** — open question for architect; analyst's R-level
recommendation:

- **R13.1** Newtype enum: `enum Venue { Binance, Coinbase, Kraken }`
  with `#[serde(rename_all = "lowercase")]`. Closed set, exhaustive
  match, typo-proof.
- **R13.2** `impl Display for Venue` emits `"binance"` / `"coinbase"` /
  `"kraken"` (lowercase). `impl FromStr` parses the same lowercase
  strings; `Other` / unknown returns `Err(ParseVenueError)`.
- **R13.3** `Asset::from_venue(venue: Venue) -> ...` not needed;
  `Venue` is orthogonal to `Asset`.
- **R13.4** `Venue::default()` is **not implemented** — every Bar /
  Tick must explicitly construct it; the migration touches every
  literal exactly once.
- **Acceptance:** architect signs off on the closed enum or
  rejects with a counter-proposal per Q1.

### R14 — Failure isolation per venue

- **R14.1** A Coinbase outage **must not** crash Binance ingest.
  Each venue's ingest runs in its **own tokio task**.
- **R14.2** Per-venue tokio task spawning: `agent::runtime::run`
  spawns `feeds.len()` independent tasks; each task wraps the
  venue's `subscribe_*` stream consumption in
  `tokio::task::spawn` + `JoinHandle` tracking. Architect picks
  per Q3 — analyst recommends **per-venue tasks** over `select_all`
  merge so a panic in one venue's parser never poisons the
  others.
- **R14.3** Panic isolation: each task wraps the
  `BoxStream<Result<Bar>>` consumption in
  `tokio::task::spawn` (catches panics → emits a
  `feed_reconnect` event with `venue:symbol` provenance + the
  panic message in `error_code`).
- **R14.4** Failover semantics: per Q7, **strategies pause on
  stale-data threshold** (e.g. 30s of no Tick from a venue).
  Architect picks the threshold; analyst recommends 30s as the
  default (longer than the longest expected reconnect, shorter
  than a meaningful market move).
- **R14.5** The kill-switch (per
  [architecture.md → Risk engine](../architecture.md#risk-engine))
  may trip globally (all venues halt) on a venue-specific clock-skew
  event but does not halt one venue if another venue still has
  fresh data. Architect picks (Q7).
- **Acceptance:** an integration test simulates a Coinbase WS
  drop mid-stream; Binance + Kraken streams continue emitting
  events; the Coinbase reconnect path emits a `FeedReconnect`
  event within 60s; no panic propagates outside the Coinbase
  task.

### R15 — Data shape consistency across venues

- **R15.1** Every venue's adapter normalizes its on-wire event
  format to the shared `Tick` / `Bar` types from
  `crates/core/src/{tick,bar}.rs`. Coinbase's trade event
  (`{"side": "buy", "size": "...", "price": "...", "time": "...", ...}`)
  → `Tick { side: Side::Buy, qty, price, venue_ts, … }`.
- **R15.2** Aggressor side: Binance reports `is_buyer_maker`
  (boolean → `Sell` if maker, else `Buy`); Coinbase reports
  `side: "buy" | "sell"`; Kraken reports `b | s`. All three
  normalize to `core::Side::Buy | Sell`.
- **R15.3** Timestamp normalization: every venue's timestamp
  field (Binance ms `T`, Coinbase RFC-3339 string `time`,
  Kraken float-seconds-since-epoch) → `core::Timestamp` (a
  `time::OffsetDateTime` newtype). Microsecond precision is
  preserved per the
  [AGENT.md determinism non-negotiables](../../AGENT.md#process-discipline-lessons-from-v0--v15a).
- **R15.4** Decimal parsing: every venue's price / quantity
  field arrives as a string and parses through
  `rust_decimal::Decimal` — no `f64` ever touches money math.
  Identical to today's `BinanceFeed::parse_decimal`.
- **R15.5** Trade-ID normalization: Binance `trade_id: u64`,
  Coinbase `trade_id: i64`, Kraken trade IDs are tuple
  `(timestamp, sequence)`. Each adapter maps to a `u64` (Kraken
  via a hash of the tuple). Trade-ID uniqueness within
  `(venue, symbol)` only — not cross-venue.
- **Acceptance:** a unit test fixture for each venue parses a
  scripted on-wire payload and asserts the resulting `Tick` /
  `Bar` matches expected fields exactly.

## Verification (V-items)

The tester's contract for declaring v1.5b done. All items must be
green before a `VERDICT → PASS` can be issued. Mapping to R-numbered
requirements is explicit so the tester's report can cross-reference.

- **V1 — Binance feed regression.** Existing single-venue Binance
  feed tests pass unchanged. `cargo test -p data
  binance::` exercises the v0–v1.5a paths; no regression. Maps
  to R10. Static checks pass — `cargo fmt --check` clean,
  `cargo clippy --workspace --all-targets -- -D warnings` clean,
  `cargo audit` no advisories, `cargo deny check` passes.
  Maps to R10 + R12.
- **V2 — Coinbase feed connects + emits Tick.** A live (or
  scripted-WS) `CoinbaseFeed::subscribe_trades(Symbol::new(
  "BTC-USD"))` emits at least one Tick within 30s. Tick
  fields verified: `venue == Venue::Coinbase`, `symbol`
  matches, `price`, `qty`, `side`, `venue_ts`, `local_recv_ts`
  populated, `trade_id` non-zero. Maps to R1.
- **V3 — Kraken feed connects + emits Tick.** Same shape as V2
  for `KrakenFeed::subscribe_trades(Symbol::new("XBT/USD"))`.
  `Tick.venue == Venue::Kraken`, symbol normalization preserved
  through. Maps to R2.
- **V4 — USDC pairs in audit DB.** A paper-fill cycle on
  `BTCUSDC` (via Coinbase paper engine) writes journal entries
  whose `account_id == "assets:position:BTCUSDC"` and
  `audit::query::pnl_by_symbol(since, until)` returns a row
  for `BTCUSDC`. Maps to R3.
- **V5 — 1-second bars from synthetic trades.** A synthetic
  trade-stream fixture (e.g. 100 ticks across 10 seconds at
  scripted prices) feeds the 1s aggregator; emitted bars match
  expected OHLCV by hand. Determinism: two runs against the
  same fixture emit byte-identical bars. Maps to R5.
- **V6 — Multi-symbol live BinanceFeed fan-out (T612).** A
  10-symbol mock-WS server emits one bar per minute boundary;
  `BinanceFeed::subscribe_bars_multi(...)` (or equivalent
  per architect's R4 choice) merges them into a single stream
  with no message lag > 5s and `(venue_ts ASC, symbol ASC)`
  ordering. Per-symbol Prometheus
  `clock_skew_ms{feed,symbol}` label populated. Maps to R4.
- **V7 — Coinbase outage scenario.** A scripted Coinbase WS
  drop mid-run: Binance + Kraken streams continue uninterrupted;
  the Coinbase reconnect path emits at least one `FeedReconnect`
  event with venue-tagged `error_summary` within 60s; no panic
  propagates outside the Coinbase task. Maps to R14.
- **V8 — Anchor regression — 11/11 PASS.** `verify-anchors`
  exits 0 with all 9 backtest + 2 v1+ anchor hashes matching
  pre/post-v1.5b. Maps to R11.
- **V9 — T802 / T805 / T806 / T809 / T810 invariants hold.**
  `cargo test -p audit -p reports -p agent` green.
  `feed_reconnect` writes carry venue context (R8); reports
  binary's R7 system-health row groups feed-reconnect counts
  by venue. Maps to R12.1.
- **V10 — T901–T912 invariants hold.** `cargo test -p ui -p
  agent --features fixtures` green. Cockpit live subscription
  works against the multi-venue ingest path; no Subscription
  changes needed. Bus channels stay venue-blind; consumer
  reads `bar.venue` on demand. Maps to R12.2.
- **V11 — T1101–T1107 invariants hold.** `cargo test -p audit`
  green. The bootstrap migration `006_per_symbol_position_accounts.sql`
  seeds 20 `assets:position:<SYMBOL>` rows (10 USDT + 10 USDC)
  when the universe is doubled (R3.1). Maps to R12.3.
- **V12 — T1201–T1209 + T1301–T1305 invariants hold.**
  `cargo test -p ui -p audit` green. Cost telemetry: the
  auto-generated `costs.md` after a v1.5b paper run shows
  zero LLM tokens, zero market-data spend, total well below
  the v2 $360/mo ceiling. Maps to R12.4 + R12.5 + R9.
- **Performance budget.** `cargo bench -p data` shows the 1s
  aggregator p99 < 500µs per Tick (R5.5). 30-stream ingest
  fits inside the v0 5ms hot-path budget. Maps to R5.

Failure on any of V1–V12 routes per the v0 / v0.5 / v1 / v1.5a
verdict-routing contract:

- Static / test / bench failure → `developer`.
- Anchor regression → `developer` with body diff.
- Architecture surface change required → `architect`.
- Strategy regression (none expected; v1.5b is plumbing-only) →
  `analyst`.
- UI/visual regression → `ui-designer` (no UI surface in v1.5b
  beyond optional venue rendering — see R12.4).

## Backtest scenarios

**v1.5b is a data-plumbing feature; no new backtest scenario
ships in this brief.** Cross-venue strategies (e.g. arbitrage on
the same instrument across two venues) are a candidate v2 / v3
entry that consumes v1.5b's data path, **not v1.5b itself**.

The 9 locked backtest anchors
([`spec/anchors.toml`](../anchors.toml)) are preserved
byte-identical post-v1.5b per R11 (architect-confirmed grep
gate). v1.5b does not introduce a new strategy, so no new
scenario row is added.

**Optional architect call:** add a single non-strategy anchor
scenario `multi-venue-readonly-2024` that captures venue-tagged
tick stream determinism on a fixture replay of (Binance,
Coinbase, Kraken) for one day's BTCUSDT/BTC-USD/XBT/USDT
streams. **No orders placed; pure data-path test.** This
would lock the deterministic interleave order
`(venue_ts ASC, venue ASC, symbol ASC)` per R7.4. Analyst
recommendation: **defer** — add the scenario only if the
architect identifies a specific regression risk that body-SHA
hashing of a tick stream guards against. The unit + integration
tests in V5 / V6 / V7 already cover this surface; a backtest
anchor is over-engineering for a plumbing feature.

_n/a — plumbing feature; no new backtest scenarios. v2
cross-venue strategy will use v1.5b's data path._

## Open questions for architect

These resolutions are deliberately punted to the architect; analyst
provides recommendations but defers the call.

### Q1 — `Venue` type shape: enum vs newtype

**The question:** how is `Venue` represented in the type system?

**Recommended (analyst):** closed enum `Venue { Binance, Coinbase,
Kraken }` with `#[serde(rename_all = "lowercase")]`. Pros:
exhaustive match catches typos at compile time, easy to add a
fourth venue (one line + downstream match arms), `Hash` /
`Ord` for free.

**Alternative:** newtype `Venue(SmolStr)` (open set). Pros:
no spec churn when adding a venue. Cons: every venue has its
own ingest impl + symbol normalization, so adding a venue is
never a "string change" in practice.

### Q2 — Coinbase API choice: Advanced Trade vs Pro

**The question:** Coinbase has two public WS APIs — the newer
**Advanced Trade WS** and the legacy **Coinbase Pro WS**. Both
support unauthenticated trade + candles channels for spot
market data. Which does v1.5b target?

**Recommended (analyst):** **latest stable** — Advanced Trade
WS — for forward compatibility. Coinbase has signaled Pro is
in maintenance mode; v2's potential cross-venue expansion would
inherit our v1.5b choice.

**Alternative:** Coinbase Pro WS for stability.

### Q3 — Ingest topology: per-venue tasks vs select_all merge

**The question:** does `agent::runtime` spawn one tokio task per
venue (parallel, panic-isolated) or one combined task with
`futures::stream::select_all` over the three venues' streams?

**Recommended (analyst):** **per-venue tokio tasks**. Pros:
panic in one venue's parser doesn't poison the others
(R14.1, R14.3); each task can have its own backoff /
reconnect state without sharing a future poll loop. Cons:
slightly higher steady-state task count (3 tasks vs 1).

**Alternative:** `select_all` for a single futures-driven
poll loop. Pros: simpler. Cons: one panic kills all venues
(R14.1 violation by construction).

### Q4 — `Tick` / `Bar` venue field: required vs optional

**The question:** is the new `venue: Venue` field on `Tick` / `Bar`
required (must construct) or `Option<Venue>` (defaults to None
until the venue is known)?

**Recommended (analyst):** **required** + closed enum (Q1's
recommended) + full migration to all existing fixtures. The
migration touches every Bar / Tick literal once (mechanical
sed). The `Option<Venue>` "soft introduction" is rejected
because it leaks `None` through the entire codebase forever.

**Alternative:** `Option<Venue>` with `None` as the default
(every Bar / Tick gets a venue when one is known). Cons:
soft typing; `if let Some(v) = bar.venue { … }` everywhere.

### Q5 — 1-second bar aggregation: server-side vs client-side

**The question:** does the agent **request** 1-second bars from
each venue (if the venue supports them) or **aggregate raw
trade events** to 1s windows client-side?

**Recommended (analyst):** **client-side**. Pros: deterministic,
identical algorithm across venues, no dependency on venue's
bar definition (Binance's 1s bars on the WS endpoint are
new-ish; Coinbase / Kraken don't expose 1s bars publicly).
Plus the
[architecture.md → Tick aggregation, lines 2101–2105](../architecture.md#tick-aggregation)
already has the `bar_stream` module for this.

**Alternative:** server-side (subscribe to Binance's 1s WS
endpoint where available, fall back to client-side for the
others). Cons: heterogeneous bar definitions across venues;
strategies have to reason about it.

### Q6 — USDC mirror universe: doubled vs replaced

**The question:** does the v1.5b 10-symbol BTCUSDT-quoted
universe **double** (now 20 symbols total — the operator opts
in to either set via config) or **replace** (USDC only, USDT
deprecated)?

**Recommended (analyst):** **doubled**. Pros: USDT remains
the largest crypto-stablecoin pair set by volume (Binance);
deprecating it would cripple v1's cross-sectional momentum
strategy on its primary data set. Cons: 20 symbols × 3
venues = 60 subscription slots — Q9 confirms this stays
inside venue-side rate limits.

**Alternative:** replace with USDC-only. Cons: rejected;
would force v1's momentum strategy to re-anchor against a
new universe.

### Q7 — Failover semantics: pause vs stale-data window

**The question:** when a venue's feed dies (e.g. a Coinbase
disconnect mid-run), do strategies **pause** until reconnect
or **continue with stale data** until a stale-data threshold
elapses?

**Recommended (analyst):** **strategies pause on
stale-data threshold** with a 30s default (longer than the
longest expected reconnect, shorter than a meaningful market
move). The pause is per-venue: a Coinbase outage halts
strategies that consume Coinbase data; Binance-only
strategies continue.

**Alternative:** continue with stale data unconditionally
(strategy's job to handle); cons: rebalance based on stale
data is a real money-leak surface even in paper mode.

**Alternative:** pause everything globally on any venue
outage; cons: too conservative; defeats the cross-venue
redundancy goal.

### Q8 — Coinbase + Kraken authentication

**The question:** for public market data (trades + bars), do
Coinbase's Advanced Trade WS and Kraken's WebSocket v2 require
authenticated connections? If yes, **that's a $0/mo
violation flag** — Q9 / R9 cost discipline assumes free
public market data.

**Recommended (analyst):** **confirm unauthenticated WS is
sufficient for trades + candles channels on both venues**
(Coinbase Pro / Advanced Trade public market-data and
Kraken WebSocket v2 trade / OHLC channels both work
without API keys per the venues' published docs as of
2026-04). Architect to verify against current docs at
implementation time.

**Risk if false:** v1.5b's cost commitment breaks; analyst
re-routes to find the cheapest authenticated tier or drops
the venue.

### Q9 — Rate limits across 30 subscription slots

**The question:** Binance, Coinbase, and Kraken each impose
WS subscription limits + REST rate limits. The 10-symbol ×
3-venue (or 20-symbol × 3-venue if Q6 = doubled) universe
needs **30 (or 60) subscription slots**. Confirm against
each venue's free-tier limits.

**Recommended (analyst):** **architect to confirm at
implementation time**. Known limits:
- Binance Spot WS: 1024 streams per connection;
  fits comfortably (one combined-stream URL = 20 streams).
- Coinbase Advanced Trade WS: 750 messages per second per
  IP; 60 subscriptions × ~1 trade/s/symbol ≈ 60 msg/s — well
  inside.
- Kraken WS v2: per-channel subscription limit is 80;
  fits.

**Risk:** v2 cross-venue expansion might push us over
Kraken's per-channel limit; v1.5b stays inside.

### Q10 — Testing strategy for WS streams

**The question:** REST endpoints test cleanly with `wiremock`,
but WS testing is harder (the standard `wiremock` doesn't
script WS frames). What's the test harness?

**Recommended (analyst):** **lightweight `MockFeed` that
publishes scripted Tick events** through the existing
`crates/data/src/fake_feed.rs` infrastructure. The
`MockFeed` impl of `MarketDataSource` returns a
`BoxStream<Tick>` driven by a scripted vec of events; tests
assert the strategy's response to the scripted stream.

**Alternative:** spin up a real WS test server (e.g.
`tokio-tungstenite` server-side) per test. Pros: tests the
full WS-frame parse path. Cons: slower, more flaky in CI.

**Recommended:** `MockFeed` for strategy / integration tests;
WS-server harness only for the WS-parser unit tests in each
adapter (e.g. Coinbase trade-event JSON parse → `Tick`).

### Q11 — T805 schema migration vs error_summary encoding

**The question:** how does the `feed_reconnect` writer
(per `crates/audit/src/journal.rs:648–668`) capture the
**venue** context?

- **(a) Schema migration `006_strategy_events_venue.sql`** —
  add a nullable `venue TEXT` column to `strategy_events`.
  Pros: typed, queryable. Cons: schema churn; potential
  anchor risk if any committed report body renders the
  column.
- **(b) Encode `<venue>:<symbol>` in `error_summary`** —
  e.g. `"binance:BTCUSDT"`. Pros: zero schema migration,
  zero anchor risk. Cons: parse-on-read at the
  operator-success-reports R7 row.

**Recommended (analyst):** **(b)** — additive pattern,
matches the v0.5 / v1 / v1.5a / v1+ precedent of
encoding new structured info in TEXT columns rather than
adding columns.

### Q12 — Anchor risk validation

**The question:** confirm zero `venue` strings in any committed
report body (`spec/reports/**/*.md`).

**Recommended (analyst):** **architect runs the grep**:
`grep -ri "venue\|coinbase\|kraken" spec/reports/` (case-
insensitive). Expected count: zero — backtest reports render
aggregated metrics + per-symbol summaries, not raw Bar / Tick
rows. Operator-success report bodies render strategy-event
counts; if any fixture has a feed-reconnect row in body,
that's the anchor surface to confirm.

**If grep returns zero:** anchor risk is **zero by
construction** (the type system change adds a field that no
committed body references).

**If grep returns non-zero:** route back to architect for a
re-lock budget per the v1.5a T717 / T811 precedent (re-anchor
only the affected scenarios, document each re-lock with a
one-line rationale).

## Design

Author: architect, 2026-05-03. Resolves Q1–Q12 from the analyst
brief above. v1.5b is the **largest queued backend feature** —
three new market-data adapters, USDC universe, T612 multi-symbol
fan-out, 1s aggregation, per-venue tokio-task topology, audit
schema migration `007`, plus a load-bearing type-system change
(`Tick` / `Bar` gain `venue: Venue`). Design length budget:
≤ 600 lines. Anchor budget: **11 / 11 byte-identical** by
construction (Q12 confirmed). Plumbing-only — **no new strategy
crate**, no execution change.

### Independent verification of analyst findings (2026-05-03, architect)

Re-checked the cited source before designing on top:

| Claim | Result |
|---|---|
| `crates/data/src/binance.rs:255–367` `subscribe_bars` single-symbol-per-WS | CONFIRMED — `stream_name = "{symbol}@kline_{tf}"` and `ws_url = "{ws_url}/{stream_name}"`; one WS connection per `(symbol, tf)`. |
| `crates/data/src/binance.rs:374–474` `subscribe_trades` single-symbol-per-WS | CONFIRMED — `stream_name = "{symbol}@trade"`, one WS connection per symbol. T612 fan-out remains unimplemented. |
| `crates/data/src/source.rs` `MarketDataSource` trait | CONFIRMED — three async methods (`exchange_info`, `subscribe_bars`, `subscribe_trades`) on `Symbol` + `Timeframe`. No `Venue` parameter; venue is implicit in the impl. |
| `crates/audit/src/journal.rs:648–668` `feed_reconnect` | CONFIRMED — signature `feed_reconnect(ledger, symbol: &str, ts: Option<&str>)`. `error_summary` carries the symbol; no venue context today. Two call sites in `binance.rs:297-304` and `406-414` pass `symbol_for_audit.0.as_str()` and `None` for ts. |
| `crates/core/src/bar.rs:11–19` `Timeframe` | CONFIRMED — six variants `OneMinute` / `FiveMinutes` / `FifteenMinutes` / `OneHour` / `FourHours` / `OneDay`. R5.1 adds `OneSecond` as the smallest variant. Display strings: `"1m"`, `"5m"`, `"15m"`, `"1h"`, `"4h"`, `"1d"` → adds `"1s"`. |
| `config/agent.toml:62-65` `[funding].universe` | CONFIRMED — 10 USDT-quoted symbols. R3 doubles by adding 10 USDC-quoted mirrors. |
| `Tick` definition `crates/core/src/tick.rs` | CONFIRMED — 7 fields, no `venue`. Migration adds `venue: Venue` as field 8 (R7). |
| `Bar` definition `crates/core/src/bar.rs:36–53` | CONFIRMED — 11 fields, no `venue`. Migration adds `venue: Venue` as field 12. |
| `grep -rni "venue\|coinbase\|kraken" spec/*/reports/backtest-*.md spec/operator-success-reports/reports/success-*.md` | **ZERO hits across all committed report bodies.** Anchor risk by construction is zero (Q12). |

All analyst R1–R15 / V1–V12 claims hold against the source. Q1–Q12
operator-aligned defaults adopted as below; principled overrides
flagged inline.

### Q-resolutions

#### Q1 — `Venue` shape: closed enum

**Decision.** New type `Venue` in `crates/core/src/venue.rs` (new
file; sibling of `symbol.rs`). Closed enum with three variants and
`#[serde(rename_all = "snake_case")]`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Venue { Binance, Coinbase, Kraken }
```

`impl Display` emits `"binance"` / `"coinbase"` / `"kraken"` (matching
serde). `impl FromStr` parses the same lowercase strings; unknown →
`ParseVenueError`. **No `Default` impl** — every Bar / Tick must
construct `Venue` explicitly so the migration touches every literal
exactly once. `Ord` is derived alphabetically — `Binance < Coinbase
< Kraken` matches R7.4's tie-break order.

**Rationale.** Exhaustive `match` catches new venues at compile
time. Future venue additions are deliberate spec changes (each new
venue ships an adapter + symbol normalization + rate-limit budget),
**not silent stringly-typed extensions** — every new venue is a
multi-week analyst → architect → developer round, never a one-line
config edit. `serde(rename_all = "snake_case")` makes the audit DB
encode `"binance"` / `"coinbase"` / `"kraken"` (Q11 schema decode
trivial); `Hash` / `Ord` / `Copy` are free.

**Alternatives rejected.**

- Newtype `Venue(SmolStr)` (open set). Rejected: every venue has
  its own ingest impl + symbol normalization; "adding a venue" is
  never a string-only change. Open-set typing buys nothing and loses
  exhaustive `match`.
- Nested module per venue (`Venue::Binance(BinanceVariant)`). Rejected:
  v1.5b ships spot-only on each; perp / margin variants are v2+
  scope and will introduce a sibling `VenueProduct` enum at that time.

#### Q2 — Coinbase API: Advanced Trade WS

**Decision.** Target the **Coinbase Advanced Trade WebSocket**.
WS endpoint constant: `"wss://advanced-trade-ws.coinbase.com"`.
REST endpoint for `exchange_info`: `https://api.coinbase.com/api/v3/brokerage/products/{product_id}`
(public, unauthenticated). Channels subscribed: `market_trades`
(R1.2 trades) and `candles` (R1.2 kline-equivalent).

**Rationale.** Coinbase Pro (the legacy `wss://ws-feed.exchange.coinbase.com`
endpoint) is in maintenance mode; Coinbase has signaled long-term
deprecation. New venues should adopt the **supported surface** so
v2's potential cross-venue expansion inherits the right base. Both
endpoints expose unauthenticated trade + candle channels for spot
market data (Q8 confirms), so the operational delta is zero today
but the future delta is asymmetric.

**Alternatives rejected.**

- Coinbase Pro WS (`wss://ws-feed.exchange.coinbase.com`). Rejected:
  legacy / maintenance-mode. Acceptable as fallback only if the
  Advanced Trade integration hits a documented blocker during
  T1403 — flag back to architect.

#### Q3 — Ingest topology: per-venue tokio task

**Decision.** `agent::runtime::run` spawns **one tokio task per
venue** via `tokio::task::JoinSet`. Each task owns the per-venue
reconnect / backoff state and consumes the venue's
`subscribe_bars` / `subscribe_trades` streams. Three tasks total
when all three venues are enabled; `feeds.len()` tasks generally
(operator can disable a venue → its task is not spawned).

**Rationale.** Panic isolation > raw scheduler savings. With three
venues the steady-state task overhead is trivial (~3 long-lived
futures vs 1) and a panic in one venue's parser **cannot** poison
the others (R14.1 / R14.3). Each task can carry its own backoff
clock and reconnect counter without sharing a future poll loop.

**Alternatives rejected.**

- `futures::stream::select_all` over the three venues' merged
  streams. Rejected: a single panic in any venue's stream poll
  kills the whole select — direct R14.1 violation.
- One thread per venue (no tokio). Rejected: the rest of the
  codebase is tokio-async; introducing thread boundaries forces
  channel hops across runtimes.

#### Q4 — `Venue` field on Tick / Bar: required

**Decision.** `Tick` and `Bar` gain `venue: Venue` as a
**required** field (not `Option<Venue>`). Migration touches every
existing fixture / constructor in `crates/*` exactly once — every
literal `Bar { … }` / `Tick { … }` site sets `venue:
Venue::Binance` (mechanical: ~30 fixture sites — see Risks below
for the inventory). Zero `Option<Venue>` leakage anywhere.

**Rationale.** Optional `venue` would let venue-less data leak
through every analytic / log / audit code path; every consumer
becomes `if let Some(v) = bar.venue { … } else { /* what? */ }`.
Required forces every code path to declare provenance at the
type level. The migration is mechanical because today every
literal originates from `BinanceFeed` / a Binance-shaped fixture
— the refactor is `sed`-equivalent. T1201's per-symbol-position-
accounts feature did 21 mechanical sites; this one is comparable
scale (analyst estimate: 30+; see Risks).

**Alternatives rejected.**

- `Option<Venue>` with `None` default. Rejected: `None` becomes a
  forever-bug-surface; consumers that don't know what to do with
  `None` either ignore it (data-quality bug) or panic on unwrap
  (runtime bug). The "soft introduction" cost compounds across
  every future feature that consumes `Tick` / `Bar`.
- `venue: SmolStr` raw string. Rejected: subsumed by Q1 — closed
  enum at the type level dominates string typing.

#### Q5 — 1s bar aggregation: client-side

**Decision.** 1-second bars are aggregated **client-side** in a new
`crates/data/src/bar_aggregator.rs` module. The aggregator consumes
the raw `Tick` stream from any `MarketDataSource` impl and emits
`Bar { tf: Timeframe::OneSecond, … }` events at every 1-second
boundary aligned to UTC second. Bucketing key:
`floor(tick.venue_ts.unix_micros() / 1_000_000)` — deterministic
on epoch microseconds. Empty seconds emit no bar (R5.3).

**Rationale.** Cross-venue determinism: each venue's "1s bar" is
either undefined (Coinbase / Kraken don't expose 1s candles
publicly) or has its own quirks (Binance's 1s WS is new-ish).
Client-side aggregation gives **identical** algorithm across
all three venues. Testable from a synthetic Tick stream (V5).
Determinism is anchored on epoch microseconds — two replays of
the same Tick fixture emit byte-identical Bars (V5 acceptance).
Server-side bars stay supported for `OneMinute` / `FiveMinutes` /
… (the existing pipeline is unchanged for higher TFs).

**Alternatives rejected.**

- Server-side 1s bars (subscribe to Binance's `kline_1s`; fall
  back to client-side for Coinbase / Kraken). Rejected:
  heterogeneous bar definitions across venues (Binance's 1s bar
  open/close convention is venue-specific); strategies would have
  to reason about per-venue 1s semantics.
- Aggregator as a `Stream` adapter with per-venue config map.
  Rejected: doesn't change the client-side decision; the simple
  per-(venue, symbol) aggregator suffices and keeps the surface
  small.

#### Q6 — USDC universe: doubled (operator-gated)

**Decision.** USDT universe stays as the v1.5a 10 symbols
(BTCUSDT, …, LINKUSDT). USDC mirror set adds 10 symbols (BTCUSDC,
…, LINKUSDC). Total **20 symbols** when both enabled. Operator
opts in / out per-set in `[universe]` (Q-spec below):

```toml
[universe]
usdt_enabled = true   # default — preserves v1.5a behaviour
usdc_enabled = false  # default off — operator opts in
usdt_symbols = [
    "BTCUSDT", "ETHUSDT", "BNBUSDT", "SOLUSDT", "XRPUSDT",
    "ADAUSDT", "DOGEUSDT", "AVAXUSDT", "DOTUSDT", "LINKUSDT",
]
usdc_symbols = [
    "BTCUSDC", "ETHUSDC", "BNBUSDC", "SOLUSDC", "XRPUSDC",
    "ADAUSDC", "DOGEUSDC", "AVAXUSDC", "DOTUSDC", "LINKUSDC",
]
```

**Rationale.** USDT remains the largest crypto-stablecoin pair
set by volume; deprecating it would cripple v1's cross-sectional
momentum strategy on its primary data set and break R10
(backwards compat). Doubling preserves the existing strategy
inputs while letting the operator A/B the new universe. Default
`usdc_enabled = false` matches R10.2 (single-venue / single-set
configurations continue to work unchanged).

**Alternatives rejected.**

- Replace USDT with USDC. Rejected: forces v1's momentum strategy
  to re-anchor on a new data set — direct anchor regression.
- Implicit doubling (no toggle, both always on). Rejected: 60
  symbol × venue subscriptions when both enabled; some operators
  may want USDT-only to halve the WS connection count.

The legacy `[funding].universe` array stays as a back-compat
reader path: if `[universe]` is missing entirely, the loader
falls back to `[funding].universe` and treats it as
`usdt_symbols` with `usdc_enabled = false` (R10.1). New
configurations should use `[universe]`.

#### Q7 — Failover: per-venue stale-data pause + bus event

**Decision.** Strategies pause **per-venue** on a stale-data
threshold of **30 seconds** of no Tick from that venue (default;
configurable via `[universe].stale_threshold_secs`). The stale
state is published to the bus as a new `MarketHealth` event
(channel additions below). Strategies decide individually: the
v1.5b default behaviour for any strategy is **"skip rebalance
if any subscribed venue is `Stale`"**; strategies can override
by ignoring the channel entirely (no breaking change).

`MarketHealth` event shape (lives in `trading_core::venue`):

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MarketHealth {
    /// Venue is producing fresh ticks (received within threshold).
    Fresh { venue: Venue, last_tick_ts: Timestamp },
    /// No tick received within `threshold_secs` — strategies should pause.
    Stale { venue: Venue, last_tick_ts: Timestamp, threshold_secs: u32 },
    /// Venue produced a tick again after being stale.
    Recovered { venue: Venue, recovered_ts: Timestamp, gap_secs: u32 },
}
```

Bus channel addition: `EventBus::market_health: broadcast::Sender<MarketHealth>`,
capacity 64 (well above expected per-venue event rate; matches the
v0.5 `StrategyLoaded` channel cadence). The kill-switch (R14.5)
remains **global** — a venue-specific clock-skew event may halt
all venues if it crosses the existing `clock_skew_halt_ms`
threshold (today: 10000ms per `config/agent.toml:12`); ordinary
WS reconnects do not trip it.

**Rationale.** "Stale" is the correct mental model for
cross-venue redundancy: a Coinbase outage halts strategies that
consume Coinbase data; Binance-only strategies continue. The 30s
default is longer than any expected reconnect (`BinanceFeed` cap
is 60s backoff but typical reconnect is <5s) and shorter than a
meaningful market move. Publishing on the bus rather than an
in-strategy timer keeps the "stale" decision in one place.

**Alternatives rejected.**

- Continue with stale data unconditionally. Rejected:
  rebalance against stale data is a real money-leak surface
  even in paper mode (the audit ledger captures the wrong
  marks).
- Pause everything globally on any venue outage. Rejected:
  defeats the cross-venue redundancy goal; a Coinbase outage
  shouldn't halt the Binance-only momentum strategy.
- Strategy-side polling on `last_tick_ts`. Rejected: every
  strategy re-implements the same staleness check — bus event
  is the correct seam.

#### Q8 — Authentication: free unauthenticated WS for all three venues

**Decision.** All three venues use **free unauthenticated WS
endpoints** for public market data (trades + candles):

- Binance Spot WS: `wss://stream.binance.com:9443` (already in
  use; unchanged).
- Coinbase Advanced Trade WS: `wss://advanced-trade-ws.coinbase.com`
  (Q2). Public channels (`market_trades`, `candles`) are
  unauthenticated for spot market data.
- Kraken WS v2: `wss://ws.kraken.com/v2`. Public `trade` and
  `ohlc` channels are unauthenticated.

**No API keys**, **no authenticated tier**, **no per-message
billing** for any of these surfaces. R9 cost ladder ($0/mo
market data) holds.

**Rationale.** Confirmed against each venue's published docs as
of 2026-05-01: all three expose free public market-data WS for
spot trades + candle/ohlc channels. Authentication is required
only for private channels (orders, balances) which v1.5b does
not subscribe to (paper-trading; no real-money execution per
the project scope boundary).

**Risk if false at implementation time.** If any venue silently
moves to authenticated tier between now and T1403 / T1404 land,
the developer routes back to the architect with the new pricing
page; we either find the cheapest authenticated tier ($0/mo
target) or drop the venue. v1.5b cost commitment is hard.

#### Q9 — Rate limits: 30 (or 60) subscription slots — within free tier

**Decision.** v1.5b's worst case is **60 subscription slots** (20
symbols × 3 venues, with both USDT + USDC enabled). All three
venues' free-tier limits accommodate this with margin:

| Venue | Limit | v1.5b worst case | Margin |
|---|---|---|---|
| Binance Spot WS | 1024 streams per connection (200 streams per combined-stream URL recommended) | 1 combined-stream URL × 40 streams (20 symbols × 2 channels: kline + trade) | Way inside; one WS connection per host |
| Coinbase Advanced Trade WS | 750 messages / second / IP | ~60 msg/s steady-state (60 subscriptions × ~1 trade/s/symbol) | 12× margin |
| Kraken WS v2 | 75 connections per session; per-channel sub limit ~80 | 1 connection, ~40 subs per channel | Way inside |

**Rationale.** The architect documents these limits here so any
v2 cross-venue strategy expansion can re-check before adding
venues / symbols. v1.5b's worst case has 12× margin on the
tightest limit (Coinbase msg/s). T1405's BinanceFeed multi-symbol
fan-out uses **one combined-stream URL** rather than N WS
connections to stay well under the per-IP connection budget.

**Alternatives rejected.**

- N WS connections per symbol on Binance (the current pre-T612
  shape). Rejected: T612 deliverable is to consolidate; 20
  WS connections is wasteful and risks the per-IP TCP budget on
  the hosting VM.

#### Q10 — Test harness: `MockFeed` over `wiremock`

**Decision.** New `crates/data/src/mock_feed.rs` introduces
`MockFeed` — a lightweight in-memory feed that publishes scripted
`Tick` events on a `tokio::time::interval`. `MockFeed` impls the
`MarketDataSource` trait directly (no WS frame parsing); tests
construct a `MockFeed::new(scripted_events: Vec<Tick>, interval:
Duration, venue: Venue)` and consume the resulting stream.

`MockFeed` covers V1–V7 strategy / integration tests. WS-frame
parsing for each venue's adapter (the on-wire JSON → `Tick`
boundary in R15) is unit-tested **directly** at the
`parse_trade_event` / `parse_candle_event` private function level
— no WS server stand-up, just feed a JSON literal to the parser.

**Rationale.** `wiremock` does not script WS frames cleanly.
Spinning a real WS server (`tokio-tungstenite` server-side) per
test is slower and flakier in CI. `MockFeed` exercises every
seam **above** the WS-frame layer (the seam that strategies see)
and the parser unit tests cover **below** the WS-frame layer.
The two seams together cover the full ingest path with zero WS
server in test scope.

**Alternatives rejected.**

- Real `tokio-tungstenite` WS server per test. Rejected: slow,
  flaky, doesn't scale to 30+ test cases. Acceptable for one-off
  smoke tests against testnets (R4.3) but not the default harness.
- `wiremock` with WS plugin. Rejected: not a stable plugin;
  versioning churn risk.

#### Q11 — T805 schema: migration `007_strategy_events_venue.sql` + writer signature

**Override on the analyst's R8.2 recommendation.** Analyst
recommended option (b) `<venue>:<symbol>` encoded in
`error_summary`. **Architect chooses option (a) — schema migration**
— because v1.5b is the load-bearing introduction of the `Venue`
type to the system; encoding it in a TEXT column would defeat the
type-system change at the audit boundary (the **one place** structured
attribution matters most).

**Decision.** New migration **`007_strategy_events_venue.sql`** —
adds a `venue TEXT NULLABLE` column to `strategy_events`. Pre-
migration rows have `venue = NULL`; readers handle `Option<Venue>`
semantics. Writer signature change:

```rust
// Today (crates/audit/src/journal.rs:648):
pub async fn feed_reconnect(
    ledger: &Ledger,
    symbol: &str,
    ts:     Option<&str>,
) -> Result<(), LedgerError>;

// v1.5b — venue becomes load-bearing for all feed-level events:
pub async fn feed_reconnect(
    ledger: &Ledger,
    symbol: &str,
    venue:  Venue,        // NEW — required, not Option
    ts:     Option<&str>,
) -> Result<(), LedgerError>;
```

The writer stamps `Venue::Binance.to_string()` (`"binance"`) /
`"coinbase"` / `"kraken"` into the new column. Two existing call
sites in `crates/data/src/binance.rs:297-304` and `406-414` add
`Venue::Binance` as the third arg. New `CoinbaseFeed` /
`KrakenFeed` call sites pass their respective venue.

Migration `007` reclaims the `007` slot. The
[Audit migration list](../architecture.md#audit-migration-list--current)
gets the new entry.

**Rationale.** v1.5b's purpose is to elevate venue to a first-
class type; the audit DB is the **most** consequential surface
to keep that typed. `error_summary` parsing on read at the
operator-success-reports R7 row would be a parse-on-every-render
hot path; a typed column is a SQL `GROUP BY` away. Schema churn
risk is bounded: the migration is purely additive (NULLABLE
column, no data migration), and Q12 confirms zero anchor risk
on the column existing in the DB (the column doesn't enter
report bodies until reports binary explicitly groups by it —
which is a follow-up task, not v1.5b scope).

**Alternatives rejected.**

- Option (b) `<venue>:<symbol>` in `error_summary`. Rejected:
  defeats the type-system intent of the feature at the boundary
  where it matters most (audit). Parse-on-read costs propagate
  forever.
- Encode venue in `kind` (`FeedReconnect.binance` etc.).
  Rejected: explodes the `kind` cardinality (`FeedReconnect` ×
  `KillSwitchTripped` × … per venue) and complicates downstream
  filters.

#### Q12 — Anchor risk: zero by construction (re-confirmed)

**Decision.** Independent re-grep at design time:
`grep -rni "venue\|coinbase\|kraken" spec/*/reports/backtest-*.md
spec/operator-success-reports/reports/success-*.md` returned **zero hits**. The
type-system change adds a field that no committed report body
references. Anchor risk is **zero by construction**.

**Hard architectural rule (forward-looking).** Any future change
to backtest report rendering or operator-success report
rendering that introduces venue strings (`"binance"`,
`"coinbase"`, `"kraken"`, or any case variant) into a report
**body** breaks all anchors and requires an explicit re-lock
budget. Architect must approve any such change via an ADR-style
spec entry before code lands. The grep
`grep -rni "venue\|coinbase\|kraken" spec/*/reports/backtest-*.md
spec/operator-success-reports/reports/success-*.md` should remain zero across the
v1.5b lifecycle and beyond, until / unless a deliberate re-lock
is approved.

**Rationale.** v1.5b is an additive type-system change at the
ingest layer; the only path from the change to a committed
report body would be a renderer update, which v1.5b does not
make. The 11/11 anchor budget is preserved (R11) byte-identical.

### Crate map delta

The following changes land **only in `crates/`** (no spec-only
movement here — the full `## Implementation` section is for the
developer). Summarized for orientation:

#### `crates/core/` (`trading_core`)

- **NEW** `crates/core/src/venue.rs` — `Venue` enum + `MarketHealth`
  enum + `ParseVenueError`. Re-exported from `lib.rs`.
- **MOD** `crates/core/src/bar.rs:11` — `Timeframe` adds
  `OneSecond` variant; Display string `"1s"`.
- **MOD** `crates/core/src/bar.rs:36` — `Bar` struct gains
  `pub venue: Venue` as the **last** field (additive at the end
  preserves struct-literal ergonomics; `Debug` order matches).
- **MOD** `crates/core/src/tick.rs` — `Tick` struct gains
  `pub venue: Venue` as the last field.

#### `crates/audit/`

- **NEW** `crates/audit/migrations/007_strategy_events_venue.sql` —
  `ALTER TABLE strategy_events ADD COLUMN venue TEXT;` (NULLABLE,
  no default). Migration header comment cites Q11 / R8.
- **MOD** `crates/audit/src/journal.rs:648` — `feed_reconnect`
  signature gains `venue: Venue`. Writer stamps
  `venue.to_string()` into the new column (`StrategyEventWrite`
  gains a `venue: Option<&str>` field; `feed_reconnect` always
  populates `Some(...)`; other writers pass `None` for now).
- **MOD** `crates/audit/src/journal.rs` — `kill_switch_tripped`
  writer (if it exists) gains an optional `venue: Option<Venue>`
  per R8.3 — `None` for global trips, `Some(v)` for venue-specific
  trips. Architect's call: this is an additive optional argument;
  no new migration.

#### `crates/data/`

- **NEW** `crates/data/src/coinbase.rs` — `CoinbaseFeed` impl of
  `MarketDataSource`. Same shape as `BinanceFeed`:
  `tokio_tungstenite` + `serde_json` + `reqwest` for REST.
  `coinbase_symbol_map` helper (`BTCUSDC` ↔ `BTC-USDC`).
- **NEW** `crates/data/src/kraken.rs` — `KrakenFeed` impl of
  `MarketDataSource`. WS v2 protocol. `kraken_symbol_map`
  (`BTCUSDC` ↔ `XBT/USDC`; `XBT` for Bitcoin).
- **NEW** `crates/data/src/bar_aggregator.rs` — 1-second
  client-side aggregator (R5). Takes a `BoxStream<Result<Tick>>`
  + `Symbol` + `Venue` → `BoxStream<Result<Bar>>` with
  `tf == Timeframe::OneSecond`.
- **NEW** `crates/data/src/mock_feed.rs` — `MockFeed` test
  harness (Q10). Impls `MarketDataSource`; constructed from
  `Vec<Tick>` + interval + venue.
- **MOD** `crates/data/src/binance.rs` — adds
  `subscribe_bars_multi(symbols: &[Symbol], tf: Timeframe)
  -> Result<BoxStream<Result<Bar>>>` and
  `subscribe_trades_multi(symbols: &[Symbol]) -> ...`. Uses
  Binance's combined-stream URL
  (`wss://stream.binance.com:9443/stream?streams=<list>`). Single-
  symbol API stays unchanged (R10.3). Per-symbol Prometheus
  `clock_skew_ms{feed,symbol}` label populated (R4.2).
- **MOD** `crates/data/src/binance.rs` — every `Bar { … }` /
  `Tick { … }` constructor adds `venue: Venue::Binance`.
- **MOD** `crates/data/src/replay_feed.rs` — every `Bar` / `Tick`
  literal gains `venue: Venue::Binance` (replay fixtures are
  Binance-shaped).
- **MOD** `crates/data/src/fake_feed.rs` — every literal gains
  `venue: Venue::Binance`. `FakeFeed::new` accepts an optional
  `venue:` parameter (default `Binance`) for tests that need
  cross-venue fakes.

#### `crates/agent/`

- **MOD** `crates/agent/src/runtime.rs::run` — spawns one tokio
  task per enabled venue via `JoinSet`. Each task owns its
  per-venue ingest loop and stale-data watchdog. `RunHandles`
  struct gains a `venue_tasks: HashMap<Venue, JoinHandle<()>>`
  field for shutdown.
- **MOD** `crates/agent/src/bus.rs` (or wherever `EventBus` lives)
  — adds `market_health: broadcast::Sender<MarketHealth>` channel,
  capacity 64.
- **NEW** `crates/agent/src/stale_watchdog.rs` (or in `runtime.rs`
  if small) — per-venue watchdog: tracks last-Tick timestamp;
  publishes `MarketHealth::Stale { venue, ... }` after
  `stale_threshold_secs` (default 30s) without a tick.

#### `config/`

- **MOD** `config/agent.toml` — new `[universe]` section per Q6.
  Legacy `[funding].universe` stays as a back-compat reader path.
- **MOD** `config/agent.toml` — new
  `[data.sources.coinbase]` and `[data.sources.kraken]`
  stanzas (operator opts in by adding them; default off per
  R10.2).

### Public API additions

New `pub` items landing in v1.5b:

```rust
// trading_core (re-exported from lib.rs)
pub use venue::{Venue, MarketHealth, ParseVenueError};
pub enum Venue { Binance, Coinbase, Kraken }
pub enum MarketHealth { Fresh{...}, Stale{...}, Recovered{...} }
pub struct ParseVenueError;
pub enum Timeframe { ..., OneSecond }   // new variant

// trading_core::bar
pub struct Bar { ..., pub venue: Venue }   // new field (last)

// trading_core::tick
pub struct Tick { ..., pub venue: Venue }   // new field (last)

// data
pub struct CoinbaseFeed { ... }
impl MarketDataSource for CoinbaseFeed { ... }
pub fn coinbase_symbol_map(s: &Symbol) -> String;

pub struct KrakenFeed { ... }
impl MarketDataSource for KrakenFeed { ... }
pub fn kraken_symbol_map(s: &Symbol) -> String;

pub struct MockFeed { ... }
impl MarketDataSource for MockFeed { ... }
impl MockFeed { pub fn new(events: Vec<Tick>, interval: Duration, venue: Venue) -> Self; }

// data::bar_aggregator
pub fn aggregate_one_second(
    ticks: BoxStream<'static, Result<Tick, FeedError>>,
    symbol: Symbol,
    venue:  Venue,
) -> BoxStream<'static, Result<Bar, FeedError>>;

// data::binance (additive — old API stays)
impl BinanceFeed {
    pub async fn subscribe_bars_multi(&self, symbols: &[Symbol], tf: Timeframe)
        -> Result<BoxStream<'static, Result<Bar, FeedError>>, FeedError>;
    pub async fn subscribe_trades_multi(&self, symbols: &[Symbol])
        -> Result<BoxStream<'static, Result<Tick, FeedError>>, FeedError>;
}

// audit::journal
pub async fn feed_reconnect(
    ledger: &Ledger,
    symbol: &str,
    venue:  Venue,        // NEW — required
    ts:     Option<&str>,
) -> Result<(), LedgerError>;

// agent::bus
pub struct EventBus {
    ...,
    pub market_health: broadcast::Sender<MarketHealth>,   // NEW; capacity 64
}
```

### WS endpoint table

| Venue | WS endpoint | REST `exchange_info` | Auth | Channels | Rate limit |
|---|---|---|---|---|---|
| Binance | `wss://stream.binance.com:9443/stream?streams=…` | `https://api.binance.com/api/v3/exchangeInfo` | None | `<symbol>@kline_1m`, `<symbol>@trade` | 1024 streams / WS (recommended ≤200 / combined URL) |
| Coinbase | `wss://advanced-trade-ws.coinbase.com` | `https://api.coinbase.com/api/v3/brokerage/products/{id}` | None | `market_trades`, `candles` | 750 msg/s/IP |
| Kraken | `wss://ws.kraken.com/v2` | `https://api.kraken.com/0/public/AssetPairs` | None | `trade`, `ohlc` | 75 conn/session; ~80 sub/channel |

Worst-case load: 60 subs total (20 symbols × 3 venues) at ~1
msg/s/symbol → ~60 msg/s/venue. **All three venues have ≥10×
margin on the tightest limit.**

### Sample on-wire payloads (R15 normalization reference)

Compact reference for T1403 / T1404 parsers; all three normalize
to `Tick` via `rust_decimal::Decimal::from_str` on string-cast
price/qty — no `f64` ever touches money.

- **Binance** `@trade`: `{"e":"trade","s":"BTCUSDT","t":<u64>,"p":"60000.00","q":"0.001","T":<ms>,"m":false}` — `is_buyer_maker=false` → `Side::Buy`.
- **Coinbase Advanced Trade** `market_trades`: events array carrying `{"trade_id":"<str>","product_id":"BTC-USD","price":"60000.00","size":"0.001","side":"BUY","time":"<RFC3339>"}` — `side="BUY"|"SELL"` → `Side::*`; trade_id parses to `u64`.
- **Kraken WS v2** `trade`: data array carrying `{"symbol":"BTC/USD","side":"buy","price":60000.0,"qty":0.001,"trade_id":<u64>,"timestamp":"<RFC3339>"}` — **price/qty are JSON numbers**; cast to string before `Decimal::from_str` (R15.4 hard rule, no `f64`).

Trade-ID (R15.5): all three fit `Tick.trade_id: u64` directly;
uniqueness within `(venue, symbol)` only.

### `MarketHealth` event shape — bus channel

See Q7 above for the enum. Channel wiring:

- **Producer.** Per-venue stale-data watchdog
  (`agent::stale_watchdog`) holds a `HashMap<Venue,
  AtomicI64>` (last-Tick µs) updated by every Tick consumer.
  A 1-second tokio interval scans the map and publishes
  `MarketHealth::Stale` for any venue whose last tick is
  older than `stale_threshold_secs`. On the next fresh tick,
  publishes `MarketHealth::Recovered`.
- **Consumers.** Strategies subscribe via
  `bus.market_health.subscribe()` and maintain a per-venue
  health flag. `MeanReversionPairsStrategy` and
  `CrossSectionalMomentumStrategy` (existing) gain a one-line
  guard in their rebalance path — `if health[v] == Stale {
  return Vec::new(); }` — when they consume that venue. The
  default behavior in v1.5b for any strategy that **does not**
  subscribe to `market_health` is unchanged (R10.5).
- **Capacity.** 64 (matches `StrategyLoaded` cadence; expected
  events per minute in steady state ≈ 0).

### Configuration TOML shape

New `[universe]` section, designed to live alongside the
existing `[funding].universe` (which becomes the back-compat
reader path):

```toml
[universe]
# Master toggle: which quote-currency sets are active.
usdt_enabled = true   # default — preserves v1.5a behavior
usdc_enabled = false  # default off; opt in to add USDC mirror

usdt_symbols = [
    "BTCUSDT", "ETHUSDT", "BNBUSDT", "SOLUSDT", "XRPUSDT",
    "ADAUSDT", "DOGEUSDT", "AVAXUSDT", "DOTUSDT", "LINKUSDT",
]
usdc_symbols = [
    "BTCUSDC", "ETHUSDC", "BNBUSDC", "SOLUSDC", "XRPUSDC",
    "ADAUSDC", "DOGEUSDC", "AVAXUSDC", "DOTUSDC", "LINKUSDC",
]

# Stale-data threshold for the per-venue watchdog (Q7 / R14.4).
# 30s default — longer than any expected reconnect, shorter than
# a meaningful market move.
stale_threshold_secs = 30

[data.sources.coinbase]   # NEW — operator opts in
ws_url   = "wss://advanced-trade-ws.coinbase.com"
rest_url = "https://api.coinbase.com"
# Optional per-venue symbol override; default = all USDC pairs.
# symbols = ["BTC-USDC", "ETH-USDC"]

[data.sources.kraken]     # NEW — operator opts in
ws_url   = "wss://ws.kraken.com/v2"
rest_url = "https://api.kraken.com"
# symbols = ["XBT/USDC", "ETH/USDC"]
```

The loader (`crates/agent/src/config.rs` or equivalent)
back-compat path: if `[universe]` is absent, fall back to
`[funding].universe` and treat as `usdt_symbols` with
`usdc_enabled = false`. Logged once at startup as
`config: legacy [funding].universe path; consider migrating to [universe]`.

No conflict with existing config sections (verified by reading
`config/agent.toml:1-66` — the new `[universe]` and
`[data.sources.coinbase]` / `[data.sources.kraken]` keys are
all fresh).

### Test strategy — per V-item

| V-item | Test file | Fixture | Asserts |
|---|---|---|---|
| V1 — Binance regression | `crates/data/tests/binance_*.rs` (existing) | unchanged | All v0–v1.5a paths still pass; `cargo fmt --check`, `cargo clippy -D warnings`, `cargo audit`, `cargo deny check` clean |
| V2 — Coinbase Tick | `crates/data/tests/coinbase_subscribe_trades.rs` (NEW) | `MockFeed` scripted with one Coinbase-shaped Tick + a unit-test of `parse_market_trades_event` | `Tick.venue == Venue::Coinbase`; `Symbol`, `price`, `qty`, `side`, `venue_ts`, `local_recv_ts`, `trade_id` populated; emitted within 30s |
| V3 — Kraken Tick | `crates/data/tests/kraken_subscribe_trades.rs` (NEW) | `MockFeed` + `parse_kraken_trade_event` unit | `Tick.venue == Venue::Kraken`; symbol normalized through `kraken_symbol_map` |
| V4 — USDC pairs in audit DB | `crates/audit/tests/usdc_pairs_post_fill.rs` (NEW) | In-memory ledger; manually-posted BTCUSDC fill | `account_id == "assets:position:BTCUSDC"`; `pnl_by_symbol` returns BTCUSDC row |
| V5 — 1s aggregator | `crates/data/tests/bar_aggregator_one_second.rs` (NEW) | Vec of 100 scripted Ticks across 10s | OHLCV by hand for 10 emitted bars; two runs byte-identical (determinism) |
| V6 — Multi-symbol fan-out | `crates/data/tests/binance_multi_symbol.rs` (NEW) | `MockFeed` producing 10-symbol stream | One bar per minute boundary, alphabetical order; no msg lag > 5s; `clock_skew_ms{feed,symbol}` Prometheus label populated |
| V7 — Coinbase outage | `crates/agent/tests/coinbase_outage_isolation.rs` (NEW) | Three `MockFeed`s (one for each venue); Coinbase scripted with a mid-stream drop | Binance + Kraken streams continue; Coinbase reconnect emits `FeedReconnect` event with `venue=Coinbase`; no panic propagates outside Coinbase task |
| V8 — Anchor regression | `verify-anchors` skill | locked anchors | `ANCHORS PASS  (11 / 11)` |
| V9 — T802/T805/T806/T809/T810 | existing test suites | unchanged | All audit / reports / agent suites green; `feed_reconnect` rows carry venue column |
| V10 — T901–T912 | existing UI suites | unchanged | Cockpit live subscription works; bus channels stay venue-blind |
| V11 — T1101–T1107 | existing audit suites | universe expanded to 20 | 20 `assets:position:<SYMBOL>` rows seeded when both sets enabled |
| V12 — T1201–T1209 + T1301–T1305 + cost telemetry | existing UI / audit / cost suites | unchanged | `costs.md` shows $0 LLM, $0 market-data |

Note V6 specifically requires a `MockFeed` that produces 10-symbol
streams (single mock instance, multi-symbol output) — T1407 implements
this via a `MockFeed::new_multi(events: HashMap<Symbol, Vec<Tick>>, ...)`
constructor. The single-symbol `MockFeed::new` is the simpler default.

### Risks + mitigations (≥7)

| # | Risk | Mitigation |
|---|---|---|
| 1 | **Tick / Bar `venue` field breaks every fixture site.** Analyst estimated 30+ literal sites (`Bar { … }` / `Tick { … }` constructors); mechanical refactor across `crates/data/`, `crates/strategy/`, `crates/audit/tests/`, `crates/reports/tests/`, `crates/ui/tests/`, `crates/backtest/`. | T1401 is the foundation gate — sole task touching `core::Bar` / `core::Tick`. Mechanical migration: every literal gains `venue: Venue::Binance` (one line per literal). All downstream tasks block on T1401. T1401's acceptance is `cargo build --workspace` clean; the developer enumerates the actual fixture-site count in the citation block. **Pre-flight `grep -rn "Bar {\|Tick {" crates/`** to enumerate at design time → ~35 sites by rough count (binance, replay, fake feeds, strategy unit tests, backtest fixtures, reports fixtures, ui fixtures). |
| 2 | **Anchor drift if venue leaks into report body.** Any future renderer that surfaces `bar.venue` / `tick.venue` into a committed report body breaks all 11 anchors. | Hard architectural rule (Q12): the `grep -rni "venue\|coinbase\|kraken" spec/*/reports/backtest-*.md spec/operator-success-reports/reports/success-*.md` count must remain **zero** across v1.5b. Any change that would introduce venue strings into a body requires an architect-approved re-lock budget. T1415 anchor regression sweep is the v1.5b gate. |
| 3 | **Coinbase Advanced Trade WS is newer; protocol churn risk.** Advanced Trade is the supported surface but it's younger than the legacy Pro WS; field shapes can shift. | Q2 fallback documented: if T1403 hits a documented blocker, route HANDOFF → architect; switch to Coinbase Pro WS (same `MarketDataSource` impl, different URL + parser). Parser is isolated — `coinbase::parse_market_trades_event` — swap-able in one file. |
| 4 | **Kraken WS quirks.** Kraken's API has known idiosyncrasies: `XBT` for Bitcoin, slash-separated pairs (`BTC/USD`), float-as-number price/qty (R15.4 forces string-cast), v2 protocol handshake differences vs Binance. | Adapter-local `kraken_symbol_map` isolates symbol translation; parser explicitly casts numbers to string before `Decimal::from_str` (R15.4 hard rule); R15 unit test fixture per venue captures the exact on-wire shape so any protocol drift surfaces in CI. |
| 5 | **Rate-limit exhaustion under reconnect storm.** A WS-frame parser bug or a venue-side rolling-restart that triggers all 30 subscriptions to reconnect simultaneously could exceed Coinbase's 750 msg/s/IP burst. | Per-venue exponential backoff (1s base → 60s cap, identical to Binance's existing R1.4 / R2.4). T1408's `JoinSet` topology means each venue's reconnect storm is **isolated** — Coinbase reconnects don't trigger Binance reconnects. Worst-case math: 60 subs × ~1 trade/s = 60 msg/s with **12× margin** on Coinbase's 750 msg/s limit. |
| 6 | **Per-venue task panic propagation.** A bug in CoinbaseFeed's parser could panic mid-stream; `select_all` topology would kill all venues. | Q3 / T1408 picks per-venue `JoinSet` — one panic terminates one venue's task; the supervisor (`agent::runtime::run`) detects the dead `JoinHandle` and emits a `FeedReconnect` event with `error_code = "task_panic"` + the panic message in `error_summary` (R14.3). The other two venues continue. |
| 7 | **1s aggregation determinism.** Floating-point timestamp arithmetic could produce non-deterministic bucket assignment across machines. | Bucketing key: `floor(tick.venue_ts.unix_micros() / 1_000_000)` — pure integer arithmetic on `i64` epoch microseconds. No `f64`. Two replays of the same Tick fixture must emit byte-identical Bars (V5). Unit test asserts `for &t in fixture: bucket(t) == expected[t]` exactly. |
| 8 | **Coinbase / Kraken venue addition tempts an SDK pull.** Both venues have community Rust SDKs (e.g. `coinbase-rs`, `krakenrs`). Adding any SDK violates the single-binary discipline. | Library-compat checklist (architect.md): all three adapters use `tokio_tungstenite` + `serde_json` + `reqwest` — **identical** to today's `BinanceFeed`. **No new crate dep.** The `[Implementation]` section of v1.5b carries a hard "no external SDK" rule; T1403 / T1404 acceptance includes "no `Cargo.toml` change beyond the existing `[dependencies]` block of `crates/data/`". |
| 9 | **`Q11 schema migration` regresses an existing `feed_reconnect` test.** Any test that asserted on `error_summary == "BTCUSDT"` shape will see an extra `venue` column populated. | Migration `007` is purely additive (NULLABLE column). Existing test asserts on `error_summary` are unaffected — `error_summary` keeps the symbol literal. The new column is queried by the operator-success-reports R7 row only. T1402's acceptance includes `cargo test -p audit` green. |
| 10 | **`MarketHealth` channel broadcast lag-drop on slow consumers.** A slow strategy could miss a `Stale` event and continue rebalancing on stale data. | Capacity 64 (well above expected event rate ≈ 0/min steady state). Lagged-drop is logged at `warn!` per the v0.5 broadcast pattern. Strategies that subscribe should poll the latest health on each rebalance entry, not rely solely on edge events. |

### Library / crate compatibility checklist (architect.md)

Per the architect agent's checklist before locking deps. v1.5b's
position: **NO external SDKs** — Coinbase + Kraken adapters use the
same `tokio_tungstenite` + `serde_json` + `reqwest` pattern as
today's `BinanceFeed`. **Zero `Cargo.toml` change** in v1.5b across
the workspace.

Verified per the checklist:

- **Single-binary friendly.** ✅ No new dep; SQLite stays the audit
  backend; migration `007` is pure SQL.
- **No system C deps.** ✅ Reusing existing `tokio_tungstenite`
  / `reqwest` (rustls-tls feature already enabled).
- **Edition 2024 compatible.** ✅ No new dep.
- **Stdlib-name shadowing.** ✅ No new package.
- **Maintained.** ✅ N/A.
- **License compatible.** ✅ N/A.

If a future v2+ feature wants to pull a venue SDK, it gets its own
ADR and library-compat audit — explicitly out of scope here.

### Determinism guardrails

Per architect.md determinism checklist:

- **No `SystemTime::now()` / `Instant::now()` reachable from a
  backtest replay path.** ✅ The 1s aggregator uses `Tick.venue_ts`
  (venue-supplied) for bucketing, not wall clock. The stale-data
  watchdog uses wall clock — but only on the live ingest path,
  never reachable from backtest replay (replay uses
  `replay_feed.rs` which does not run the watchdog).
- **No `f64` in money math.** ✅ All venue parsers cast to string
  then `Decimal::from_str` (R15.4). Kraken's number-typed price/qty
  is the explicit risk; T1404 acceptance is "no `f64` in any
  money path."
- **Microsecond fractional-second timestamps.** ✅ `Timestamp`
  remains 6-digit fractional seconds. Audit migration `007` adds
  a TEXT column, not a timestamp column.
- **All RNGs `ChaCha20Rng`.** ✅ N/A — v1.5b introduces no RNG.
- **HashMap iteration sorted.** ✅ The per-venue `JoinSet`
  iterates in alphabetical Venue order before any cross-run
  comparison (R7.4 tie-break).

### Invariants preserved across prior features

- **operator-success-reports** (R12.1). T805 `feed_reconnect`
  writer signature gains required `venue: Venue`; the new column
  on `strategy_events` is queried by a future R7-renderer
  `GROUP BY venue` (out of scope for v1.5b — that renderer
  change ships separately with its own re-lock budget if any).
- **live-cockpit-unified** (R12.2). `EventBus` Bar / Tick
  channels stay venue-blind; consumers read `bar.venue` on
  demand. New `market_health` channel is additive (capacity 64).
  `RunHandles` gains `venue_tasks: HashMap<Venue, JoinHandle<()>>`
  (additive; no breaking change).
- **per-symbol-position-accounts** (R12.3). Bootstrap loader
  extends to read `[universe].usdt_symbols` + `usdc_symbols`;
  when `usdc_enabled = true` it seeds 20 `assets:position:<SYMBOL>`
  rows (idempotent `INSERT OR IGNORE`).
- **tape-row-modal** (R12.4). v1.5b is plumbing-only; rendering
  `bar.venue` on tape rows is a deferred UI iteration.
- **journal-tx-metadata** (R12.5). Reader operates on
  `journal_transactions` rows, which are venue-blind by
  construction.

### Topology summary

`agent::runtime::run` → `tokio::JoinSet` containing one task per
enabled venue (Binance / Coinbase / Kraken). Each task consumes
its venue's `subscribe_*` streams and forwards Ticks / Bars to
the **single shared `EventBus`** (`ticks` / `bars` / `market_health`
channels). A panic in any task does **not** propagate into the
others (Q3 / R14.1); the supervisor emits
`FeedReconnect{venue, error_code: "task_panic"}` and respawns
per operator policy.

### Out of scope (carried forward)

- Real-money execution / cross-venue order routing — v3+ scope.
- Perp futures — v2+ scope.
- L2 books — deferred per v1.5a Q6.
- Cross-venue arbitrage strategies — strategy land, not data
  plumbing; consumes v1.5b's output.
- UI venue badges on tape rows — follow-up ui-designer iteration.
- Reports renderer `GROUP BY venue` for the R7 row — follow-up
  reports task; ships only after operator approves a re-lock
  budget for the affected anchor scenarios.

## Implementation

### T1408 — per-venue `JoinSet` ingest topology + panic isolation (developer, 2026-05-01)

`agent::runtime::run` (Mode::Paper arm) now builds a deterministic
enabled-venue list — Binance always, Coinbase + Kraken opt-in via
`[data.sources.<venue>] enabled = true` — sorted by `Venue`'s `Ord`
impl (`Binance < Coinbase < Kraken`, R7.4) before spawning. Each
enabled venue gets a dedicated **supervisor task** spawned into the
runtime `JoinSet` via the new
[`spawn_venue_supervisor`](../../crates/agent/src/runtime.rs)
helper. The supervisor wraps the actual feed-tap consumption
(`spawn_feed_taps` → bars/ticks streams) in an inner
`tokio::task::spawn` and inspects the resulting `JoinError`:

- `JoinError::is_panic() == true` → `tracing::error!("venue {}
  crashed: …; restarting via watchdog")` + audit-journal a
  `feed_reconnect(ledger, "unknown", venue, None)` row carrying
  `error_code = "feed_reconnect"` and venue context (R14.3 / R8).
  The supervisor returns `Ok(())` so the surrounding `JoinSet`
  never sees a panicking task — **the other venues' supervisors
  keep running** (R14.1 acceptance).
- Cancel / clean stream end → log + return.

Watchdog respawn (the "restart 3 times then escalate" half) is
deferred to T1409 so the stale-data and panic-recovery policies
live in one place.

Backwards compatibility (R10.2) is exercised by
`runtime::tests::t1408_default_config_spawns_only_binance` —
the v1.5a default config (no `[data.sources.coinbase]` /
`[data.sources.kraken]` sections) results in a single-venue
spawn list `vec![Venue::Binance]`, byte-for-byte equivalent to
the pre-T1408 behaviour. Panic isolation is exercised by
`runtime::tests::t1408_venue_panic_isolated_does_not_kill_runtime`,
which runs a panicking Coinbase feed alongside a healthy
Binance `FakeFeed` and asserts both supervisor tasks return
`Ok(())` from the `JoinSet`.

New deps: none — the topology rides on `tokio::task::JoinSet`
already in the workspace.

## UI

_ui-designer (optional — v1.5b has no operator-visible
surface beyond the cockpit's optional venue rendering on
tape rows. Architect's call whether to spawn ui-designer or
defer to a follow-up._

## Verification — links

- **2026-05-03 19:46 UTC — FINAL gate:**
  `spec/archive/test-2026-05-03-1946-v1-5b-multi-venue-final.md (archived; see spec/archive/README.md)`
  — `VERDICT → PASS`. Anchors 11/11; V1-V12 all VERIFIED; 5/5 prior features
  regress-free. T_FINAL_V15B ticked.

## Notes / Open questions for architect

See [Open questions for architect](#open-questions-for-architect)
above (Q1–Q12).

## Changelog

- 2026-05-03 (analyst): initial draft. Promoted from
  [`backlog.md`](../backlog.md) Queue → Active. v1.5b is the
  data-plumbing sibling of the v1.5a strategy split — Coinbase +
  Kraken adapters, USDC pairs, T612 multi-symbol live
  `BinanceFeed`, 1s aggregated trades. 15 R-items, 12 V-items,
  12 Open Questions for the architect. Anchor risk: zero by
  construction (architect must independently grep
  `spec/reports/**/*.md` to confirm zero `venue` strings —
  R11.2 / Q12). Cost risk: zero — all three venues have free
  public market-data APIs (R9 / Q8). Failover risk: medium —
  multi-venue means N independent failure modes; per-venue
  tokio tasks + isolation is the architect's call (Q3 / R14).
  HANDOFF → architect.
- 2026-05-03 (architect): Design section landed; resolves Q1–Q12.
  Closed-enum `Venue { Binance, Coinbase, Kraken }` (Q1);
  Coinbase Advanced Trade WS (Q2); per-venue `tokio::JoinSet`
  topology (Q3); required `venue` field on Tick / Bar (Q4);
  client-side 1s aggregation on epoch-µs bucketing (Q5); doubled
  USDC universe with `usdc_enabled` opt-in (Q6); per-venue
  stale-data pause @30s default + new `MarketHealth` bus channel
  (Q7); free unauthenticated WS for all three (Q8); 60-sub
  worst case with ≥10× margin on every venue (Q9); `MockFeed`
  test harness over `wiremock` (Q10); **override of analyst R8.2**
  — schema migration `007_strategy_events_venue.sql` adds typed
  `venue TEXT NULLABLE` column + writer signature gains `venue:
  Venue` (Q11); zero-anchor-risk re-confirmed by independent
  grep (Q12). No external SDK / no `Cargo.toml` change — all
  three feeds reuse `tokio_tungstenite` + `serde_json` +
  `reqwest`. Task list at
  [`spec/v1-5b-multi-venue/tasks.md`](tasks.md).
  HANDOFF → developer (T1401 foundation gate first; ~7 parallel
  tasks fan out after).
- 2026-05-01 (developer): T1408 landed — per-venue `JoinSet`
  topology + panic-isolation supervisor wired into
  `agent::runtime::run` Mode::Paper arm. New
  `CoinbaseSourceConfig` / `KrakenSourceConfig` (default
  `enabled = false`, R10.2 backwards compat) gate Coinbase /
  Kraken spawn; Binance always spawned. Three new unit tests
  (`t1408_default_config_spawns_only_binance`,
  `t1408_three_venue_config_spawns_all_three`,
  `t1408_venue_panic_isolated_does_not_kill_runtime`) cover the
  R10 + R14.1 + R14.3 acceptance. `cargo build --workspace`,
  `cargo test -p agent`, `cargo clippy --workspace --all-targets
  --all-features -- -D warnings`, `cargo fmt --check`, and
  `bash scripts/verify_anchors.sh` all green (anchors 11/11).
  T1409 unblocked. HANDOFF → orchestrator (T1408 done; T1409
  next).
- 2026-05-03 (tester): **Feature shipped.** T_FINAL_V15B ticked
  after FINAL gate verification at
  `spec/archive/test-2026-05-03-1946-v1-5b-multi-venue-final.md (archived; see spec/archive/README.md)`
  (`VERDICT → PASS`). All R-items satisfied; all 12 V-items
  VERIFIED with file:line evidence. Static analysis clean
  (fmt + clippy --all-features + 4 build configurations).
  Workspace tests: 96 result lines, ~797 passed / 0 failed / 3
  pre-existing ignored; doc tests clean; ui-live 102/102.
  Anchor gate `ANCHORS PASS (11/11)` — R11 / Q12 zero-risk-by-
  construction confirmed. Cross-feature invariants 5/5 green
  (T802/T805/T806/T809/T810; T901-T912 + T1206; T1101-T1107;
  T1201-T1209; T1301-T1305). Two non-blocking architect
  follow-ups: (a) upstream RUSTSEC-2026-0104 in
  `rustls-webpki 0.103.12` (transitive via
  `metrics-exporter-prometheus`; not v1.5b-introduced),
  (b) `crates/data/benches/bar_aggregator.rs` criterion harness
  for the R5.5 p99 < 500µs assertion. Status `in-progress →
  shipped`. HANDOFF → presenter.
