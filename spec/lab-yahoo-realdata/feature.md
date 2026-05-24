---
slug: lab-yahoo-realdata
version: 0.1.0
status: proposed
owner: analyst
updated: 2026-05-24
parent: lab-end-to-end-v2
---

# Lab Yahoo realdata — multi-asset pivot, replace Binance for Lab

> **Operator decision 2026-05-24** (verbatim): "Replace Binance for Lab —
> multi-asset pivot." Promoted from Idea → Active in
> [`spec/backlog.md`](../backlog.md) under the same date.
>
> **Predecessor chain**: this brief sits downstream of
> [`backtest-real-binance-data v0.1.0`](../backtest-real-binance-data/feature.md)
> (shipped 2026-05-18; locked the parquet revision-pin protocol per
> [ADR-0032](../architecture/adr/0032-backtest-realdata-path-and-revision-pin.md)),
> and downstream of [`lab-end-to-end-v2 v0.1.0`](../lab-end-to-end-v2/feature.md)
> Wave D-2 (single-symbol dispatch arms shipped 2026-05-24 — `v0.sma`,
> `v0.5.macd`, `v0.5.rsi`, `v0.5.bbands` now respect `cfg.pair.1` in
> `engine::run_scenario`). The Lab's "pair × strategy" UX is now real
> for single-symbol strategies; this brief swaps the underlying data
> source for that UX from synthetic GBM to Yahoo-Finance-cached
> historical OHLCV.

## Why

### Operator request

The Lab is the operator-facing experimentation surface. Today it has
three real-data paths:

1. **Synthetic GBM** (`ChaCha20Rng` per-pair seed; default for fixtures
   cockpit). Fast iteration; F11 mixes the pair into the seed for
   per-pair variation. Operator-acceptable for UX wiring; useless for
   actually evaluating a strategy.
2. **Binance parquet cache** (`data/binance/<SYM>/<YEAR>/*.parquet`;
   revision SHA `3a8b96c43f2d8980fd8039303197ff3ac5d01e8f9cebaecdf74c853622dbbfc7`;
   10 USDT pairs × 2023+2024 hourly bars). Wired into the
   `crates/backtest` CLI via `--features realdata`; **NOT wired to Lab
   dispatch today** — `engine::run_scenario` accepts no `--features`-
   gated bar source, and the Lab path constructs its own
   `synthetic_bars_hourly` upstream of dispatch.
3. (this feature) **Yahoo Finance via Rust crate** — fetched on demand,
   cached on disk in a parquet layout that mirrors (2), and dispatched
   into the same `engine::run_scenario` arms the synthetic path uses.

The operator's "replace Binance for Lab" decision means: **path (3)
becomes the Lab's real-data option; path (1) stays as the fast-
iteration default; path (2) becomes a CLI-only legacy path** (anchored
Binance-based reports stay as immutable evidence — the 34/34 anchor
invariant is non-negotiable per
[`spec/anchors.toml`](../anchors.toml) — but Lab dispatch no longer
calls path (2)).

### Why Yahoo, why now

- **Multi-asset universe**. Binance is crypto-USDT-only. Yahoo covers
  crypto (`BTC-USD`, ...), equities (`AAPL`, `SPY`, ...), FX
  (`EURUSD=X`, ...), commodities (`GC=F`, `CL=F`). The Lab's
  strategy roster (SMA / MACD / RSI / BBands — all bar-cadence-
  agnostic, all per-symbol) is sector-agnostic by construction; the
  data source has been the constraint.
- **Decade-scale history**. Binance covers ~5 years of crypto;
  equities + FX go back 30+ years on Yahoo. Lab's H1/H2/full-year
  presets become more interesting with multi-decade reach.
- **License + cost**. Yahoo is free at the rate-limited unofficial-
  API tier. No exchange membership, no API key for the historical
  endpoint. ADR-0032's revision-pin protocol generalises.
- **One source of cost** (K4): Yahoo data is occasionally revised
  upstream (corporate actions, ticker remaps), and the unofficial
  API has no SLA. Both mitigated by the revision-pin protocol +
  retry logic; see K1, K2, K4 below.

### What "replace Binance for Lab" does NOT mean

- **The 34 locked anchors stay byte-identical.** Binance-based
  anchored reports (the 4 single-symbol legacy SHAs:
  `btc-2023-1m-sma-cross`, `btc-2023-1m-macd-trend`,
  `btc-2023-1m-rsi-reversion`, `btc-2023-1m-bbands-mean-revert`; plus
  the cross-sectional + TCN anchored families) remain historical
  evidence. This feature emits NEW Yahoo-based anchors at a future
  M-FINAL (architect-decide whether under separate scenario IDs or
  appended to the existing `spec/anchors.toml` registry); the
  existing 34 are not touched.
- **The CLI Binance path stays usable.** `cargo run --features
  realdata --bin backtest -- <scenario>` still works for the
  4 anchored Binance scenarios + the cross-sectional + TCN
  scenarios. Lab simply no longer dispatches into that path.
- **Cross-sectional strategies (`v1.momentum` etc.) are out of scope.**
  Those are hardcoded for top-10 crypto hourly and remain so for
  v0.1.0. Yahoo's free tier returns DAILY bars for any range > 60
  days (Yahoo's free intraday window is 60 days for ≥5m bars and 7
  days for 1m bars — see § Architecture findings § F2 below), so a
  cross-sectional daily-cadence rewrite is a separate v0.2.0 feature
  if/when the operator wants it.

## Scope (v0.1.0 — analyst-proposed; architect ratifies at M-T1)

- **Q1-resolution dependent**: a new `crates/data/src/yahoo.rs`
  (analyst-recommended) OR new top-level `crates/data_yahoo` (architect-
  decide at M-T1) module wraps the chosen Yahoo crate and exposes a
  `YahooBarSource` analogous to
  [`RealDataBarSource`](../../crates/backtest/src/realdata.rs).
- A parquet disk cache under `data/yahoo/<TICKER>/<YEAR>/<MONTH>.parquet`
  (analyst-recommended; mirrors Binance layout), with a
  `data/yahoo/REVISION.toml` aggregate-SHA manifest (mirrors
  [`data/binance/REVISION.toml`](../../data/binance/REVISION.toml)).
- A `cargo run --bin fetch_yahoo_klines -- <ticker> <range>` tool
  (mirrors the unsurfaced `fetch_binance_klines` from ADR-0032) that
  populates the cache on operator demand. **No automatic background
  fetch from the cockpit at v0.1.0** (K1 mitigation).
- A new dispatch path in the cockpit's Lab that routes
  `Source = Yahoo` to the new bar source. The 4 single-symbol
  scenario arms (`v0.sma`, `v0.5.macd`, `v0.5.rsi`, `v0.5.bbands`)
  gain a Yahoo dispatch arm under the existing
  `engine::run_scenario` entry — Q1=(a) — OR Yahoo dispatch is a
  pre-engine bar-source swap, Q1=(b). Architect decides at M-T1.
- Lab UI gains a `Source: [Synthetic | Yahoo]` toggle in the top
  bar (R-UI-1 below). Pair picker re-populates with Yahoo tickers
  when `Source = Yahoo`. Date-range picker constrains to ranges
  whose required bar cadence is available on Yahoo (free tier
  daily-only for ranges > 60 days).
- Asset-universe scoping per **Q2** below. Analyst-recommended:
  10-ticker crypto-mirror (`BTC-USD`, `ETH-USD`, `BNB-USD`,
  `SOL-USD`, `XRP-USD`, `ADA-USD`, `DOGE-USD`, `AVAX-USD`,
  `DOT-USD`, `LINK-USD`) for clean A/B vs the existing Binance
  cohort. Multi-asset expansion (equities + FX + commodities)
  deferred to v0.2.0 (R-V0.2-1 below).
- A new ADR (analyst-proposed sketch: **ADR-0040 — Yahoo realdata
  path and revision pin**) following the
  [ADR-0032](../architecture/adr/0032-backtest-realdata-path-and-revision-pin.md)
  template. Architect authors at M-T1.

## Out of scope (v0.1.0)

- **Live Yahoo streaming.** Yahoo's WebSocket is a separate
  endpoint, undocumented, brittle. v0.1.0 is historical bars only,
  consumed by the existing replay-driven `engine::run_scenario`.
- **Cross-sectional strategies on Yahoo daily bars.** The 4
  cross-sectional scenarios (`v1.momentum`, `v1.5a.pairs`,
  `v2.5.tcn`, `v2.5.tcn.weights`) keep their Binance / synthetic
  bar sources. Daily-cadence cross-sectional is a v0.2.0 follow-on
  (R-V0.2-2).
- **Equities + FX + commodities universe.** Crypto-mirror only at
  v0.1.0 per Q2 analyst-recommended default. Multi-asset expansion
  is a one-week follow-up once the wiring is proven.
- **Auto-refresh of the disk cache.** Operator triggers fetches
  explicitly via the CLI; the cockpit reads the cache read-only.
  v0.1.0 ships no scheduler, no daemon, no nightly cron. Auto-
  refresh is v0.2.0+ if operator wants.
- **Removing the Binance dispatch from `crates/backtest`.** The CLI
  `--features realdata` path stays in tree to preserve the 34 anchored
  reports' reproducibility. The Lab side simply stops calling into it.
- **Persistence of Yahoo cache across `cargo clean`.** The cache
  lives in `data/yahoo/` which is `.gitignore`d (architect to
  confirm at M-T1 — Binance precedent has `data/binance/` tracked
  with its REVISION.toml; analyst preference is to track Yahoo
  REVISION.toml + sample fixtures but `.gitignore` the bulk
  parquets — see K3 mitigation).
- **Auth/secrets**. Yahoo's free tier needs no key. If we later
  promote to a paid tier (Yahoo Finance Premium) for higher rate
  limits or intraday history beyond 60 days, ADR-0040 amends.

## Architecture findings

### F1 — Two viable Rust crates; both usable; analyst recommends `yahoo_finance_api`

Survey as of 2026-05-24:

| Crate              | Latest  | Last release  | License  | Async runtime | Notes                                                                                                          |
| ------------------ | ------- | ------------- | -------- | ------------- | -------------------------------------------------------------------------------------------------------------- |
| `yahoo_finance_api` | 4.1.x   | 2025-09       | MIT/Apache-2.0 | tokio (reqwest 0.10+) | Most popular; ~400 GitHub stars; `Quote` + `get_quote_history(ticker, start, end)` API; provides aggregated dividend + split adjustments. |
| `yfinance-rs`       | 0.7.2   | 2025-10-31    | MIT      | tokio (reqwest) | Newer; broader feature set (options, fundamentals, real-time streaming); less production-tested. |
| `yahoo-finance`     | 1.x     | ~2021         | MIT      | async-std (older) | Stale; uses `async-std`; not a fit for the project's tokio-only stack. |
| Custom HTTP scrape  | n/a     | n/a           | n/a      | tokio (reqwest direct) | Maximum control; bypasses any crate-side bugs but takes on full maintenance burden of Yahoo's unofficial endpoint shape. |

**Analyst-recommended: `yahoo_finance_api`** for v0.1.0 — the
narrower API surface (just `get_quote_history`) maps cleanly onto
the `BarSource` trait we already need; the tokio compat is given;
the dual MIT/Apache-2.0 license matches the project's defaults; it
has the larger user-base so upstream-breakage risk on Yahoo's API
shape is mitigated by community pressure.

**Deferred-Q3 fallback**: if `yahoo_finance_api` breaks on a Yahoo
API change and isn't patched within 14 days, fall back to a custom
HTTP scrape (the endpoint shape is well-documented in
community-maintained reverse-engineering notes). The wrapper
module's `YahooBarSource` trait stays the same; the implementation
swaps.

### F2 — Yahoo free-tier cadence is the load-bearing constraint

Yahoo's free historical endpoint has different cadence limits by
interval (verified empirically 2026-05-24; community-known limits):

| Interval | Max range supported           |
| -------- | ----------------------------- |
| `1m`     | ~7 days lookback              |
| `2m`     | ~60 days lookback             |
| `5m`     | ~60 days lookback             |
| `15m`    | ~60 days lookback             |
| `30m`    | ~60 days lookback             |
| `60m`/`1h` | ~730 days lookback (~2 yr) |
| `1d`     | ~30+ years lookback           |
| `1wk`    | ~50+ years lookback           |
| `1mo`    | ~50+ years lookback           |

Concrete implication: the current Binance path uses **1h bars over
full calendar years (2023, 2024)** — i.e., ~8,760 bars/year. Yahoo
free-tier supports 1h for ~2 years lookback, so the **2023+2024 1h
mirror is feasible on Yahoo** (`?period1=2023-01-01&period2=now&interval=1h`
should return). For ranges beyond ~2 years (the operator's
prospective "give me 10-year SPY backtest" use case), bars degrade
to daily.

**Q4** below asks the operator to pick the v0.1.0 cadence: (a)
mirror Binance at 1h hourly (best A/B vs current anchors; lookback
~2 years); (b) daily across full Yahoo history (multi-decade reach;
unlocks the equities/FX expansion); (c) adaptive — 1h when the
range fits inside 2 years, daily otherwise (most flexible; biggest
UX surface). Analyst-recommended: **(c) adaptive** with a UI
indicator showing which cadence the Lab is running.

### F3 — Strategy parameter semantics ARE bar-cadence-dependent

The 4 single-symbol strategies in `crates/strategy/src/`:

| Strategy   | Defaults                       | Bar-cadence intent                                              |
| ---------- | ------------------------------ | --------------------------------------------------------------- |
| SMA cross  | `fast_len=20`, `slow_len=50` (per `composed/sma.rs`) | hourly: ~1d / ~2d; daily: ~1mo / ~2.5mo. Both classic.   |
| MACD       | 12 / 26 / 9                    | hourly: ~12h/26h/9h; daily: ~12d/26d/9d (the textbook setting). |
| RSI        | 14                             | hourly or daily — both standard.                                |
| BBands     | 20, ±2σ                        | hourly: ~1d; daily: ~4 weeks. Both standard.                    |

The defaults are **textbook for daily bars**. Running them on hourly
bars (the current Binance path) is a deliberate "compress the
timescale" — the same parameter set produces 24× more trades on
hourly than daily. Switching the Lab from hourly Binance to daily
Yahoo bars therefore **changes the semantics of every backtest**,
even on the same nominal asset (BTC).

**Mitigation**: this is by design at v0.1.0. The operator's "replace
Binance for Lab" decision is explicitly multi-asset; the Lab's
strategy roster is designed to be cadence-agnostic; daily on
equities IS the textbook use case. K3 below logs this as a
quantification-risk row to revisit at M-FINAL.

**Q5** below asks whether per-cadence parameter overrides (e.g.
"SMA cross 12h/24h" for hourly vs "SMA cross 20/50" for daily)
should be a v0.1.0 feature or deferred. Analyst-recommended:
deferred — the strategies' JSON-schema defaults are already
exposed in the config layer; the operator can tweak per-run via
the existing param editor at the architect's discretion.

### F4 — Ticker convention: convert at the dispatch boundary

The existing UI universe stores `(Venue, Symbol)` where `Symbol` is
`BTCUSDT`. Yahoo uses `BTC-USD`. Two architecturally distinct
approaches:

- **(a) Convert at the dispatch boundary.** Keep
  `Venue::Yahoo + Symbol("BTCUSDT")` in the UI; the bar-source
  wrapper converts `BTCUSDT → BTC-USD` before calling the Yahoo
  crate. The conversion table lives in `crates/data/src/yahoo.rs`
  alongside the source. Smallest UI blast radius. Operator-facing
  ticker stays familiar to a crypto-trader.
- **(b) Re-base the UI on Yahoo tickers.** Add `Symbol::from_yahoo`
  / extend `crates/ui/src/lab/universe.rs` with a parallel
  `YAHOO_FIRST_UNIVERSE` slice using `BTC-USD` etc. UI displays
  Yahoo-native tickers. Cleaner for the multi-asset expansion
  (where `EURUSD=X` and `GC=F` have no Binance equivalent), but
  bigger short-term blast radius.

**Q6** below — analyst-recommended: **(a) convert at boundary**
for v0.1.0 (crypto-mirror only, the conversion is mechanical);
**(b) re-base UI** when multi-asset expansion lands in v0.2.0
(at which point hand-crafting `BTCUSDT ↔ BTC-USD ↔ EURUSD=X` in the
boundary becomes more painful than just owning Yahoo's namespace).

### F5 — Cache layout: parquet, mirrored on Binance

The Binance precedent is:

```
data/binance/
├── REVISION.toml             # aggregate SHA + per-file SHA index
├── BTCUSDT/2023/01.parquet   # one file per (symbol, year, month)
├── BTCUSDT/2023/02.parquet
└── …
```

Each parquet is ~700-800 KB / month (1h bars × ~30 days × ~10
symbols ≈ 7.2 KB/symbol/month — uncompressed; parquet compression
hits ~3-4 KB/symbol/month). The aggregate SHA pin is the regression
gate; ADR-0032 § 2 documents the protocol.

Analyst-recommended Yahoo layout (architect ratifies at M-T1):

```
data/yahoo/
├── REVISION.toml             # aggregate SHA + per-file SHA + Yahoo response checksum
├── BTC-USD/                  # Yahoo native ticker convention (Q6 = re-base at storage)
│   ├── 1d/                   # cadence as a subdir (Yahoo can return 1d, 1h, 1m on the same symbol)
│   │   ├── 2023/01.parquet
│   │   └── 2023/02.parquet
│   └── 1h/
│       └── 2024/01.parquet
└── EURUSD=X/                 # equities + FX expansion follows same shape
    └── 1d/
        └── 2024/01.parquet
```

**Differences vs Binance**:
- Cadence subdirectory (`1d/`, `1h/`, `1m/`) because Yahoo can serve
  the same symbol at multiple cadences and the operator may want
  both cached.
- Tickers stored in Yahoo-native form on disk (per F4 analyst-
  recommendation: convert at boundary but cache in Yahoo's
  namespace — that way the cache survives a future UI re-basing).
- `REVISION.toml` additionally carries the Yahoo response checksum
  (the raw JSON-as-CSV body from the unofficial endpoint, hashed
  before parquet conversion) so we can detect Yahoo-side data
  revisions (K2 mitigation) without re-fetching every cache.

**Q7** below — analyst-recommended **(a) parquet** per the above;
alternatives are (b) sqlite (per-query rows; better incremental-fetch
ergonomics but worse for replay-loop streaming) or (c) memory-only
(re-fetch every cockpit launch; cheap for daily bars but rejected
because the operator's "show me 30 years of SPY" use case is 30×
slower without on-disk persistence).

### F6 — Lab UI surface: Source toggle + range-aware constraints

Changes (architect + ui-designer at M-T1):

1. **New `Source: [Synthetic | Yahoo]` toggle** in the Lab top bar,
   between the strategy chips and the run button. F10's existing
   `RunState::Disabled` gate extends to "Yahoo selected but no cache
   for the (pair, range) tuple" → Run disabled with tooltip
   "Run `fetch_yahoo_klines BTC-USD --interval 1d --range 2024` first."
2. **Pair picker re-populated** with Yahoo tickers when toggle is
   Yahoo. v0.1.0 ships the 10-ticker crypto-mirror; v0.2.0 expands.
3. **Date-range picker constrained** by Yahoo's free-tier cadence
   limits. The existing presets (`Last7d`, `Last30d`, `Last90d`,
   `H1_2024`, `H2_2024`, `Full_2024`) all fit inside Yahoo's 60-day
   intraday window for `Last7d` and `Last30d`; longer ranges
   auto-degrade to daily cadence (or show a tooltip explaining
   the cadence).
4. **Optional "Fetch data" button** next to the toggle. Pressing
   triggers the CLI (`Command::Yahoo(YahooFetchCmd)`-style) and
   surfaces progress in the Lab status strip. **Out-of-scope at
   v0.1.0** if the operator picks Q8=(b) — i.e., operator fetches
   from terminal, no in-cockpit fetch UI.
5. **Status-strip "cache state" badge** showing data revision
   (`yahoo-rev:f1a2…`) so the operator can correlate a Lab run with
   the exact cache snapshot.

### F7 — Cross-feature impact: lab-end-to-end-v2 D-2c "Binance realdata wiring" is SUPERSEDED

[`spec/lab-end-to-end-v2/tasks.md`](../lab-end-to-end-v2/tasks.md)
Wave D-2 shipped 2026-05-24 the **single-symbol scenario dispatch
arms** (`v0.sma`, etc.) in `engine::run_scenario`. There was a
deferred D-2c task ("wire Binance parquet through Lab") in the
broader Lab roadmap.

**Operator decision 2026-05-24 supersedes D-2c**: Lab's real-data
source is Yahoo, not Binance. D-2c is therefore retired; this
brief is the replacement.

Concrete cross-feature edit (analyst owns; persisted to the v2
brief in this M0 pass):

```diff
- D-2c — wire Binance parquet through Lab (deferred to follow-on)
+ D-2c — **SUPERSEDED 2026-05-24** by `lab-yahoo-realdata v0.1.0`
+   per operator decision. Binance CLI path stays in tree for the 34
+   anchored reports; Lab dispatch points to Yahoo for real data.
+   See `spec/lab-yahoo-realdata/feature.md`.
```

The 34/34 anchor invariant is preserved because no anchored
scenario is touched — anchors live on the CLI path, which is
unchanged.

## Requirements

Numbered, testable, derived from F1-F7 + the operator's 2026-05-24
decision. Each preserves the 34/34 anchor invariant
([`spec/anchors.toml`](../anchors.toml)) and the 692-lib-test
baseline.

### R1 — Yahoo bar source crate wrapper

- **R1.1** New module `crates/data/src/yahoo.rs` (architect-decide at
  M-T1: separate `crates/data_yahoo` crate vs sub-module of
  `crates/data`) exposes `YahooBarSource` analogous to
  [`RealDataBarSource`](../../crates/backtest/src/realdata.rs):
  ```rust,ignore
  pub struct YahooBarSource { /* … */ }
  impl YahooBarSource {
      pub async fn fetch(&self, ticker: &str, interval: Interval,
                        start: OffsetDateTime, end: OffsetDateTime)
                        -> Result<Vec<Bar>, YahooError>;
      pub fn load_cached(&self, ticker: &str, interval: Interval,
                         span: TimeSpan) -> Result<Loaded, YahooError>;
  }
  ```
- **R1.2** Wrap `yahoo_finance_api` (F1 analyst-recommended). The
  crate dep is `--features yahoo` gated. The Lab UI compiles
  without `--features yahoo` and degrades the Source toggle to
  show "Yahoo (not built)" — keeping the cockpit build green for
  fixtures + synthetic-only operators.
- **R1.3** `YahooError` covers: network failure (`Reqwest`),
  Yahoo's rate-limit `429` (typed variant), JSON-parse failure,
  missing-cache (`CacheMiss { ticker, interval, span }`),
  cadence-violation (`CadenceUnsupported { interval, range_days }`
  — e.g., asking 1m for a range > 7 days).
- **Acceptance**: a `cargo test -p data --features yahoo
  yahoo::tests::fetch_btc_usd_1d_last_30_returns_bars` test (gated
  `#[ignore]`-or-`#[cfg(network)]` per project convention) that
  pulls `BTC-USD` daily for the last 30 days and asserts >= 25
  bars returned (Yahoo can skip weekends + occasional gaps).

### R2 — Disk cache: parquet, revision-pinned

- **R2.1** On-disk layout per F5:
  `data/yahoo/<TICKER>/<INTERVAL>/<YEAR>/<MONTH>.parquet`. Parquet
  schema mirrors `crates/data/src/lib.rs::Bar` (open_ts_ms,
  open, high, low, close, volume, symbol, venue). Symbol stored
  Yahoo-native (e.g., `BTC-USD`).
- **R2.2** `data/yahoo/REVISION.toml` mirrors the Binance manifest
  shape:
  ```toml
  [revision]
  sha256 = "<aggregate>"

  [revision.metadata]
  generated_at = "<RFC3339>"
  yahoo_base = "https://query1.finance.yahoo.com"
  fetch_tool = "fetch_yahoo_klines"
  fetch_version = "0.1.0"

  [files]
  "BTC-USD/1d/2024/01.parquet" = "<sha256>"
  ```
- **R2.3** `RevisionMismatch` error mirrors
  `RealDataError::RevisionMismatch` — on Lab run, the bar-source
  loader recomputes the per-file SHA and fails fast if it diverges.
  K2 mitigation (Yahoo-side data revision).
- **R2.4** `cargo run --features yahoo --bin fetch_yahoo_klines --
  <ticker> --interval <1d|1h|1m> --start <YYYY-MM-DD> --end <YYYY-MM-DD>`
  populates the cache + (re)writes `REVISION.toml`. Idempotent
  (re-running over an existing range no-ops).
- **R2.5** Coverage tolerance: by analogy to ADR-0032 § 2
  (`MissingData` at < 99.50%), Yahoo loader emits `MissingData`
  when actual bars < 95% of expected. **Loosened from Binance's
  99.50% because Yahoo skips weekends + market holidays on
  equities + FX; crypto is 24/7 but Yahoo's crypto daily series
  occasionally has 1-2 day gaps around exchange outages.** Q9
  operator-decide; analyst-recommended 95% threshold.
- **Acceptance**: integration test
  `crates/data/tests/yahoo_revision_verify.rs` (fixture cache
  under `tests/fixtures/yahoo/`) round-trips through
  `YahooBarSource::load_cached` and asserts SHA stability across
  2 calls.

### R3 — Lab dispatch wiring

- **R3.1** Lab UI gains a `Source: Synthetic | Yahoo` toggle (F6.1).
  State lives in `LabState.source: LabSource` (new enum); default
  `Synthetic`. Toggling does not auto-fire a run.
- **R3.2** When `source = Yahoo`, the Lab's `route_equity_overlay`
  / `runner::spawn_lab_run` path constructs a `YahooBarSource`
  (instead of `synthetic_bars_hourly`) keyed on `cfg.pair.1`
  (the selected ticker, converted via F4-(a) boundary mapping).
- **R3.3** `engine::run_scenario` either gains a `bars_source:
  ScenarioBarSource` field that's read by the 4 single-symbol arms
  (Q1=(a) inline arm extension) OR Lab swaps the bars *upstream* of
  dispatch and the engine remains source-agnostic (Q1=(b)
  pre-engine swap). Architect decides at M-T1; analyst-recommended
  (b) for minimum engine-arm blast radius.
- **R3.4** When `source = Yahoo` but the cache is empty for
  `(ticker, interval, range)`, the Run button enters
  `RunState::Disabled` with the existing F10 tooltip pattern;
  tooltip cites the exact `fetch_yahoo_klines` invocation needed.
- **R3.5** When `source = Synthetic`, behaviour is byte-identical
  to today (the synthetic path is the default; the toggle is a
  no-op).
- **Acceptance**: integration test
  `crates/ui/tests/lab_yahoo_dispatch.rs` boots fixtures cockpit
  + a fixture Yahoo cache + asserts `LabRunCompleted(Ok(_))` for
  `BTC-USD + v0.sma + Last30d` returns within 30 s; equity-series
  length matches Yahoo daily bar count (~30).

### R4 — Asset universe: crypto-mirror at v0.1.0

- **R4.1** New const `YAHOO_CRYPTO_UNIVERSE` in
  `crates/ui/src/lab/universe.rs` parallel to
  `XRP_FIRST_UNIVERSE`:
  ```rust,ignore
  pub const YAHOO_CRYPTO_UNIVERSE: &[(Venue, &str)] = &[
      (Venue::Yahoo, "BTC-USD"),
      (Venue::Yahoo, "ETH-USD"),
      (Venue::Yahoo, "BNB-USD"),
      (Venue::Yahoo, "SOL-USD"),
      (Venue::Yahoo, "XRP-USD"),
      (Venue::Yahoo, "ADA-USD"),
      (Venue::Yahoo, "DOGE-USD"),
      (Venue::Yahoo, "AVAX-USD"),
      (Venue::Yahoo, "DOT-USD"),
      (Venue::Yahoo, "LINK-USD"),
  ];
  ```
  Adds `Venue::Yahoo` variant to `trading_core::Venue`. R4.1's
  ticker count + order is locked by an analogue to the existing
  `xrp_first_order_pinned` test.
- **R4.2** When `source = Yahoo`, the Lab pair-chip row renders
  `YAHOO_CRYPTO_UNIVERSE`; when `source = Synthetic` (or default),
  renders `XRP_FIRST_UNIVERSE`. Toggle-driven, not click-driven.
- **R4.3** UI shows ticker text in Yahoo convention
  (`BTC-USD`, `ETH-USD`) when source = Yahoo. F4-(a) boundary
  mapping converts internally; the operator sees Yahoo-native.
- **Acceptance**: insta snapshots
  `lab__source_yahoo_pair_chip_row` and
  `lab__source_synthetic_pair_chip_row` differ exactly in the 10
  ticker labels; UI unit test
  `yahoo_crypto_universe_order_pinned` mirrors the XRP-first test
  shape.

### R5 — Adaptive bar cadence (Q4 analyst-recommended)

- **R5.1** `LabState` gains
  `cadence: Cadence` (enum `Daily | Hourly | Minute`) — derived
  from the selected range:
  - `range >= 60 days` → `Daily`.
  - `range < 60 days` AND `range >= 7 days` → `Hourly` (default;
    Yahoo supports ≥5m for 60 days, 1h within 730 days).
  - `range < 7 days` → `Minute` (1m bars, Yahoo's tightest free
    tier).
- **R5.2** Cadence shown as a badge in the Lab top bar
  (`"BTC-USD · 1d · Last90d"` style).
- **R5.3** Strategy parameters do NOT auto-rescale — the operator
  sees the same param sheet defaults regardless of cadence. K3
  (semantic-shift risk) logged; operator-trains-the-feel is the
  v0.1.0 stance per F3.
- **Acceptance**: unit tests for `Cadence::derive(range)` covering
  the 3 branches + edge cases (exactly 60 days, exactly 7 days).

### R6 — Anchor + revision-pin story

- **R6.1** The 34 locked Binance-based anchors in
  [`spec/anchors.toml`](../anchors.toml) are NOT touched. Yahoo
  reports emit under NEW scenario IDs at a future M-FINAL
  (architect-decide naming at M-T1; analyst suggests
  `yahoo-btc-usd-2024-1d-sma-cross`, `…-macd-trend`, etc., to
  parallel the Binance shape).
- **R6.2** Yahoo cache revision pin (`data/yahoo/REVISION.toml`)
  is verified at the head of every Yahoo-source Lab run; mismatch
  fails fast with the actionable error message ("rev mismatch:
  cache says X, recompute says Y — re-fetch with `…`").
- **R6.3** First Yahoo-based anchor SHAs lock at M-FINAL after
  the operator approves a sample report. Until then, Yahoo runs
  are non-anchored — reproducibility comes from the cache
  revision pin alone.
- **Acceptance**: `scripts/verify_anchors.sh` continues to
  return `ANCHORS PASS  (34 / 34)`. No new anchors emit at
  v0.1.0; that's a v0.1.1 follow-up gated on an operator-
  approved baseline.

### R7 — ADR-0040: Yahoo realdata path and revision pin

- **R7.1** New ADR at
  `spec/architecture/adr/0040-yahoo-realdata-path-and-revision-pin.md`
  following the
  [ADR-0032](../architecture/adr/0032-backtest-realdata-path-and-revision-pin.md)
  template. Architect authors at M-T1 from the analyst's outline
  below (§ ADR-0040 outline).
- **R7.2** ADR-0040 explicitly notes ADR-0032's revision-pin
  protocol as the generalised precedent — the only delta is the
  Yahoo-side data-revision risk (K2 in this brief) which the
  per-fetch response checksum addresses.
- **R7.3** External dep declaration: `yahoo_finance_api 4.1.x`
  (or `yfinance-rs 0.7.x` if M-T1 picks differently). Per
  CLAUDE.md non-negotiable, all new external deps require
  ADR-level justification.

### R-UI-1 — Lab Source toggle + supporting widgets

- **R-UI-1.1** New `crates/ui/src/widgets/source_toggle.rs` —
  Lumen-token-styled binary toggle (Synthetic | Yahoo). Inputs:
  `current: LabSource`, `on_change: Box<dyn Fn(LabSource) ->
  Message>`. Outputs: `Element<'static, Message>`.
- **R-UI-1.2** Cache-state badge in the Lab status strip when
  `source = Yahoo`: `"yahoo-rev: f1a2c3 · last fetched 2026-05-24
  14:32 UTC"`. Reads `data/yahoo/REVISION.toml` lazily.
- **R-UI-1.3** Optional "Fetch data" button (Q8 operator-decide).
  Analyst-recommended Q8=(b) — no in-cockpit fetch button at
  v0.1.0; operator runs `fetch_yahoo_klines` from terminal. The
  cache-state badge points to the command if cache empty.
- **R-UI-1.4** Cadence badge (R5.2) co-located with the source
  toggle.
- **Acceptance**: gallery snapshots `source_toggle__synthetic`,
  `source_toggle__yahoo`, `cache_state_badge__present`,
  `cache_state_badge__missing`, `cadence_badge__daily`,
  `cadence_badge__hourly`. Phase F default-disabled byte-identity
  preserved.

### R-NR — Non-regression contract (v0.1.0)

- **R-NR.1 — Anchors stay byte-identical (34/34).** No Yahoo
  change touches anchored scenarios. `scripts/verify_anchors.sh`
  exit 0 is mandatory at M-FINAL.
- **R-NR.2 — 692 lib tests stay green.** New tests are additive.
  Network-touching tests are gated `#[ignore]` or
  `#[cfg(feature = "yahoo-online")]`.
- **R-NR.3 — Phase F default-disabled byte-identity.** No Phase F
  code touched.
- **R-NR.4 — `spec-lint` clean.** New feature folder passes all
  9 enforced categories.
- **R-NR.5 — `cockpit-smoke` PASS.** Source toggle + cache-state
  badge + cadence badge render without panic in fixtures cockpit.
- **R-NR.6 — Idle-CPU floor ≤ 13.1%.** Yahoo cache reads are
  lazy + on-demand; no background polling. Cockpit-performance
  budget preserved.
- **R-NR.7 — `--features yahoo` is additive.** Default build
  (no `--features yahoo`) compiles + tests pass; the Lab Source
  toggle shows "Yahoo (not built)" inert state. Architect
  ratifies the gating at M-T1.
- **R-NR.8 — Default Lab behaviour unchanged.** With no operator
  interaction, `source = Synthetic` is the default; the cockpit
  looks + behaves byte-identical to pre-v0.1.0.

## Operator-decide Q-rows

Each Q has an analyst-recommended default. Listed in dispatch
order — Q1, Q2, Q4, Q6 are load-bearing (architect M-T1 needs
them); Q3, Q5, Q7, Q8, Q9 can defer to a later wave.

**Operator resolutions 2026-05-24** (all 10 Q's resolved; standing Autoapprove applied to Q3 / Q5 / Q7 / Q8 / Q9 / Q10):
- **Q1 = (b)** source-agnostic engine + Lab-side bar swap *(operator; matches analyst rec)*.
- **Q2 = (a)** crypto-mirror only — 10 Yahoo crypto tickers *(operator; matches analyst rec)*.
- **Q3 = (a)** `yahoo_finance_api 4.1.x` *(Autoapprove)*.
- **Q4 = (c)** adaptive cadence (1m ≤7d / 1h 7-60d / 1d >60d) *(operator; matches analyst rec)*.
- **Q5 = (b)** per-cadence param overrides deferred to v0.1.1 *(Autoapprove)*.
- **Q6 = (a)** UI ticker stays Binance-style (`BTCUSDT`); convert at dispatch boundary *(operator; OVERRODE analyst rec of (b))*. Rationale (operator): preserves UI symbol familiarity; hides Yahoo's `-USD` convention; avoids two-namespace cognitive load even though it costs operator visibility of the actual Yahoo ticker.
- **Q7 = (a)** parquet cache *(Autoapprove)*.
- **Q8 = (b)** defer in-cockpit Fetch Data button; CLI-only at v0.1.0 *(Autoapprove)*.
- **Q9 = (b)** 95% coverage threshold for `MissingData` *(Autoapprove)*.
- **Q10 = (b)** `.gitignore` parquets, track only REVISION.toml + sample fixtures *(Autoapprove)*.

All 10 resolutions are load-bearing for architect M-T1.



- **Q1 — Engine dispatch shape.** (a) extend
  `engine::run_scenario` with Yahoo-specific arms (`yahoo.v0.sma`,
  etc.); (b) keep engine source-agnostic, swap bars upstream in
  the Lab runner. *Analyst-recommended: (b)*. Engine stays
  unmodified; minimum anchor risk.
- **Q2 — Asset universe scope at v0.1.0.** (a) **crypto-mirror
  only** (10 USDT pairs mapped to `<SYM>-USD`); (b) crypto +
  equities (add `AAPL, MSFT, NVDA, GOOGL, SPY, QQQ, TSLA`);
  (c) crypto + equities + FX (`EURUSD=X, GBPUSD=X`); (d) full
  multi-asset (add commodities `GC=F, CL=F`). *Analyst-
  recommended: (a) crypto-mirror only*. Cleanest A/B vs the
  existing Binance cohort; (b)-(d) are one-week follow-ups
  each, gated on (a) working.
- **Q3 — Yahoo crate pick.** (a) `yahoo_finance_api 4.1.x`;
  (b) `yfinance-rs 0.7.x`; (c) custom HTTP. *Analyst-
  recommended: (a)*. Largest user-base; narrowest API surface;
  tokio compat.
- **Q4 — Bar cadence in v0.1.0.** (a) hourly-only (mirror
  Binance; ~2 yr lookback); (b) daily-only (multi-decade reach);
  (c) **adaptive** (1h for ≤2 yr ranges, daily otherwise).
  *Analyst-recommended: (c) adaptive*. UI shows the cadence
  badge so operator knows.
- **Q5 — Per-cadence strategy parameter overrides.** (a) ship
  per-cadence defaults (SMA 12h/24h vs 20d/50d); (b) defer to
  v0.1.1; operator tweaks via existing param sheet.
  *Analyst-recommended: (b) defer*. The strategies' JSON-schema
  defaults already exist; per-cadence overrides are a UX-debate
  not a v0.1.0 wiring concern.
- **Q6 — Ticker convention in the UI.** (a) keep UI ticker as
  Binance-style (`BTCUSDT`) and convert at the dispatch
  boundary; (b) display Yahoo-native (`BTC-USD`) in the UI.
  *Analyst-recommended: (b) for source=Yahoo only* — UI shows
  what the source uses; when source=Synthetic, UI shows
  `BTCUSDT`. Avoids the "two namespaces" cognitive load.
- **Q7 — Cache backend.** (a) **parquet** (mirrors Binance);
  (b) sqlite (per-query rows); (c) memory-only (re-fetch every
  cockpit launch). *Analyst-recommended: (a) parquet*. Mirrors
  existing pattern; revision-pin protocol generalises.
- **Q8 — In-cockpit "Fetch data" button.** (a) ship at v0.1.0;
  (b) **defer** — operator fetches from terminal via
  `fetch_yahoo_klines` CLI. *Analyst-recommended: (b) defer*.
  Keeps the cockpit read-only against the cache; avoids
  threading async-fetch + progress + cancel into the UI on
  top of the Lab progress-bar work that's already in flight in
  lab-end-to-end-v2 Wave D-4.
- **Q9 — Coverage threshold for `MissingData`.** (a) 99.50%
  (mirrors ADR-0032); (b) **95%** (relaxed for Yahoo gaps);
  (c) 90%. *Analyst-recommended: (b) 95%*. Yahoo skips
  weekends + holidays on non-crypto; crypto occasionally has
  1-2 day gaps on Yahoo's side.
- **Q10 — `data/yahoo/` git-tracking.** (a) track parquets +
  REVISION.toml (mirrors Binance); (b) `.gitignore` parquets,
  track only REVISION.toml + sample fixtures. *Analyst-
  recommended: (b)*. Yahoo cache will balloon as the
  multi-asset universe expands; tracking it in git is unwise.
  REVISION.toml is enough for reproducibility.

## Risks (K-rows)

- **K1 — Yahoo API rate limits + occasional garbage responses.**
  Yahoo's free tier returns `429` after ~1000 requests/hour from
  a single IP; bursty requests can return truncated CSV.
  *Mitigation*: disk cache is the primary path; cockpit reads
  from cache, not from the live API. Fetches happen via the
  `fetch_yahoo_klines` CLI which has built-in exponential
  backoff + retry (architect ratifies the retry policy at M-T1).
- **K2 — Yahoo data revisions across re-fetches.** Yahoo
  occasionally rewrites history (corporate actions, ticker
  remaps, exchange-side corrections). A re-fetch of the same
  `(ticker, interval, range)` can return subtly different bars
  vs an older fetch. *Mitigation*: REVISION.toml pin per-file
  SHA + aggregate SHA; mismatch fails fast. Operator decides
  whether to re-anchor or roll back.
- **K3 — Strategy params semantically shift when cadence
  changes.** SMA 20/50 means "20 hours / 50 hours" on hourly
  bars; "20 days / 50 days" on daily bars. Same param, 24×
  different timescale. Operators who learned the strategy on
  hourly Binance will need to recalibrate intuition on daily
  Yahoo. *Mitigation*: cadence badge in the Lab UI (R5.2);
  the responsibility falls on the operator (analyst-explicit
  per F3).
- **K4 — `yahoo_finance_api` is unofficial and could break.**
  Yahoo doesn't publish a stable public API; the crate
  reverse-engineers the response shape. *Mitigation*: pin
  exact version (`yahoo_finance_api = "=4.1.x"` per
  ADR-0040); maintain a fallback "custom HTTP" implementation
  notes inside the wrapper module so a 14-day fix window is
  feasible (F1 deferred-Q3 fallback).
- **K5 — License + TOS.** Yahoo's TOS technically prohibits
  *redistribution* of historical data. *Mitigation*: cached
  parquets live in `data/yahoo/`, which per Q10=(b) is
  `.gitignore`d. Only REVISION.toml + sample fixtures go to
  git. We are not redistributing Yahoo data; we are
  redistributing checksums.
- **K6 — Coverage drift on equities.** When v0.2.0 expands to
  equities, the 99.50% Binance coverage tolerance is not
  achievable on Yahoo equity series (weekends + holidays +
  occasional gaps). *Mitigation*: Q9=(b) — relax to 95%; build
  the loader to be tolerant of expected weekend gaps via a
  market-calendar layer at v0.2.0 (out-of-scope here).
- **K7 — `Venue::Yahoo` variant breaks exhaustive matches.**
  Adding to `trading_core::Venue` is a downstream typecheck
  cascade. *Mitigation*: architect tracks the match-arm
  additions; clippy `-D warnings` catches missed arms.
- **K8 — UI test snapshots drift on F-toggle.** The new Source
  toggle widget changes panel layout; insta snapshots in
  `crates/ui/tests/panel_snapshots.rs` may need refresh.
  *Mitigation*: architect M-T1 plans the snapshot refresh as
  part of the Wave G UI delivery; ui-designer authors the
  Lumen-correct rendering.
- **K9 — Cache rev-mismatch on cockpit restart UX is
  alarming.** If the operator re-fetches the cache externally
  + the SHA changes, the next Lab run fails with
  `RevisionMismatch`. *Mitigation*: clear error message with
  the actionable command + a "Reload cache" affordance in the
  Lab status strip (R-UI-1.2).
- **K10 — Async-runtime mismatch.** `yahoo_finance_api` uses
  reqwest+tokio; the cockpit's iced loop is its own runtime.
  *Mitigation*: fetch happens off the iced loop (the
  `fetch_yahoo_klines` CLI is a separate process at v0.1.0
  per Q8=(b)); the cockpit only ever does sync parquet reads.

## Hypotheses (H-rows; falsifiable, measurable)

- **H1 — Yahoo daily BTC-USD 2023+2024 yields a `v0.sma`
  equity series that diverges from Binance hourly BTCUSDT
  on the same span by < 30% terminal value.** Falsifier: if the
  divergence exceeds 30%, log a finding + investigate (most
  likely cause: missing intraday signal; expected outcome:
  daily under-trades, terminal value differs but trajectory
  matches qualitatively). Measured at M-FINAL on a sample run.
- **H2 — `yahoo_finance_api 4.1.x` fetches `BTC-USD` daily
  for the last 365 days successfully on > 95% of invocations
  across a 7-day measurement window.** Falsifier: > 5% failure
  rate routes back to analyst for Q3 fallback consideration.
- **H3 — Disk cache hit-rate from a warm cockpit run is
  100%** (no network calls during a Lab run; all reads come
  from `data/yahoo/`). Falsifier: any network egress during a
  Lab run is a wiring bug.
- **H4 — Parquet revision-pin SHA is deterministic** across 2
  back-to-back `fetch_yahoo_klines` invocations against the
  same range, ASSUMING Yahoo's response is identical. Falsifier:
  same-day SHA drift implies our parquet writer is
  non-deterministic; investigate.
- **H5 — Default Lab UX (source = Synthetic) is byte-identical
  to pre-v0.1.0.** Insta panel-snapshot diff = 0 with
  `LabSource::Synthetic` default. Falsifier: any pixel diff is
  a v0.1.0 regression — fix before ship.
- **H6 — Switching `source = Synthetic → Yahoo` does not
  trigger a cargo rebuild** (no codegen on the source flip; it's
  a runtime state mutation). Falsifier: rebuild observed →
  feature-gate is mis-scoped, architect investigates.

## ADR-0040 outline (analyst → architect to author at M-T1)

```
Title:    ADR-0040 — Yahoo realdata path and revision pin
Status:   Proposed (analyst-drafted 2026-05-24; architect ratifies at M-T1)
Context:  Operator decision 2026-05-24 "replace Binance for Lab —
          multi-asset pivot". ADR-0032 (Binance) is the precedent;
          Yahoo generalises the pattern across crypto + equities +
          FX + commodities.
Decision: Adopt `yahoo_finance_api 4.1.x` (Q3=(a)) as the Yahoo
          client; parquet cache layout under `data/yahoo/<TICKER>/
          <INTERVAL>/<YEAR>/<MONTH>.parquet` (Q7=(a)); revision-
          pin protocol mirrors ADR-0032 § 2 with an additional
          per-fetch Yahoo response checksum to detect upstream
          revisions (K2). Engine remains source-agnostic; Lab
          runner swaps bars upstream of `engine::run_scenario`
          (Q1=(b)). UI uses Yahoo-native tickers when
          source=Yahoo (Q6=(b)).
Consequences:
  - New `--features yahoo` build flag; default-off; Lab
    degrades gracefully to "not built" when off.
  - `data/yahoo/` `.gitignore`d except for REVISION.toml +
    fixture samples (Q10=(b)).
  - `Venue::Yahoo` variant cascades through
    `trading_core::Venue` matches (K7).
  - The 34 Binance-based anchors are immutable; future Yahoo
    anchors lock under new scenario IDs after operator
    approval of a sample.
References:
  - ADR-0032 (Binance precedent)
  - `spec/lab-yahoo-realdata/feature.md`
  - `spec/lab-end-to-end-v2/feature.md` (the predecessor that
    extracted single-symbol dispatch arms)
  - `data/binance/REVISION.toml` (the file format we're
    generalising)
```

## Routes (planned waves)

Architect M-T1 finalises:

- **Wave A — Analyst (this brief).** ✅ M0 close 2026-05-24.
- **Wave B — Architect M-T1.** ADR-0040 author; module layout
  (`crates/data/src/yahoo.rs` vs new crate); engine-dispatch
  decision (Q1); cache-layout ratification.
- **Wave C — Developer (parallel split possible).**
  - **C-1 — `YahooBarSource` + cache.** `crates/data` work;
    no UI surface. Independent of UI work.
  - **C-2 — `fetch_yahoo_klines` CLI.** New binary under
    `crates/backtest/src/bin/` (or `crates/data/src/bin/`,
    architect-decide). Wraps Yahoo client; writes parquet +
    REVISION.toml.
  - **C-3 — Lab dispatch wiring.** `crates/ui/src/lab/`
    changes for `LabSource` enum + runner swap.
  - **C-4 — `Venue::Yahoo` cascade.** `crates/core` + clippy-
    guided match-arm fixes across the workspace.
- **Wave D — UI-designer (parallel with C-3).**
  - Source toggle widget, cache-state badge, cadence badge —
    Lumen-tokenised, snapshot-gated.
- **Wave E — Tester (M-FINAL).** rust-build + rust-validate +
  rust-test + cockpit-smoke + spec-lint + verify-anchors
  (34/34) + a new integration test for the round-trip cache
  → Lab → equity.
- **Wave F — Presenter.** Sprint-review deck with a live
  cockpit run on `BTC-USD + v0.sma + Last90d` (Yahoo source) +
  H1-H6 numerical verdict + operator-approval block.

## References

- [`spec/backtest-real-binance-data/feature.md`](../backtest-real-binance-data/feature.md)
  — the Binance precedent (shipped 2026-05-18).
- [`crates/backtest/src/realdata.rs`](../../crates/backtest/src/realdata.rs)
  — `RealDataBarSource` is the API shape we're generalising.
- [`data/binance/REVISION.toml`](../../data/binance/REVISION.toml)
  — the manifest format we're mirroring.
- [`spec/architecture/adr/0032-backtest-realdata-path-and-revision-pin.md`](../architecture/adr/0032-backtest-realdata-path-and-revision-pin.md)
  — the protocol ADR.
- [`spec/lab-end-to-end-v2/feature.md`](../lab-end-to-end-v2/feature.md)
  — the predecessor that landed single-symbol scenario dispatch
  in `engine::run_scenario`; D-2c "Binance Lab wiring" is
  SUPERSEDED by this brief.
- [`crates/ui/src/lab/universe.rs`](../../crates/ui/src/lab/universe.rs)
  — the existing `XRP_FIRST_UNIVERSE` we're paralleling with
  `YAHOO_CRYPTO_UNIVERSE`.
- [`spec/anchors.toml`](../anchors.toml) — the 34 locked anchor
  SHAs that v0.1.0 preserves byte-identical.
- [`https://docs.rs/yahoo_finance_api/latest/yahoo_finance_api/`](https://docs.rs/yahoo_finance_api/latest/yahoo_finance_api/)
  — Q3=(a) candidate crate docs.
- [`https://docs.rs/yfinance-rs`](https://docs.rs/yfinance-rs)
  — Q3=(b) candidate crate docs.

## Implementation

### Wave C-4 — `Venue::Yahoo` variant cascade (developer, 2026-05-24)

**Files touched:**

- `crates/core/src/venue.rs` — added `Yahoo` variant with doc-comment explaining
  data-only semantics; extended `Display`, `FromStr`, and unit tests.
- `crates/agent/tests/coinbase_outage_isolation.rs` — cascade fix: added
  `Venue::Yahoo => unreachable!("Yahoo is data-only; no live tick feed routes
  ticks with Venue::Yahoo")` arm to the non-exhaustive `match tick.venue`.
- `crates/ui/src/lab/persistence.rs` — string decode: added `"Yahoo" =>
  Venue::Yahoo` arm for future deserialization of persisted Lab state.
- `crates/backtest/src/scenarios/sma_composed_run.rs` — fixed pre-existing
  `doc_markdown` clippy warning (ChaCha20Rng backtick).

**Cascade map:** clippy `-D warnings` surfaced exactly 1 non-exhaustive match
site (agent/tests/coinbase_outage_isolation.rs:308). All other Venue::X usages
are constructors, not match arms.

**Anchor impact:** `Venue::Yahoo` is additive. All 34 anchored body-SHAs remain
byte-identical (`bash scripts/verify_anchors.sh` → `ANCHORS PASS (34 / 34)`).
The existing Binance-path audit writes use `venue.to_string()` → "binance" —
unchanged.

**Cargo gates:**
- `cargo fmt --check` → PASS
- `cargo clippy --workspace --features candle,realdata,live -- -D warnings` → PASS (0 warnings)
- `cargo test --workspace --lib --features candle` → 1,078 passed; 0 failed
- `bash scripts/verify_anchors.sh` → ANCHORS PASS (34 / 34)

### Wave C-2 — `fetch_yahoo_klines` CLI binary (developer, 2026-05-24)

**Scope:** workspace `Cargo.toml` dep add + `crates/data/Cargo.toml` feature gates +
`crates/data/src/bin/fetch_yahoo_klines.rs` (NEW).

**Files touched:**

- `Cargo.toml` — added `yahoo_finance_api = { version = "=4.1.0",
  default-features = false }` to `[workspace.dependencies]`.
  ADR-0040 D2 (6-item library-compat checklist) is the CLAUDE.md
  non-negotiable gate for this external dep.
- `crates/data/Cargo.toml` — added `yahoo_finance_api = { workspace = true,
  optional = true }` + features `yahoo = ["dep:yahoo_finance_api"]` and
  `yahoo-online = ["yahoo"]` (default-off). Also added
  `[[bin]] name = "fetch_yahoo_klines" required-features = ["yahoo-online"]`.
- `crates/data/src/yahoo.rs` — added `fetch_and_cache` async method under
  `#[cfg(feature = "yahoo-online")]` with supporting helpers:
  `classify_yfa_error`, `sha256_of_quotes`, `quotes_to_bars`,
  `write_bars_by_month`, `upsert_yahoo_response_checksum`,
  `regenerate_revision_manifest`, `write_revision_manifest`.
- `crates/data/src/bin/fetch_yahoo_klines.rs` (NEW, ~340 LOC):
  - clap arg parsing: `--tickers`, `--interval`, `--start`, `--end`,
    `--out`, `--dry-run`, `--emit-revision-manifest`.
  - tokio async main.
  - `fetch_with_backoff`: exponential backoff 1s→60s cap, max 5 retries
    on `YahooError::RateLimited` (K1 mitigation, ADR-0040 § T-AR7).
  - `run_dry`: prints URL + expected bar count, zero I/O (T-C2.4).
  - 9 unit tests: `parse_interval_*`, `parse_date_range_*`,
    `parse_date_to_midnight_ms_*`, `format_date_*`,
    `dry_run_executes_without_panic`.

**Anchor impact:** zero. `cargo run --bin fetch_yahoo_klines` never touches
anchored CLI paths. `ANCHORS PASS (34 / 34)` confirmed.

**Cargo gates:**
- `cargo fmt --check` → PASS
- `cargo clippy -p data --features yahoo-online -- -D warnings` → PASS (0 warnings)
- `cargo build -p data --features yahoo-online --bin fetch_yahoo_klines` → PASS
- `cargo test -p data --features yahoo-online --bin fetch_yahoo_klines` →
  `test result: ok. 9 passed; 0 failed`
- `cargo test -p data --features yahoo-online --lib` →
  `test result: ok. 56 passed; 0 failed; 1 ignored`
- `bash scripts/verify_anchors.sh` → `ANCHORS PASS (34 / 34)`

## Changelog

- 2026-05-24 (analyst): initial v0.1.0 brief. Crate survey
  (F1) → `yahoo_finance_api` recommended. Cadence constraint
  (F2) → adaptive recommended (Q4=(c)). Strategy semantic-
  shift logged (F3, K3). Ticker convention (F4) → boundary
  conversion at v0.1.0, UI re-base at v0.2.0 (Q6=(b)). Cache
  layout (F5) → parquet mirror of Binance (Q7=(a)). UI surface
  (F6) outlined for ui-designer. Cross-feature impact (F7) →
  lab-end-to-end-v2 D-2c marked SUPERSEDED via a separate spec
  edit. 10 Q-rows, 10 K-rows, 6 H-rows, 8 R-rows + 1 R-UI +
  1 R-NR contract. ADR-0040 outline drafted. HANDOFF →
  orchestrator → operator-decide → architect for M-T1.
