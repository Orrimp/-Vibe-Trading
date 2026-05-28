---
adr: 0040
title: Yahoo realdata path + revision pin (Lab dispatch source)
status: accepted
date: 2026-05-24
supersedes: none
superseded-by: none
---

# ADR-0040: Yahoo realdata path + revision pin (Lab dispatch source)

## Context

Operator decision 2026-05-24 (verbatim): **"Replace Binance for Lab —
multi-asset pivot."** Promoted Idea → Active in
[`spec/backlog.md`](../../backlog.md) the same day. The brief at
[`spec/lab-yahoo-realdata/feature.md`](../../lab-yahoo-realdata/feature.md)
resolves 10 operator-decide questions (Q1-Q10, all closed 2026-05-24)
and depends on this ADR locking the cross-cutting decisions.

[ADR-0032](0032-backtest-realdata-path-and-revision-pin.md) (Binance
precedent, shipped 2026-05-18) locked four orthogonal decisions for
the **CLI** `backtest --features realdata` path: (1) parquet-read
module placement, (2) `REVISION.toml` schema + aggregate-SHA
algorithm, (3) `ScenarioDataSource` axis on `Scenario`, (4)
`data_revision_sha` in frontmatter + body. ADR-0040 generalises the
revision-pin protocol to a second data source (Yahoo Finance) on the
**Lab dispatch** path, with these structural deltas:

- **Engine stays source-agnostic.** The Q1 = (b) operator pick keeps
  `engine::run_scenario` unchanged — bars are swapped upstream of
  dispatch via the existing `bars_override: Option<Vec<Bar>>` hook in
  `crates/backtest/src/scenarios/sma_composed_run.rs:245` (extracted
  in lab-end-to-end-v2 Wave D-2). No new engine arm; minimum anchor
  blast radius.
- **Cadence sub-directory on disk.** Yahoo serves multiple cadences
  for the same ticker; the cache layout is
  `data/yahoo/<TICKER>/<INTERVAL>/<YEAR>/<MONTH>.parquet`. The
  Binance layout has no cadence subdir (it is 1h only).
- **Relaxed missing-data threshold.** Yahoo's free crypto series has
  occasional 1-2 day gaps around exchange outages. Q9 = (b) drops the
  threshold from 99.50% (ADR-0032 § R3) to 95%.
- **UI ticker namespace stays Binance-style.** Q6 = (a) operator
  override of analyst rec (b). The conversion
  `BTCUSDT → BTC-USD` happens at the dispatch boundary, not at the
  UI symbol type. Rationale (operator verbatim): preserves UI symbol
  familiarity, hides Yahoo's `-USD` convention, avoids two-namespace
  cognitive load.
- **CLI-only fetch at v0.1.0.** Q8 = (b): no in-cockpit "Fetch data"
  button; operator runs `fetch_yahoo_klines` from terminal.
- **Parquets `.gitignore`d.** Q10 = (b): track only
  `data/yahoo/REVISION.toml` + sample fixtures. Yahoo TOS
  technically prohibits redistribution of bulk data (K5 mitigation).

This ADR exists because **CLAUDE.md non-negotiable** — every new
external crate dep requires ADR-level justification — gates the
`yahoo_finance_api 4.1.x` (Q3 = (a)) addition. The library-compat
checklist below documents the gate.

## Decision

### D1 — Module placement: `crates/data/src/yahoo.rs` sub-module

`YahooBarSource` lives in `crates/data/src/yahoo.rs`, gated behind a
new cargo feature `yahoo` on `crates/data/Cargo.toml`. Public surface
is `pub use crate::yahoo::{YahooBarSource, LoadedBars, YahooError}`
at the crate root, mirroring `data::ReplayFeed`.

**Rationale.** A sub-module of `crates/data` (not a new
`crates/data_yahoo` crate) because:

1. The Yahoo loader's pure-Rust read path (parquet → `Vec<Bar>`)
   is **identical in shape** to `data::ReplayFeed::read_parquet_bars`.
   Splitting into a separate crate forces duplication or a downstream
   `crates/data_yahoo → crates/data` dep that adds zero functional
   isolation.
2. The `data` crate already owns the `revision` module (used by
   `crates/backtest/src/realdata.rs`); generalising it for two sources
   is the natural next refactor. Co-locating Yahoo here keeps the
   refactor surface contiguous.
3. The `yahoo` feature is **default-off**, so cockpit builds without
   `--features yahoo` are byte-identical to today's default build
   (R-NR.7 / R-NR.8). The `--features live --features yahoo` build
   activates the Yahoo dispatch path.

`fetch_yahoo_klines` CLI binary lands at
`crates/data/src/bin/fetch_yahoo_klines.rs` (NOT under
`crates/backtest/src/bin/`) because it is a data-acquisition tool,
not a backtest scenario tool — co-located with `fetch_binance_klines`
in the same parent crate keeps fetch utilities discoverable.

### D2 — `yahoo_finance_api 4.1.x` external dep (CLAUDE.md gate)

External crate: `yahoo_finance_api` from crates.io, pinned at
`=4.1.x` (architect ratifies the exact patch version at developer
Wave C-2 cargo add time; recommended initial pin `=4.1.0`).

CLAUDE.md non-negotiable checklist:

- [x] **Single-binary friendly.** Pure-Rust client wrapping
  `reqwest` + `tokio`; no system C deps, no Postgres pin, no extra
  daemon.
- [x] **No system C deps without `bundled` option.** Uses
  `reqwest = { default-features = false, features = ["rustls-tls",
  "json"] }` per project convention. No native-tls / libssh2 leak.
- [x] **Edition 2024 compatible.** Verified at architect M-T1
  via `cargo check` against latest stable; clean.
- [x] **No stdlib crate-name shadowing.** Package name
  `yahoo_finance_api` does not collide with `core`, `std`, `alloc`,
  `test`, `proc_macro`.
- [x] **Maintained.** Last release 2025-09; > 400 GitHub stars;
  > 2 years of activity. Within the 18-month maintenance window.
- [x] **License compatible.** Dual MIT / Apache-2.0. Matches project
  license policy (workspace is MIT-licensed).

**Alternatives considered:**

- `yfinance-rs 0.7.x` — broader API surface (options + fundamentals
  + real-time WebSocket), but newer (~2 years old), smaller
  user-base, larger blast radius for unofficial-API drift (K4).
  *Deferred: revisit at v0.2.0 if Yahoo's WS endpoint becomes
  in-scope.*
- `yahoo-finance 1.x` — uses `async-std`. **Rejected** — project is
  tokio-only; `async-std` is a non-starter.
- Custom HTTP scrape via `reqwest` — maximum control, but moves the
  full Yahoo-endpoint reverse-engineering burden in-tree. **Deferred
  to fallback** (F1 § Deferred-Q3): if `yahoo_finance_api` breaks on
  a Yahoo API change and isn't patched within 14 days, the wrapper's
  internal HTTP path replaces the dep — the `YahooBarSource` public
  API stays unchanged.

### D3 — Revision-pin protocol (mirrors ADR-0032 § D2)

**Location.** `data/yahoo/REVISION.toml`. One file per data root.
The `data/yahoo/` directory is `.gitignore`d (Q10 = (b)); only
`REVISION.toml` and sample fixtures under `tests/fixtures/yahoo/`
ship to git. Operator regenerates locally via
`fetch_yahoo_klines --emit-revision-manifest`.

**Schema (sorted-keys TOML, ADR-0032 § D2 shape + cadence subdir +
per-fetch Yahoo response checksum):**

```toml
# data/yahoo/REVISION.toml — generated by fetch_yahoo_klines.

[revision]
# Aggregate SHA over the (relpath → file-sha256) sorted map below.
# Algorithm identical to ADR-0032 § D2:
#   sha256("\n".join(f"{relpath}\t{sha256}" for ... in sorted_entries))
sha256 = "<64 hex chars>"

[revision.metadata]
# Advisory only — NOT part of the aggregate SHA.
generated_at  = "2026-05-24T12:00:00Z"
yahoo_base    = "https://query1.finance.yahoo.com"
fetch_tool    = "fetch_yahoo_klines"
fetch_version = "0.1.0"

# Sorted (lexicographic) per-fetch Yahoo response checksums.
# Key shape: "{TICKER}/{INTERVAL}/{YEAR}-{MONTH}".
# Detects upstream data revisions WITHOUT recomputing the parquet
# SHA — operator can correlate a parquet-SHA flip with the
# upstream JSON-body change that caused it. K2 mitigation.
[revision.yahoo_response]
"BTC-USD/1d/2024-01" = "<64 hex of raw JSON-as-CSV body>"

# Sorted per-file parquet SHAs (the aggregate-SHA input).
[files]
"BTC-USD/1d/2024/01.parquet" = "<64 hex chars>"
"BTC-USD/1d/2024/02.parquet" = "<64 hex chars>"
"ETH-USD/1d/2024/01.parquet" = "<64 hex chars>"
# ...
```

**Aggregate-SHA algorithm.** Byte-identical to ADR-0032 § D2.
Re-using the existing `data::revision::compute_aggregate_sha`
helper (in `crates/data/src/revision.rs`); no second implementation.
The Yahoo path adds the `[revision.yahoo_response]` table for
upstream-revision forensics — those entries are NOT part of the
aggregate SHA (same as ADR-0032's `[revision.metadata]` carve-out).

**Verifier.** `YahooBarSource::load_cached` (D5 below) runs the
same 3-step verification as ADR-0032 § D2 verbatim — manifest
exists → per-file SHA match → recomputed aggregate matches claimed.

### D4 — Engine remains source-agnostic; Lab swaps bars upstream (Q1 = (b))

`engine::run_scenario` (in `crates/backtest/src/engine.rs:467`) is
**unchanged**. The 4 single-symbol arms (`v0.sma`, `v0.5.macd`,
`v0.5.rsi`, `v0.5.bbands`) all dispatch to
`crates/backtest/src/scenarios/sma_composed_run.rs::run` which
already accepts `bars_override: Option<Vec<Bar>>` (extracted in
lab-end-to-end-v2 Wave D-2).

Lab dispatch becomes: the Lab runner constructs the `Vec<Bar>` from
either synthetic generation or `YahooBarSource::load_cached` before
calling `run_scenario`. The new wiring lives **outside** the engine:

- `ScenarioConfig` gains a new `data_source: ScenarioDataSource` enum
  (default `Synthetic`; new variant `YahooCache`) at
  `crates/backtest/src/engine.rs:151`.
- `run_scenario`'s 4 single-symbol arms read `cfg.data_source` and, on
  `YahooCache`, pre-populate `bars_override` from
  `YahooBarSource::load_cached` (passed in via a NEW required field
  `cfg.bars_override: Option<Vec<Bar>>`).
- Cross-sectional arms (`v1.momentum`, `v1.5a.pairs`, `v2.5.tcn*`)
  **reject** `data_source = YahooCache` with a typed error — those
  scenarios are out-of-scope per § Out of scope.
- Anchor-bearing `-realdata` Binance scenarios (the 18 anchors in
  `v2.6.0-realdata` + `v3.0.0-llm-forecaster` namespaces) are
  **unreached** by the Lab path; they stay on the CLI
  `--features realdata` route. **No anchor touched.**

**Anchor neutrality proof:**

- Default `data_source = ScenarioDataSource::Synthetic` is byte-
  identical to today's behaviour (no `bars_override`, RNG-seeded
  synthetic path).
- The 34 anchored body-SHAs all originate from CLI invocations on
  `cargo run --features realdata --bin backtest -- <scenario>`; Lab
  does not call those scenarios. Adding `data_source` to
  `ScenarioConfig` with a `#[serde(default)]` `Synthetic` keeps every
  existing call site (including all anchor-generating CLI paths)
  byte-identical.
- R-NR.1 gate: `scripts/verify_anchors.sh → ANCHORS PASS (34 / 34)`
  is the regression test.

### D5 — `YahooBarSource` API surface

```rust
// crates/data/src/yahoo.rs (feature-gated: yahoo)

pub struct YahooBarSource {
    cache_root: PathBuf,      // data/yahoo/
    revision_sha: OnceCell<String>,
}

pub struct LoadedBars {
    pub bars: Vec<trading_core::Bar>,  // k-way merged (open_ts ASC, sym ASC)
    pub revision_sha: String,
    pub loaded_count: usize,
    pub expected_count: usize,
    pub interval: Interval,            // 1m | 1h | 1d
}

#[derive(thiserror::Error, Debug)]
pub enum YahooError {
    #[error("data/yahoo/REVISION.toml not found at {path}")]
    RevisionMissing { path: String },

    #[error("REVISION.toml parse error: {0}")]
    RevisionParse(String),

    #[error("data revision mismatch for {file}: \
             manifest={manifest_sha}, on-disk={actual_sha}")]
    RevisionMismatch { file: String, manifest_sha: String, actual_sha: String },

    #[error("cache miss for ({ticker}, {interval:?}, [{start_label}..{end_label})); \
             run `fetch_yahoo_klines --ticker {ticker} --interval {interval_str} \
             --start {start_iso} --end {end_iso}`")]
    CacheMiss {
        ticker: String,
        interval: Interval,
        interval_str: &'static str,
        start_ms: i64,
        end_ms: i64,
        start_label: String,
        end_label: String,
        start_iso: String,
        end_iso: String,
    },

    #[error("cadence {interval:?} not supported for range {range_days} days \
             (Yahoo's free tier: 1m ≤ 7d, 1h ≤ 730d)")]
    CadenceUnsupported { interval: Interval, range_days: i64 },

    #[error(
        "ticker {ticker} ({interval:?}, [{start_label}..{end_label})) expected \
         {expected} bars; got {actual} ({pct:.2}% present), below tolerance 95.00%"
    )]
    MissingData {
        ticker: String,
        interval: Interval,
        expected: usize,
        actual: usize,
        pct: f64,
        start_label: String,
        end_label: String,
    },

    #[error("yahoo http error: {0}")]
    Http(String),

    #[error("yahoo rate-limited (429); retry after {retry_after_secs}s")]
    RateLimited { retry_after_secs: u64 },

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

impl YahooBarSource {
    pub fn new(cache_root: PathBuf) -> Self;

    /// Read-only cache load — used by Lab dispatch (no network).
    /// Verifies revision pin first; computes coverage; emits
    /// MissingData if < 95.00%; returns merged bars sorted
    /// (open_ts ASC, symbol ASC).
    pub fn load_cached(
        &self,
        ticker: &str,        // Yahoo-native, e.g. "BTC-USD"
        interval: Interval,  // 1m | 1h | 1d
        start_ms: i64,
        end_ms: i64,
    ) -> Result<LoadedBars, YahooError>;

    /// Online fetch — only called by `fetch_yahoo_klines` CLI.
    /// Behind `#[cfg(feature = "yahoo-online")]`.
    #[cfg(feature = "yahoo-online")]
    pub async fn fetch_and_cache(
        &self,
        ticker: &str,
        interval: Interval,
        start_ms: i64,
        end_ms: i64,
    ) -> Result<LoadedBars, YahooError>;
}
```

The split between read-only `load_cached` (used by the cockpit) and
async `fetch_and_cache` (used by the CLI) is feature-gated so the
cockpit build does NOT pull `tokio::reqwest::Client` into the iced
loop. K10 mitigation (async-runtime mismatch).

### D6 — Adaptive cadence (Q4 = (c))

```rust
pub enum Interval { Minutes1, Hours1, Days1 }

impl Interval {
    /// Q4 = (c) — adaptive cadence derivation.
    ///
    /// Branches:
    /// - range < 7 days       → 1m (Yahoo free-tier minute limit)
    /// - range in [7, 60] d   → 1h (Yahoo free-tier hourly limit ~730d)
    /// - range > 60 days      → 1d (multi-decade reach on Yahoo)
    ///
    /// Boundary at exactly 7 days → 1h (inclusive on the upper side).
    /// Boundary at exactly 60 days → 1h (inclusive on the upper side).
    pub fn derive_from_range(start_ms: i64, end_ms: i64) -> Self {
        const MS_PER_DAY: i64 = 86_400_000;
        let range_ms = (end_ms - start_ms).max(0);
        let range_days = range_ms / MS_PER_DAY;
        match range_days {
            d if d < 7 => Interval::Minutes1,
            d if d <= 60 => Interval::Hours1,
            _ => Interval::Days1,
        }
    }
}
```

UI shows a cadence badge (`1m` / `1h` / `1d`) co-located with the
date-range picker (R-UI-1.4). Strategy parameters do NOT auto-rescale
(K3 — operator-trains-the-feel is the v0.1.0 stance per F3); Q5 per-
cadence overrides deferred to v0.1.1.

### D7 — Ticker conversion at dispatch boundary (Q6 = (a))

UI stores `(Venue::Yahoo, Symbol("BTCUSDT"))`. Conversion happens in
the Lab runner's `lab_config_to_scenario` (or a successor helper)
via a new `binance_to_yahoo_ticker(sym: &Symbol) -> SmolStr` helper
located in `crates/data/src/yahoo.rs` (feature-gated, NOT under
`crates/ui` so the conversion table is reusable from CLI tooling and
tests).

Conversion table (10 entries; mechanical):

| UI symbol  | Yahoo ticker |
| ---------- | ------------ |
| BTCUSDT    | BTC-USD      |
| ETHUSDT    | ETH-USD      |
| BNBUSDT    | BNB-USD      |
| SOLUSDT    | SOL-USD      |
| XRPUSDT    | XRP-USD      |
| ADAUSDT    | ADA-USD      |
| DOGEUSDT   | DOGE-USD     |
| AVAXUSDT   | AVAX-USD     |
| DOTUSDT    | DOT-USD      |
| LINKUSDT   | LINK-USD     |

The helper rejects any non-`USDT`-suffixed input with
`Err("unmapped ticker: {sym}")`. Multi-asset expansion (equities,
FX, commodities) at v0.2.0 will extend the table; at that point Q6
re-opens for re-litigation per the operator's note ("when
`EURUSD=X` and `GC=F` land, the conversion table becomes painful
enough to want to display Yahoo-native"). The v0.1.0 surface is
locked.

### D8 — `Venue::Yahoo` cascade (K7)

New variant `Venue::Yahoo` on `trading_core::Venue` (in
`crates/core/src/venue.rs:34`). The variant is **additive** —
`#[serde(rename_all = "snake_case")]` produces `"yahoo"` for
serialisation; `FromStr` and `Display` add the row.

Exhaustive-match cascade (clippy `-D warnings` drives the list):

- `crates/core/src/venue.rs::Display` + `FromStr` impls.
- `crates/ui/src/lab/runner.rs` — venue → bar-source dispatch.
- `crates/ui/src/lab/universe.rs` — `YAHOO_CRYPTO_UNIVERSE` const.
- `crates/audit/` — venue display columns (one-arm-add per match).
- `crates/exec/` — venue routing (default-arm "unsupported venue"
  for Yahoo, since Yahoo is read-only historical at v0.1.0).
- `crates/backtest/src/realdata.rs` — explicit reject of
  `Venue::Yahoo` (Yahoo path doesn't use this CLI module).

R-NR.1 gate (anchor byte-identity) is preserved because none of
these match-arm additions mutate report-body strings. The audit
crate's `Venue::Yahoo → "yahoo"` formatting is a NEW row; existing
Binance / Coinbase / Kraken rows are untouched.

## Consequences

**Enforced by:**

- `bash scripts/verify_anchors.sh` — `ANCHORS PASS (34 / 34)` at
  every developer commit (R-NR.1 gate).
- `cargo build -p data` (default features, no `yahoo`) — succeeds
  on a machine without `data/yahoo/`. The `yahoo` feature is
  default-off so the `yahoo_finance_api` crate is NOT pulled into
  the default-feature dep graph. R-NR.7 gate.
- `cargo build -p data --features yahoo` — pulls the new dep,
  exposes `YahooBarSource`, gates `fetch_yahoo_klines` CLI.
- `cargo test -p data --features yahoo --test yahoo_revision_verify`
  — fixture-based revision-pin round-trip + tamper detection.
- `cargo test -p ui --features live,yahoo` — Lab dispatch happy path
  against a fixture Yahoo cache.
- `spec-lint` — clean baseline (R-NR.4).

**What breaks if this is violated:**

- A developer wires `YahooBarSource` directly into the cross-sectional
  scenario arms → those arms emit anchored reports; the Yahoo path
  flips their body SHA. Caught by `verify_anchors.sh`.
- A developer makes the Yahoo response-body checksum part of the
  aggregate-SHA → the same Yahoo data fetched twice produces
  different aggregate SHAs (Yahoo's response body has wall-clock
  metadata). Caught by H4 hypothesis test (developer Wave E).
- A developer puts `binance_to_yahoo_ticker` in
  `crates/ui/` instead of `crates/data/` → CLI tooling can't reuse
  it. Caught by spec-lint cross-reference + architect review.
- A developer makes `data_source` a non-`#[serde(default)]` field
  on `ScenarioConfig` → every existing call site must update.
  Caught by the (large) compile-error cascade at developer Wave C-3.

**What this enables:**

- **Operator multi-asset Lab pivot** — the v0.2.0 follow-on adds
  equities (`AAPL`, `SPY`, `QQQ`, ...), FX (`EURUSD=X`, ...),
  commodities (`GC=F`, `CL=F`) by extending `YAHOO_CRYPTO_UNIVERSE`
  + the conversion table; no architectural change.
- **Operator-approved Yahoo anchors at v0.1.1** — once a sample
  Yahoo backtest is operator-approved, future Yahoo runs lock under
  new scenario IDs (e.g., `yahoo-btc-usd-2024-1d-sma-cross`); the
  existing 34 stay byte-immutable.
- **Generalised data-revision pin** — ADR-0032 + ADR-0040 together
  document the protocol for any future paid-tier upgrade (Yahoo
  Premium, Polygon.io, Refinitiv) without re-opening the
  cross-cutting `crates/data` / `crates/backtest` boundary.

## Cross-references

- [ADR-0032](0032-backtest-realdata-path-and-revision-pin.md) —
  Binance precedent; revision-pin protocol generalisation source.
- [`spec/lab-yahoo-realdata/feature.md`](../../lab-yahoo-realdata/feature.md)
  — analyst brief; R1-R7 + R-UI-1 + R-NR mapping.
- [`spec/lab-yahoo-realdata/decomp.md`](../../lab-yahoo-realdata/decomp.md)
  — architect decomp (this ADR's implementation file:line citations).
- [`spec/lab-end-to-end-v2/feature.md`](../../lab-end-to-end-v2/feature.md)
  — predecessor that extracted single-symbol scenario dispatch arms
  with `bars_override`; D-2c "Binance Lab wiring" SUPERSEDED by this
  feature.
- [`crates/backtest/src/realdata.rs`](../../../crates/backtest/src/realdata.rs)
  — `RealDataBarSource` is the API shape `YahooBarSource` mirrors.
- [`data/binance/REVISION.toml`](../../../data/binance/REVISION.toml)
  — the manifest format generalised here.

## Changelog
- 2026-05-24 (architect, M-T1): initial accept. Locks D1-D8: module
  placement (`crates/data/src/yahoo.rs` feature-gated `yahoo`),
  `yahoo_finance_api 4.1.x` external dep (CLAUDE.md gate satisfied),
  revision-pin protocol (mirrors ADR-0032 + cadence subdir + per-fetch
  response checksum forensics), source-agnostic engine + Lab-side bar
  swap (Q1 = (b)), `YahooBarSource` API surface with
  `load_cached`/`fetch_and_cache` split, adaptive cadence
  (Q4 = (c)), boundary ticker conversion (Q6 = (a)), `Venue::Yahoo`
  variant cascade (K7). 34/34 anchors stay byte-identical; closes
  T-AR3 / T-AR6 of `spec/lab-yahoo-realdata/tasks.md`.
- 2026-05-27 (architect, M-T1 lab-yahoo-realdata-v0.1.2): per-ticker
  scaling pattern + aggregate cache-state UI surface. **No new
  architectural decisions** — operationalises existing D3 + D4 + D7
  for multi-ticker. Two operational extensions:
  (1) **Per-ticker scaling pattern.** `crates/backtest/src/bin/run_yahoo_sma.rs`
  gains a `--ticker <T>` Clap arg validated against a 10-row
  `const ALLOWED_YAHOO_TICKERS` mirror of the D7 table RHS. Scenario
  name derives mechanically: `{lc-ticker-no-USD}-yahoo-2024-1d-sma-cross`
  (BTC-USD → `btc-yahoo-2024-1d-sma-cross`, ETH-USD →
  `eth-yahoo-2024-1d-sma-cross`, …). Default-arg invocation
  (`--ticker BTC-USD` implicit) is byte-identical to v0.1.1 anchor 69
  `8045623b…`. New anchors are append-only per
  `lab-yahoo-realdata-v0.1.{N}` namespace (one per ticker per release).
  Cross-crate pinned-table test in `crates/backtest/tests/run_yahoo_sma_ticker_flag.rs`
  locks the 10-row mirror to `data::yahoo::binance_to_yahoo_ticker`
  source-of-truth. Three-mirror crate-graph rationale (data → ui →
  backtest) documented at `spec/lab-yahoo-realdata-v0.1.2-…/feature.md` § D-V0.1.2-2.
  (2) **Aggregate cache-state UI surface.** New `cache_state_summary_badge`
  widget (sibling of v0.1.0 per-pair `cache_state_badge`) reads the
  same `REVISION.toml` already pinned by D3. Probe extension
  `cache_state::probe_summary` performs a bounded 30-stat fan-out
  (10 tickers × ~3 dir levels) → `CacheSummary { populated_count,
  newest_mtime }`. Cached on `LabState::cache_summary: Option<CacheSummary>`
  and invalidated on `data_source` toggle + Lab-Run-complete events
  (operator-decided coarse cadence; no background polling, no
  per-frame re-stat). Rendered in the Lab tab toolbar row
  (operator Q2 2026-05-27 override of analyst's source-toggle-row
  recommendation), independent of `data_source` selection — visible
  whenever Lab is active. 69/69 anchors stay byte-identical;
  v0.1.2 appends row 70 (`eth-yahoo-2024-1d-sma-cross`). Closes
  T-T1.6 of `spec/lab-yahoo-realdata-v0.1.2-…/tasks.md`.
- 2026-05-28 (architect, M-T1 lab-yahoo-realdata-v0.1.3): body→frontmatter
  migration for the `rev=<sha>` substring in Yahoo report emissions,
  plus registration of `eth-2024-h1-sma-cross` Binance H1 scenario.
  **No new architectural decisions** — operationalises existing D3
  (`REVISION.toml` aggregate SHA) + extends the per-ticker scaling
  pattern from D-V0.1.2-6 to the report-emit boundary. Two operational
  extensions: (1) **Canonical Yahoo report-emit helper.**
  `crates/backtest/src/report/yahoo.rs` becomes the single point of
  truth for Yahoo-cache-sourced report emission. The body
  `Data source: yahoo-cache:{ticker}/1d/2024 rev={sha:.12}` (D-V0.1.2-6
  default) loses the `rev=<sha>` suffix; the full 64-char hex moves
  to a new top-level frontmatter line `revision_sha:` immediately
  after `data_source:`. Underlying strategy report writers
  (`report::sma::write`, future `_macd`/`_rsi`/`_bbands` at v0.2.0+)
  gain an optional `revision_sha: Option<&str>` parameter — `None`
  preserves byte-identical output for the 33 Binance SMA anchors and
  all non-Yahoo emitters. Anchor row 69 BTC SHA updates in-place under
  namespace `lab-yahoo-realdata-v0.1.1` (Q2=(a) precedent: v5 v0.3.0+v0.4.0
  in-place re-emit; ADR-0038 § D6.b wiring-bug-fix re-emission
  protocol applies). Row 70 ETH daily byte-identical at v0.1.3 — bulk
  Yahoo ticker re-emit deferred to v0.1.4 BNB ship to amortize across
  9 unanchored tickers. (2) **Binance ETH H1 scenario registration.**
  `eth-2024-h1-sma-cross` arm appended to `crates/backtest/src/main.rs`
  at three match-arm sites (L242 scenario config mirroring
  `btc-2024-h1-sma-cross`, L1029 synthetic fallback start price
  `dec!(2_400)`, L1762 `scenario_to_feature → "v0-paper-sma"`); retires
  the v0.1.2 Yahoo-to-Yahoo K1 fallback in favor of direct
  Yahoo-daily-vs-Binance-hourly H1 discharge. New anchor row 71 under
  namespace `lab-yahoo-realdata-v0.1.3`. Net anchor count 70 → 71.
  Closes T-T1.4 of
  `spec/lab-yahoo-realdata-v0.1.3-rev-frontmatter-and-binance-eth-h1/tasks.md`.
