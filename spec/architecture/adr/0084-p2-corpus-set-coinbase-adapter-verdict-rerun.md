---
adr: 0084
title: P2 corpus set + Coinbase second-venue adapter + multi-corpus verdict-rerun harness
status: accepted
date: 2026-07-09
supersedes: none
superseded-by: none
---

# ADR-0084: P2 corpus set + Coinbase second-venue adapter + multi-corpus verdict-rerun harness

## Context

The product's terminal thesis — **"no active strategy robustly beats
buy-and-hold net of costs"** (2026-06-08 ship-passive verdict, `spec/product.md`)
— rests on a narrow evidence base: **one venue** (Binance), **one bar size**
(hourly), **~2 regimes** (pinned 2023-24 `3a8b96c4…` + 2021-22 `4f390622…`,
plus the 35-symbol broad-universe corpus for cross-sectional width). The
product's own overfitting scorecard says the honest minimum backtest length to
*trust* a crown (`MinBTL ≈ 6.4 years` at `SR_target=1, N_eff=24`,
`bakeoff/scorecard.rs:46`) exceeds the ~2 years per regime the pinned corpora
carry.

**Remediation plan P2** (`spec/backlog.md` § Remediation plan, ratified
2026-07-09) extends the evidence base + re-runs the verdict. Feature brief:
`spec/v3/advisor-corpus-expansion/feature.md` (analyst M-T0), requirements
R1–R8, open questions Q-CE-1..7. This ADR records the M-T1 architecture
decisions.

Three decisions are ADR-worthy because they set durable conventions future
features inherit:

1. **A second reconcilable venue** enters the data layer for the first time as
   a *historical backfill* path (the shipped `coinbase.rs` is a live-WS feed,
   not a REST backfiller — see D2). This establishes the "mirror the Binance
   fetcher for a new venue" pattern.
2. **A new pinned-corpus set** (4 corpora) extends the ADR-0032 REVISION.toml
   pin convention to new windows/venues; the existing pins stay byte-immutable.
3. **A multi-corpus verdict-rerun harness** establishes how the *existing*
   bake-off + null-CI are replayed over N corpora without touching the shipped
   runner or the FROZEN gate.

The analyst availability reality-check (code + API grounded) is load-bearing and
is adopted verbatim: **Kraken REST OHLC deep-hourly history is INFEASIBLE**
(720-candle total cap; `since` advances the forward cursor only, no backward
paging — freqtrade #2134; ~5 days reachable on 1h). Coinbase Exchange
`get-product-candles` is the durable alternative (windowed backward pagination
to full listing history; Coinbase already `VenueTrust::HighReconcilable` in the
shipped P1-7 DATA-quality panel).

## Decision

### D1 — Corpus set (Q-CE-1): RATIFY the analyst's 3-corpus Binance set, unchanged

Three new pinned Binance hourly corpora, additive `--out` dirs, each with its
own `REVISION.toml`, symbol subsets driven by honest full-coverage:

| Corpus dir          | Window (UTC)          | Symbols (honest subset)                                  | Regime |
|---------------------|-----------------------|----------------------------------------------------------|--------|
| `data/binance-1718` | 2017-08-01 → 2018-12-31 | BTCUSDT, ETHUSDT, BNBUSDT                                | 2017 mania blow-off + 2018 bear |
| `data/binance-2020` | 2020-01-01 → 2020-12-31 | BTCUSDT, ETHUSDT, BNBUSDT, XRPUSDT, ADAUSDT, LINKUSDT, DOGEUSDT | COVID crash (Mar-2020) + recovery |
| `data/binance-2526` | 2025-01-01 → last fully-closed UTC month at fetch time | all 10 | recent 2025-26 |

Rationale for subsets (verbatim from R1): a symbol whose Binance listing
post-dates a corpus's start yields empty/ragged early months — the fetcher warns
+ skips (no crash), but presenting it as "N coins" is dishonest. Each corpus
carries **only symbols with contiguous full coverage across its window**. BTC+ETH
are in every corpus (the only two with 2017→now contiguous). `binance-2020`
deliberately excludes DOT/SOL (2020-08) + AVAX (2020-09) — they would be
half-empty. No corpus added or trimmed beyond the analyst set: it captures
mania/crash + COVID + recent with honest subsets and stays bounded (not a data
lake).

### D2 — Second venue (Q-CE-2, THE key decision): Coinbase Exchange hourly. RATIFIED.

**Verdict: Coinbase-hourly (analyst option 1). Ratified, not overturned.**

Grounds (durable > cheap, per operator standing preference):

- It is the only option that gives a **true apples-to-apples hourly** venue
  cross-check on the *same* windows as the Binance corpora. Kraken-daily
  (fallback) is coarser (daily bars) — it would test venue-dependence on a
  *different* bar size, confounding the comparison.
- It **reuses the proven windowed-pagination pattern**. Coinbase
  `get-product-candles` caps at **300 candles/call** BUT `start`/`end` page
  backward to full listing history — the identical "iterate windows, page within
  each" shape the Binance fetcher already implements. On hourly, 300 candles =
  12.5 h-days, so a calendar month (~720 h) needs ~3 sub-windows.
- Coinbase is **already trust-blessed** — `VenueTrust::HighReconcilable` names
  Binance / Coinbase / Kraken in the shipped DATA-quality panel; BTC-USD hourly
  reaches ~2015-16 (deeper than Binance).

**Crucial correction to the assumption that `coinbase.rs` can be reused for
backfill:** the shipped `crates/data/src/coinbase.rs` is a **WebSocket live-feed
adapter** (Advanced Trade `candles` channel, streaming 1-minute bars for
paper-sim) — it does NOT hit the historical REST `get-product-candles` endpoint
and CANNOT backfill. Therefore a **new `fetch_coinbase_klines` bin + a new
`crates/data/src/coinbase_klines.rs` library module** are genuinely required
(D2.a). The one reuse: the existing `data::coinbase_symbol_map` helper already
maps `BTCUSD → BTC-USD` (`coinbase.rs:162`).

**D2.a — the Coinbase fetcher (mirror of the Binance one, ONE real seam).** New
`crates/data/src/coinbase_klines.rs` + `crates/data/src/bin/fetch_coinbase_klines.rs`,
mirroring `binance_klines.rs` + `fetch_binance_klines.rs`:

- **Same output layout + parquet schema** — `<out>/<SYMBOL>/<YEAR>/<MM>.parquet`
  with the identical 8-column `replay_feed.rs` schema (`open_time`,
  `close_time`, `open`, `high`, `low`, `close`, `volume`, `trade_count` — all
  price/volume as Utf8 strings, times as Int64 millis). This is the
  venue-agnostic contract `ReplayFeed` reads (A3 verified: `merge_symbols` reads
  by column name, venue-neutral).
- **The seam = the venue mapping shim.** Coinbase differs in FOUR ways the
  adapter must translate at parse time, all confined to `coinbase_klines.rs`:
  1. **Symbol string:** `BTCUSDT` (Binance) vs `BTC-USD` (Coinbase). The
     corpus stores the on-disk symbol dir as **`BTCUSDT`** (the engine's
     canonical `Symbol`), so the bin takes a `--symbols BTCUSDT` and maps to the
     product-id `BTC-USD` for the REST call via `coinbase_symbol_map`. This
     keeps the corpus consumable by `resolve_bakeoff_bars`/`ReplayFeed` with the
     same `Symbol::new("BTCUSDT")` the rest of the engine uses (A3's "one likely
     seam" resolved: normalize to the Binance symbol string on disk).
  2. **Candle array order:** Coinbase returns `[time, low, high, open, close,
     volume]`; Binance's `RawKline` is `[open_time, open, high, low, close,
     volume, close_time, …]`. The Coinbase parser maps positionally into the
     canonical `Kline` struct (reuse `binance_klines::Kline` verbatim; it is
     venue-neutral).
  3. **Timestamp unit:** Coinbase `time` is **seconds**; the schema stores
     **millis**. Multiply by 1000 on parse. `close_time` is synthesized as
     `open_time + granularity_ms − 1` (Coinbase returns only the open time).
  4. **No `trade_count`:** Coinbase candles omit it → store `0` (the same
     sentinel `coinbase.rs:299` uses for live candles). `expected_bars_per_month`
     verify + the `should_skip` idempotency path work unchanged (they key on
     row count + content-SHA, not `trade_count`).
- **Pagination:** a new `paginate_coinbase_candles` mirroring `paginate_klines`
  but with a **300-candle window step** (not 1000) and a **≥200 ms inter-request
  pace** (Coinbase public rate limit is ~10 req/s; 200 ms = 5 req/s, safe). The
  windowing is **caller-driven forward sub-windows within each month** (Coinbase
  ignores `start`/`end` unless BOTH are set and rejects >300-point spans), which
  is the deep-history property: iterating months back to listing IS the backward
  paging.
- **Idempotency + REVISION.toml:** reuse `data::revision::write_revision_manifest`
  + the `should_skip` content-SHA convention verbatim — the corpus is byte-stable
  on re-fetch exactly like Binance.
- **`--emit-revision-manifest`** identical flag.

**D2.b — the cross-check corpus (Q-CE-2 scope): `data/coinbase/`, BTC-USD hourly,
2020-01-01 → last-closed-month, BTC only.** The venue-dependence question is
"does the verdict change on a different venue's *same-asset* price series", NOT
"re-run the whole universe on Coinbase". 2020→ is the deepest window where a
*direct* Binance-vs-Coinbase hourly comparison is possible on BTC across ≥3 of
the Binance corpora (2020, 2122, 2324, 2526). On disk the symbol dir is
`BTCUSDT` (D2.a normalization).

**D2.c — Kraken is explicitly OUT of P2.** Kraken-daily (fallback) and
Kraken-hourly-via-CSV (heaviest) are both rejected for P2: daily confounds the
bar-size comparison; the CSV path is a new non-REST ingest adapter
disproportionate for a bounded cross-check. If a future feature needs Kraken
deep-history, the CSV-dump adapter is a separate ADR.

### D3 — Exogenous back-fill scope (Q-CE-3): FETCH DVOL + macro, bounded + additive. RATIFIED analyst lean.

The whole point of P2 is wider evidence; a warm-up-only proxy arm is not a real
test of the DVOL/macro thesis. Back-fill, all additive (existing pinned SHAs
byte-identical — new years/months are new files):

- **DVOL** (`fetch_deribit_dvol`, BTC/ETH, per-year parquet): fetch **2021 +
  2022 + 2025 + 2026** so `v0.dvol_regime` is genuinely evaluable on the 2122 +
  2526 corpora. (2023-24 already on disk; DVOL history reaches 2021-04, so
  2021's early months before 2021-04 are legitimately short — the arm keys on
  the available span.)
- **macro** (`fetch_yahoo_klines` → `data/yahoo-macro/`, DXY/GSPC/TNX daily,
  per-month parquet): fetch **2025 + 2026** so `v0.macro_riskon` is evaluable on
  the 2526 corpus. (2021-2024 already on disk; macro starts 2021 → the 1718 +
  2020 corpora legitimately have no macro → that arm is ABSENT there, framed as
  "not evaluable (no macro before 2021)".)

**Not back-filled:** perp-basis + funding (basis is 2023-24 only by
construction; those MN arms stay ABSENT on all other corpora — the honest
graceful-degradation path). The per-corpus arm-availability matrix (R4) is
adopted verbatim as the harness's arm-selection contract.

### D4 — Re-run harness seam (Q-CE-7): a DEDICATED `p2_verdict_rerun` harness. RATIFIED analyst lean, grounded.

**Decision: a dedicated `crates/backtest/tests/p2_verdict_rerun.rs` harness that
reproduces `run_bakeoff`'s per-arm sequence over each corpus — NOT a
`--corpus <dir>` selector on the shipped runner.**

Decisive code grounding: `run_bakeoff` resolves real bars through
`resolve_bakeoff_bars` → `preload_bakeoff_binance_bars`, which **hardcodes**
`BINANCE_CORPUS_ROOT = "data/binance"` (`bakeoff/mod.rs:101`, a `const`; the fn is
`pub(crate)`). Pointing the shipped runner at `data/binance-1718` would require
adding a corpus-root parameter to `BakeoffConfig`/`BakeoffRequest` (public,
frozen-adjacent config touched by every UI call site) — a larger, riskier change
than the honest multi-corpus loop warrants.

The clean seam already exists and is proven twice:
`realdata_simple_strategy_bear_survey.rs:168` loads an arbitrary corpus via
`ReplayFeed::new(root.join("data/binance-2122"), true).merge_symbols(...)` →
`Vec<Bar>`; `null_data_no_crown.rs::run_field_and_rank` reproduces
`run_bakeoff`'s EXACT per-arm sequence (`run_scenario` with `bars_override` →
`derive_candidate_kpis` → `derive_master_seed` + `compute_robustness_flag` →
`rank_candidates` → `compute_scorecard`) against caller-supplied bars. The P2
harness **composes these two proven pieces**: for each `(corpus_root, symbol,
supported_arm_field)`, load bars via `ReplayFeed` and run the null-CI's
`run_field_and_rank` verbatim. Every function called is the identical production
function `run_bakeoff` calls; only the bar *source* differs (a pinned corpus dir
here, `data/binance` there).

Consequences (all D4-load-bearing):

- **Zero shipped-runner change** — `run_bakeoff`/`BakeoffConfig`/`BakeoffRequest`
  byte-untouched; no new public config field; the UI/Lab call sites unaffected.
- **`write_report=false` on every arm** (the `scenario_cfg_for` field in the
  null-CI already sets it) → no anchored CLI report body → **anchors 119/119 by
  construction** (R7). The harness produces only in-memory `FieldOutcome`s +
  writes the NON-anchored re-run report.
- **DVOL/macro arms** need the harness to thread `dvol_override` /
  `macro_regime_series` into `scenario_cfg_for` for the corpora where those arms
  are supported (D3). The harness resolves them via the SAME public
  `resolve_dvol_override` / `load_macro_regime_series` fns `run_bakeoff` uses
  (`bakeoff/mod.rs`), pointed at the (now back-filled) exogenous corpora. Where
  an arm is ABSENT for a corpus (R4 matrix ❌), it is simply **not added to that
  corpus's field** — the report shows it as "not evaluable (no <data>)", never a
  silent drop, never a warm-up-only proxy masquerading as a real evaluation.
- **SKIP-safe** — each corpus's harness fn returns early (eprintln SKIP) when the
  gitignored parquets are absent, mirroring the null-CI + survey SKIP guards, so
  CI without the corpora is green.

### D5 — 2526 end-date clamp (Q-CE-6): clamp `--end` to the last fully-closed UTC month. RATIFIED.

The developer clamps the `binance-2526` (and `data/coinbase`) `--end` to the
**last fully-closed UTC month at fetch time** so no partial trailing month enters
the pin. A still-growing month is a legitimately-short month, but pinning it
breaks idempotent re-fetch (the content-SHA changes as bars accrue). The clamp is
a fetch-time developer step (compute "first day of the current UTC month, minus
one day" → the last day of the prior month); the exact end month is recorded in
the fetch report. The `should_skip` idempotency path already handles genuinely
gapped historical months; the clamp only prevents the *trailing* partial month.

### D6 — Survivorship caveat strength (Q-CE-4): STRONGER worded per-old-era caveat, prose only. RATIFIED.

No code change (the always-present shipped survival note,
`strings::LEADERBOARD_DATA_QUALITY_SURVIVAL_NOTE`, already fires for every
bake-off). The re-run REPORT (tester-authored) carries a **stronger worded
survivor-of-survivors caveat** for the 1718 + 2020 corpora: the 2017-18
BTC/ETH/BNB are the survivors of the survivors (the 2017-18 top-cap universe was
full of coins that no longer exist), so a "these 3 coins over 2017-18" result is
the *most favourable possible slice*, framed as "conditioned on the three largest
survivors" — NOT "how a 2017 coin pick would have done". This is R5 verbatim and
is an acceptance criterion (AC6).

### D7 — Era-cost sensitivity annex (Q-CE-5): YES, opt-in `VolScaledSpread` annex; primary verdict stays on the frozen flat-bps default. RATIFIED analyst lean.

Register **E-2** (do NOT change the flat-8-bps default — frozen for cross-regime
comparability). The primary per-corpus verdict runs the frozen default. The
report ALSO carries an **era-cost sensitivity annex**: each old-era corpus (1718,
2020) re-run once under the opt-in `SlippageModel::VolScaledSpread` (ADR-0081,
opt-in-forever, anchor-safe) to quantify how much an active arm's net edge moves
when spreads widen in stress. This directly quantifies the E-2 caveat instead of
only asserting it in words. It is opt-in and `write_report=false`, so anchors
stay 119/119. The annex is SUPPLEMENTARY — a verdict flip that appears ONLY under
`VolScaledSpread` is reported as "cost-sensitive, not regime-signal".

### D8 — Anchor safety + gate freeze (R7): by construction, NON-NEGOTIABLE

- Every re-run path runs `write_report=false` (D4) → no anchored report body →
  `scripts/verify_anchors.sh` stays **119/119** (run before AND after every
  commit; anchors keyed by NAME not filename). This feature writes only
  NON-anchored `reports/fetch-*.md` + `reports/backtest-<date>-p2-verdict-rerun.md`.
- The FROZEN gate (`bakeoff/robustness.rs::verdict_bands` /
  `compute_robustness_flag`, `rank.rs::classify_verdict`, the ADR-0066 benchmark
  exemption) is **byte-untouched**. This is NOT a band proposal.
  BenchmarkWins/AllFragile reachability UNCHANGED.
- Existing pinned corpora SHAs (`3a8b96c4…` `data/binance/`, `4f390622…`
  `data/binance-2122/`, `data/binance-broaduni/`, DVOL, macro, basis, funding)
  are **byte-immutable**; P2 adds new `--out` dirs only. `ci.yml.deferred` stays
  parked. The €200 stays SIMULATED (no live trading anywhere).
- The 9 `spec/anchors.toml` anchor SHAs are **untouched** — this ADR adds NO
  anchor and mutates none (no anchor-additive re-emission owed; the analyst lean
  is confirmed).

## Consequences

**Positive.** The evidence base behind the ship-passive verdict widens from ~2
regimes / 1 venue to 5 regimes (1718/2020/2122/2324/2526) + a second reconcilable
venue (Coinbase BTC), with `MinBTL` quantified before/after. Both outcomes are
product value (ship-passive survives → stronger claim; it wobbles somewhere →
real signal, surfaced honestly). The "mirror the Binance fetcher for a new venue"
pattern is now established (Coinbase is the reference for any future venue). The
verdict-rerun harness is a reusable multi-corpus replay seam that never touches
the shipped runner or the gate.

**Negative / accepted.** A new venue adapter (~1 dev-day, D2.a) + 4 new corpora
fetches + 4 exogenous back-fill windows are compute/dev cost; the multi-hour
multi-corpus fetch is unavoidable (the developer emits the `watch -n N` probe
block per the long-running-task memory). The Coinbase corpus is BTC-only
(deliberately bounded); a full-universe Coinbase re-run is a follow-on, not P2.
Survivorship on the 2017-18 corpus is extreme and is handled by prose framing
(D6), not by reconstructing dead coins (out of scope — that needs delisted-symbol
data the venue does not serve for historical klines).

**Alternatives considered.**
- *Kraken as the second venue* — REJECTED for hourly (REST OHLC 720-cap, no
  backward paging — infeasible); daily fallback REJECTED (bar-size confound); CSV
  dump REJECTED (new non-REST adapter, disproportionate). D2.c.
- *A `--corpus <dir>` selector on `run_bakeoff`* — REJECTED (touches public
  `BakeoffConfig`/`BakeoffRequest` + the hardcoded corpus root + every UI call
  site; the dedicated harness is smaller and mirrors the proven null-CI
  structure). D4.
- *Reusing `coinbase.rs` for backfill* — IMPOSSIBLE (it is a live-WS feed, not a
  REST backfiller). D2.
- *Full-universe Coinbase cross-check* — REJECTED as out-of-scope for the
  venue-dependence question (BTC is the price-discovery leader; whole-universe is
  a follow-on). D2.b.
- *Changing the flat-bps default for old eras* — REJECTED (frozen for
  cross-regime comparability; E-2 caveat + opt-in annex instead). D7.

## References

- Feature: `spec/v3/advisor-corpus-expansion/feature.md` (R1–R8, Q-CE-1..7,
  A1–A5).
- Charter: `spec/backlog.md` § Remediation plan P2 (ratified 2026-07-09).
- Fetcher pattern: `crates/data/src/bin/fetch_binance_klines.rs`,
  `crates/data/src/binance_klines.rs`, `crates/data/src/bin/fetch_deribit_dvol.rs`
  (per-year non-Binance mirror).
- Harness pattern: `crates/backtest/tests/null_data_no_crown.rs`
  (`run_field_and_rank` — the exact per-arm sequence),
  `crates/backtest/tests/realdata_simple_strategy_bear_survey.rs:168` (arbitrary-
  corpus `ReplayFeed` load).
- Graceful degradation: `crates/backtest/src/bakeoff/mod.rs`
  (`resolve_dvol_override`, macro preload — ADR-0072 / ADR-0073).
- Pin convention: ADR-0032 (REVISION.toml), `crates/data/src/revision.rs`,
  `crates/data/tests/binance_2122_revision_consistency.rs`.
- Venue trust: `spec/dev-notes/venue-trust-map-2026-07-01.md` (ADR-0081),
  P1-7 DATA-quality panel (`VenueTrust::HighReconcilable`).
- Cost annex: ADR-0081 (`SlippageModel::VolScaledSpread`, opt-in-forever).
- Coinbase Exchange `get-product-candles`:
  https://docs.cdp.coinbase.com/api-reference/exchange-api/rest-api/products/get-product-candles
  (300-candle cap; granularity ∈ {60,300,900,3600,21600,86400}; schema `[time,
  low, high, open, close, volume]`; time in seconds).

## Changelog

- 2026-07-09 (architect): accepted. M-T1 design lock for P2 corpus expansion.
  D1 ratifies the 3-corpus Binance set; D2 ratifies Coinbase-hourly as the second
  venue (Kraken infeasible for hourly) + specifies the new `fetch_coinbase_klines`
  bin (the shipped `coinbase.rs` is a live-WS feed, cannot backfill — the ONE
  real seam is the Coinbase→canonical symbol/schema shim); D3 fetches DVOL
  (2021-22 + 2025-26) + macro (2025-26); D4 picks a dedicated `p2_verdict_rerun`
  harness (the shipped runner hardcodes `data/binance` → the null-CI
  `run_field_and_rank` + arbitrary-corpus `ReplayFeed` compose the seam, zero
  runner change); D5 clamps the 2526 end to the last closed UTC month; D6
  stronger survivorship prose for 1718/2020; D7 opt-in `VolScaledSpread` era-cost
  annex; D8 anchors 119/119 + FROZEN gate + existing pins byte-immutable by
  construction. No anchor-additive re-emission owed; the 9 anchors.toml SHAs
  untouched.
