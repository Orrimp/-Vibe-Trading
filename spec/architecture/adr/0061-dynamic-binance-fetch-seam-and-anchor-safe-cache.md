---
adr: 0061
title: Dynamic on-demand Binance fetch seam + anchor-safe dynamic cache separation
status: accepted
date: 2026-06-21
supersedes: none
superseded-by: none
---

# ADR-0061: Dynamic on-demand Binance fetch seam + anchor-safe cache separation

## Context

The advisor bake-off (`backtest::run_bakeoff`, the F3 guided-input flow,
ADR-0059) preloads bars ONCE from the **pinned** Binance corpus
(`data/binance/`, 10 curated symbols × 2021–2024 hourly) via
`preload_bakeoff_binance_bars` (`crates/backtest/src/bakeoff/mod.rs:74`). When
the operator picks a `(coin, lookback)` the corpus does not cover — the
**recent** windows (`DateRange::Last30d`/`Last90d`, which are wall-clock-relative
and slide past the 2024 cap **every day**) or an arbitrary post-2024 `Custom`
range — the preload returns `RunError::Internal("0 bars in range")`. The bake-off
surfaces an error instead of an answer. The operator wants the app to **fetch the
data on demand** for whatever coin + window is picked, then rank on it.

The fetch logic already exists in the CLI bin
`crates/data/src/bin/fetch_binance_klines.rs` (paginated REST against
`https://api.binance.com/api/v3/klines`, `build_klines_url`, `paginate_klines`
with a `last_close + 1` cursor, parse to `Kline`, `write_parquet` to
`<SYM>/<YEAR>/<MM>.parquet`), and `KlineFetcher` is already a mockable trait.

This decision is load-bearing for two reasons, each carrying a durable contract:

1. **Anchor-safety (HARD non-negotiable).** A live-data fetch + its cache must
   NOT mutate the pinned corpus, its `REVISION.toml`, the 119 anchored backtest
   scenarios, or `scripts/verify_anchors.sh` (must stay 119/119). This is a
   git-boundary + write-isolation contract, exactly the shape ADR-0055 settled
   for `lab-runs/`.
2. **Determinism boundary.** Live-API data is not reproducible (Binance revises/
   extends recent bars; `Last30d` slides daily). It must stay strictly OUT of the
   anchored / determinism-pinned path (ADR-0053 § D6, ADR-0032). The bake-off on
   dynamic data is **exploratory**, not anchored — a deliberate inverse of the
   pinned corpus's whole purpose.

Hence an ADR: a new cross-crate seam (`data` library fetch fn + a dynamic cache
+ a `backtest` resolver), an anchor-safety topology with a git boundary, weighed
alternatives, and a precedent for future dynamic-data surfaces.

## Decision

**D1 — Extract the fetch into a reusable `data` library function.** Move the
bin's already-tested pure + fetch pieces (`Kline`/`RawKline::parse`,
`build_klines_url`, `KlineFetcher` + `HttpKlineFetcher`, `paginate_klines`,
`write_parquet`) into a new module `crates/data/src/binance_klines.rs`; the bin
re-exports them so the CLI + its parquet-write path are byte-unchanged. Add the
public entry point
`fetch_binance_klines_range(symbol, start_ms, end_ms, interval) -> Result<Vec<trading_core::Bar>, BinanceFetchError>`:
paginated `limit=1000`, polite 200ms pacing (≤300 req/min vs Binance's ~1200
weight/min; klines weight 1–2/call), one exponential-backoff retry on
`RateLimited` honouring `Retry-After`, each `Kline` mapped to `Bar` by a private
`kline_to_bar` (same decimal-string→`Price`/`Quantity` parse `read_parquet_bars`
performs; `local_recv_ts = close_ts` per ADR-0032 § D1 Step 7). The
`KlineFetcher` trait IS the R3 "external I/O behind a trait" seam — every test
injects a mock fetcher; no test hits a live socket.

**D2 — Typed error model, no panics.** `BinanceFetchError` (thiserror):
`Network` (DNS/refused/TLS), `Timeout`, `RateLimited { http_status,
retry_after_secs }` (HTTP 429/418), `UnknownSymbol` (HTTP 400 + Binance code
`-1121`), `NoDataForRange` (HTTP 200, zero bars — returned **instead of**
`Ok(vec![])` so callers branch on a typed no-data, mirroring
`YahooError::NoDataForRange`), `Other` (any other non-2xx / malformed body). A
pure `classify_binance_error(status, body)` does the mapping so it is
unit-testable without a socket (the `yahoo::classify_yfa_error` precedent). No
`.unwrap()` / panic on any path (CLAUDE.md library rule).

**D3 — Fetch-the-WHOLE-window for not-covered ranges (NOT per-gap stitching).**
For a `(coin, window)` the bake-off loads the pinned corpus when it **fully
covers** the window, else fetches the **whole** window from the dynamic cache.
v0.1.0 does NOT splice a pinned prefix onto a dynamic suffix. Rationale: (a)
correctness over cleverness — the apples-to-apples invariant clones ONE preloaded
`Vec<Bar>` to every arm, so a one-bar seam error at the pinned↔dynamic boundary
would corrupt all candidates identically and silently; fetching from one source
removes the seam; (b) the windows that miss the corpus are the **recent** ones,
entirely outside 2021–2024, so "fetch the gap" == "fetch the whole window" for
them anyway; gap-stitching only helps a rare 2024-straddle window. The resolver
`resolve_bakeoff_bars(symbol, range, data_source)` (new in `bakeoff/`): non-
`BinanceCache` → existing path unchanged; `BinanceCache` + `covers(window,
pinned)` → existing read-only REVISION-verified pinned path; else →
`dynamic_cache::load_or_fetch`. A straddle is treated as not-covered (fetch the
whole window — a correct superset; the partial pinned prefix is simply not
reused). Gap-stitching is a noted v0.2.0 follow-up.

**D4 — Anchor-safe cache separation is BY CONSTRUCTION (three mechanical
guarantees, not reviewer vigilance — the ADR-0055 D2 discipline).**
- **(a) Separate, git-ignored root.** Dynamic bars go to
  `data/binance-dynamic/` (`dynamic_cache::BINANCE_DYNAMIC_ROOT`), never
  `data/binance/`. `.gitignore` already has `/data/*` (line 12) with `!`-
  allowlist exceptions **only** for the committed-REVISION corpora
  (`data/yahoo/`, `-funding`, `-broaduni`, `-basis`, `-2122`). The dynamic root
  adds **no `!` exception** ⇒ git-ignored by the existing rule ⇒ never committed
  ⇒ invisible to every `find spec/ …` the anchor machinery runs. Only a
  documenting comment block is added to `.gitignore` (no rule change).
- **(b) `verify_anchors.sh` is structurally blind to it.** The verifier resolves
  all 119 scenarios via `find "$root"/spec -type f -path "*/reports/…"` — it only
  walks `spec/`. A sibling `data/binance-dynamic/` cannot be discovered; no row
  is added to `spec/anchors.toml`; no anchored body-SHA is touched. **119/119 by
  construction.**
- **(c) Pinned corpus read-only on the dynamic path; its REVISION pin still
  verifies.** The dynamic branch NEVER calls `write_parquet` into
  `BINANCE_CORPUS_ROOT` and NEVER calls `write_revision_manifest` /
  `regenerate_revision_manifest` on it — contrast `yahoo::fetch_and_cache`, which
  regenerates the Yahoo manifest; the dynamic root has **no** `REVISION.toml` at
  all (D5), so there is nothing to regenerate. `preload_bakeoff_binance_bars`
  keeps its read-only `read_and_verify_revision_manifest(data/binance)` check,
  which passes unchanged. `data/binance/REVISION.toml` is out of scope for any
  write.

The enforcement test (`crates/data/tests/dynamic_cache_anchor_safety.rs`, mock
fetcher, dynamic root = tempdir, sentinel corpus fixture) asserts: corpus files'
(path, mtime, content-SHA) identical before/after `load_or_fetch`; no
`REVISION.toml` under the dynamic root; the fixture's revision-verify still Ok
with the unchanged aggregate SHA. Plus `scripts/verify_anchors.sh` → 119/119 as
the bake-off-hook acceptance gate.

**D5 — Determinism boundary: dynamic data is exploratory, never anchored.** The
dynamic root carries **no `REVISION.toml`** — no determinism pin, because the
source is not reproducible (the deliberate inverse of the pinned corpus, whose
purpose is the ADR-0032 determinism contract). No dynamic bar reaches an anchored
report: the bake-off writes no report body (ADR-0059 anchor-additive), and the
dynamic branch is reachable ONLY from `BinanceCache` + a not-covered window — the
anchored CLI scenarios all use covered 2021–2024 windows, so they never take it.
Same-seed-every-arm still holds WITHIN a run (all arms see the one fetched
`Vec<Bar>` — apples-to-apples preserved); it is ACROSS runs that dynamic data is
non-reproducible, by design. The Leaderboard window-context copy states the
window is "fetched live" (operator honesty; the existing not-advice disclaimer
frames the result as exploratory).

**D6 — Cache fidelity via the ReplayFeed round-trip.**
`dynamic_cache::load_or_fetch` reuses the corpus's `<SYM>/<YEAR>/<MM>.parquet`
month layout (ADR-0056) so `ReplayFeed` reads it with only a ROOT change. On a
cache miss it `write_parquet`s the fetched months into the dynamic root, then
reads them back through `ReplayFeed::new(BINANCE_DYNAMIC_ROOT, true)
.merge_symbols(...)` clipped to `[start_ms, end_ms)` — re-reading through
`ReplayFeed` (not returning the in-memory fetch) guarantees a dynamic bar is
byte-for-byte the same `Bar` a corpus bar would be, and that the cache-hit and
cache-miss paths return identical bars. The partial trailing (current) month is
re-fetched (the bin's short-month case, but WITHOUT a REVISION pin — there is
none to compare against).

**D7 — Curated coin set + 1h interval retained; `ui` purity preserved.** The
`BAKEOFF_COIN_UNIVERSE` 10-symbol picker is unchanged for MVP (every entry is a
liquid Binance USDT pair → dynamic fetch resolves; `UnknownSymbol` is reduced to
defence-in-depth). The picker's "must be in the **pinned** corpus" unit-test
constraint relaxes to "in the curated set" (the dynamic path lifts corpus-
coverage). The fetcher is called with `interval = "1h"` (the bake-off is hourly
end-to-end). `ui` imports NO new crate: the fetch lives in `data`/`backtest`,
triggered through the existing `spawn_bakeoff` seam (ADR-0059); the typed errors
become `RunError::Internal(<friendly copy>)` → the already-wired
`PanelState::Error`, and the operator copy lives in `ui::strings`. Picker
expansion + non-hourly intervals are noted v0.2 follow-ups.

## Alternatives considered

- **Fetch-the-gaps (splice pinned prefix + dynamic suffix).** Saves a re-fetch on
  a 2024-straddle window, but introduces a boundary-bar seam (duplicate/missing
  bar, off-by-one on the half-open clip) that the apples-to-apples ONE-preload
  invariant would propagate silently to every candidate. Rejected for v0.1.0 (the
  miss windows are entirely-recent so there is no overlap to reuse anyway);
  retained as a v0.2 follow-up if a straddle window proves common.
- **Reuse `data/binance/` as the cache (write fetched bars into the pinned
  corpus + regenerate its `REVISION.toml`).** The `yahoo::fetch_and_cache`
  shape — but it would mutate the anchored corpus + its revision pin, directly
  violating the HARD non-negotiable and breaking `verify_anchors.sh`. Rejected
  categorically.
- **In-memory-only fetch (no on-disk cache).** Simplest, anchor-safe trivially,
  but every bake-off run re-fetches (seconds each) — fails the "cache so repeat
  runs are fast" requirement (R2). Rejected; the git-ignored on-disk cache is the
  same cost with the repeat-run win.
- **Audit-DB / SQLite cache for dynamic bars.** Queryable, but discards the
  `ReplayFeed` parquet reader (D6's byte-fidelity guarantee) and adds a schema +
  migration for no MVP benefit. The right home only if dynamic runs later need
  queryable metadata; out of scope (mirrors ADR-0055's same call for `lab-runs/`).
- **A new `live`/`fetch` crate.** Over-structured: the fetch is a `data` concern
  (reqwest already there) and the resolver a `backtest` concern; a new crate adds
  a dep edge for nothing. Rejected.
- **Per-gap REVISION pin on the dynamic root.** Would imply the dynamic data is
  reproducible, contradicting D5; and a pin the operator can't reproduce is worse
  than no pin. Rejected — no `REVISION.toml` on the dynamic root is the correct
  determinism statement.

## Consequences

- **Anchor gate:** `scripts/verify_anchors.sh` stays **119/119** after any number
  of dynamic fetches — by construction (the dynamic root is outside every
  `spec/**` glob and outside the pinned corpus write path). `spec/anchors.toml`
  is untouched; no anchored body-SHA moves.
- **Byte-immutability:** anchored reports in `spec/*/reports/` are never written
  by the dynamic path (CLAUDE.md non-negotiable / ADR-0038 § D6 upheld by the git
  boundary + write-isolation, not reviewer vigilance). No anchor-mutation ADR is
  required (the 9 anchors in `spec/anchors.toml` do not change).
- **Determinism:** the dynamic path is explicitly exploratory (no REVISION pin,
  no anchored report); ADR-0053 § D6 / ADR-0032 scope is preserved — the pinned/
  anchored path is byte-identical. Same-seed-every-arm holds within a run.
- **Money discipline:** `kline_to_bar` parses OHLCV into `Price`/`Quantity`
  (the existing `read_parquet_bars` Decimal path); no new `f64` money (ADR-0003).
- **No new dependency:** the fetcher + cache reuse `reqwest` (rustls — single-
  binary-friendly, no system C dep), `serde_json`, `tokio`, `thiserror`, `time`,
  `async-trait`, `polars`, all already in `crates/data`'s edition-2024 graph
  (CLAUDE.md crate-compat checklist satisfied; nothing to add to architecture.md
  beyond the new-module note).
- **No live trading:** this fetches read-only public market data for backtesting;
  it touches no order/execution path (`live-trading-removed-2026-06-12.md` scope
  retained).
- **UX is mostly already wired:** `PanelState` Loading/Error + `begin_run()`/
  `finish_run()` + the side-thread `spawn_bakeoff` dispatch already exist; the
  delta is honest error copy for the new failure modes + an optional coarse fetch
  progress, both through existing seams. The new error copy is verified at the
  rendered-pixel layer (`leaderboard_populated_render.rs`), per CLAUDE.md.
- **Future dynamic surfaces:** this establishes the pattern (git-ignored sibling
  root, no REVISION pin, ReplayFeed round-trip, typed fetch errors) any later
  on-demand data source (other venues, other intervals) should follow.

## Changelog

- 2026-06-21 (architect): initial accept. Extract the bin's Binance-klines fetch
  into `data::binance_klines::fetch_binance_klines_range`; add a git-ignored
  `data/binance-dynamic/` cache (`dynamic_cache::load_or_fetch`, no REVISION
  pin); hook `resolve_bakeoff_bars` into the bake-off preload (fetch-the-whole-
  window for not-covered ranges, ADR-0059 seam). Anchor-safe by construction
  (separate git-ignored root + verifier blind to non-`spec/` paths + corpus
  read-only); determinism boundary = dynamic data is exploratory, never anchored.
  Feature `advisor-dynamic-data`. Leans on ADR-0055 (git-ignored-root precedent),
  ADR-0053 § D6 / ADR-0032 (determinism), ADR-0059 (bake-off seam), ADR-0056
  (corpus parquet layout).
