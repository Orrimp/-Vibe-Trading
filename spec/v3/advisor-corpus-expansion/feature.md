---
slug: advisor-corpus-expansion
status: shipped
owner: operator
updated: 2026-07-10
version: 3.3.1
trace: REQ-V3-P2-CORPUS-EXPANSION-001
---

# P2 — Data corpus expansion + ship-passive verdict re-run

## Why

The product's terminal thesis — **"no active strategy robustly beats
buy-and-hold net of costs"** (2026-06-08 ship-passive verdict, `product.md`) —
currently rests on a **narrow evidence base**:

- **One venue:** Binance only.
- **One bar size:** hourly (1h).
- **~2 regimes:** the pinned 2023-24 corpus (`data/binance/`, aggregate SHA
  `3a8b96c4…`) + the 2021-22 bear corpus (`data/binance-2122/`, SHA
  `4f390622…`). Plus the broad-universe 35-symbol 2023-24 corpus
  (`data/binance-broaduni/`) for cross-sectional width.

That is **solid within corpus** but **modest for crypto's regime diversity**.
And the product's own overfitting scorecard says so out loud: `MinBTL ≈ 6.4
years` at `SR_target=1, N_eff=24` (`bakeoff/scorecard.rs:46`) — i.e. the honest
minimum backtest length to *trust* a crown exceeds the ~2 years of Binance
hourly the current pinned corpora carry per regime. Short records prove little;
that is precisely the MinBTL lesson the scorecard already prints.

**P2 extends the evidence base and re-runs the verdict.** Two outcomes, both
product value:

1. **Ship-passive survives** the wider regime + venue coverage → a *stronger*
   claim, and `MinBTL` before/after improves (more years, more independent
   regimes).
2. **It breaks somewhere** (an arm's verdict flips on a regime or venue) →
   *real signal*, surfaced honestly with an explicit wobble list.

The deliverable is **an honest re-run report**, not a data lake. Anchors stay
**119/119 by construction** (`write_report=false` on every re-run path — the
same anchor-safety contract the P2-2 null-CI and the advisor bake-off already
honor).

> **Framing (LOAD-BEARING, per the ship-passive precedent):** the goal is to
> *test* whether the verdict holds across regimes/venues, NEVER to manufacture
> a flip. A null result ("ship-passive holds on every new corpus and venue,
> `MinBTL` improves") is the **expected, valid, shippable** outcome. The gate
> decides, not the author. **The FROZEN gate stays byte-frozen throughout**
> (`classify_verdict` / `verdict_bands` / `compute_robustness_flag` untouched);
> this feature adds corpora + a report, NOT a band change.

## Availability reality-check (analyst investigation — code + API grounded)

### The existing fetcher: what it actually supports

`crates/data/src/bin/fetch_binance_klines.rs` (+ the extracted library
`crates/data/src/binance_klines.rs`) is the 10-symbol Binance hourly pipeline:

- CLI: `--symbols … --start YYYY-MM-DD --end YYYY-MM-DD --interval {1m,5m,15m,1h,4h,1d} --out <dir> --emit-revision-manifest`.
- Output layout: `<out>/<SYMBOL>/<YEAR>/<MONTH-padded>.parquet`.
- **Idempotent for gapped months** (`should_skip`, 2026-06-16): a legitimately-short
  month whose content-SHA matches the pinned `REVISION.toml` is skipped without
  re-fetching. Re-running over a pinned corpus is byte-stable.
- Pagination: `paginate_klines`, 1000-candle pages, 200 ms between requests
  (≤300 req/min, well under Binance's limit). **`start`/`end` window pages
  BACKWARD to full listing history** — this is the deep-history property P2
  needs, and it already works.
- **`interval` is a free CLI arg** → the same binary fetches `1d` (daily) with
  zero code change (only the `expected_bars_per_month` bar-count-verify path is
  1h/4h/… aware; `1d` months skip conservatively — a known, benign behaviour).

`fetch_yahoo_klines.rs` is the **lab / macro** path (`data/yahoo-macro/`
already carries DXY / GSPC / TNX daily 2021→2026). `fetch_deribit_dvol.rs`
fetches the DVOL implied-vol index (BTC/ETH only; history reaches 2021-04).

### Binance earliest-history per symbol (the current 10)

Binance spot launched **2017-07-14**; BTCUSDT klines start **2017-08** (first
full month). The 10-symbol universe has **very uneven listing dates** — this is
the single most important corpus-design constraint:

| Symbol   | Binance spot listing (approx) | 2017-18 | 2020 COVID | Note |
|----------|-------------------------------|:-------:|:----------:|------|
| BTCUSDT  | 2017-08                       | ✅ full | ✅         | deepest history |
| ETHUSDT  | 2017-08                       | ✅ full | ✅         | deepest history |
| BNBUSDT  | 2017-11                       | ✅ part | ✅         | Binance's own token |
| XRPUSDT  | 2018-05                       | ⚠️ part | ✅         | mid-2018 on |
| ADAUSDT  | 2018-04                       | ⚠️ part | ✅         | mid-2018 on |
| LINKUSDT | 2019-01                       | ❌      | ✅         | 2019 on |
| DOGEUSDT | 2019-07                       | ❌      | ✅         | 2019 on |
| DOTUSDT  | 2020-08                       | ❌      | ⚠️ late    | post-COVID-crash |
| SOLUSDT  | 2020-08                       | ❌      | ⚠️ late    | post-COVID-crash |
| AVAXUSDT | 2020-09-22                    | ❌      | ❌         | post-COVID-crash |

**Consequence:** the deep-history corpora cannot be uniform-10-symbol. The 2017-18
mania/crash corpus is honestly **BTC+ETH(+BNB) only**; a full-10 fetch there
returns empty months for 7 symbols (the fetcher warns + skips — no crash, but
the coverage is a lie if presented as "10 coins"). See Requirement R1 for the
per-corpus symbol-subset design.

### Kraken deep-hourly: the honest verdict — NOT fetchable via the REST OHLC endpoint

**This is the load-bearing availability finding.** The prompt names Kraken as the
second reconcilable venue (correct per the venue-trust map — Kraken reconciles
on the `|ΔOI| ≤ volume` identity, cleaner than HTX). BUT:

- **Kraken's REST `OHLC` endpoint returns only the most-recent ~720 candles
  TOTAL**, and the `since` parameter **cannot page backward** for deep history —
  it advances the *forward* cursor only. On 1h that is ~30 days; in practice
  users report ~5 days reachable (freqtrade issue #2134). **Deep hourly history
  is structurally unreachable through this endpoint.**
- The documented workarounds are: (a) reconstruct OHLC from the **Trades**
  endpoint (multi-GB of tick data per symbol, a large new aggregation pipeline —
  out of scope for a bounded cross-check), or (b) Kraken's **free downloadable
  historical OHLCVT CSV dump** (full history, 1/5/15/30/60/240/720/1440-min
  intervals) — but this is a **manual ZIP download + CSV path, NOT the existing
  REST→parquet fetcher**; wiring it is a new adapter.

**Honest alternatives proposed (architect decides via Q-CE-2):**

1. **(Recommended) Coinbase Exchange as the second venue, hourly, via
   pagination.** Coinbase's `get-product-candles` endpoint has a 300-candle
   per-call cap BUT `start`/`end` **page backward to full listing history** —
   the *identical* windowed-pagination pattern the Binance fetcher already uses.
   Coinbase is on the venue-trust HIGH-spot-price tier (Binance / Coinbase /
   Kraken) — the shipped `VenueTrust::HighReconcilable` doc names all three, so
   Coinbase is already a blessed reconcilable cross-check venue. `BTC-USD`
   hourly reaches back to ~2015-2016 (deeper than Binance). A new
   `fetch_coinbase_klines` bin mirrors the Binance one (~1 dev-day; same parquet
   schema + REVISION.toml).
2. **(Fallback) Kraken DAILY (`1440`) cross-check only.** The REST OHLC 720-cap
   gives ~720 daily candles ≈ 2 years per call and pages forward from a `since`
   near listing — daily deep-history IS reachable (multiple forward pages).
   Honest but coarser (daily bars, not hourly); still tests venue-dependence of
   the verdict on the same date windows.
3. **(Heaviest, likely OUT) Kraken hourly via the CSV dump** — full fidelity but
   a new CSV-ingest adapter; disproportionate for a bounded cross-check.

**Analyst lean:** Coinbase-hourly (option 1) is the durable choice — it gives a
true apples-to-apples *hourly* venue cross-check on the same windows as the
Binance corpora, reuses the existing paginate-window pattern, and Coinbase is
already trust-blessed in the shipped panel. Kraken-daily (option 2) is the
if-budget-tightens fallback. Do NOT build the Kraken CSV pipeline in P2.

## Requirements

### R1 — New pinned Binance corpora (additive; existing SHAs untouched)

Fetch these as **NEW pinned corpora** (new `--out` dirs + new `REVISION.toml`
each), mirroring the `data/binance-2122/` convention exactly (fetch command in
a non-anchored `reports/fetch-*.md`, per-symbol bar totals, aggregate SHA, the
"must stay" existing SHAs `3a8b96c4…` + `4f390622…` recorded). **Bounded — regime
+ venue coverage, not a data lake.** Proposed set (architect confirms windows +
symbol subsets via Q-CE-1):

| Corpus dir (proposed)     | Window      | Symbols (honest subset)          | Regime captured |
|---------------------------|-------------|----------------------------------|-----------------|
| `data/binance-1718`       | 2017-08 → 2018-12 | BTCUSDT, ETHUSDT, BNBUSDT (the only pre-2019 listers with ≥full-year coverage) | 2017 mania blow-off + 2018 bear |
| `data/binance-2020`       | 2020-01 → 2020-12 | BTC, ETH, BNB, XRP, ADA, LINK, DOGE (the 7 pre-2020 listers; DOT/SOL/AVAX listed mid/late-2020 → ragged, excluded) | COVID crash (Mar-2020) + recovery |
| `data/binance-2526`       | 2025-01 → 2026-06 | all 10 (all listed by 2020) | recent 2025-26 regime |

Rationale for the subsets: including a symbol whose listing post-dates a
corpus's start yields empty/ragged early months (the fetcher warns + skips — no
crash — but presenting it as "N coins" would be dishonest). Each corpus carries
**only symbols with contiguous full coverage across its window**. The 2020
corpus deliberately holds DOT/SOL/AVAX OUT (they listed Aug/Sep-2020 → would be
half-empty). BTC+ETH are in every corpus (the only two with 2017→now contiguous).

> **`data/binance-2526` end-date caveat:** "2026-06" is the intended end but the
> fetch runs in the *present* — the developer clamps `--end` to the last
> **fully-closed** UTC month at fetch time so no partial trailing month enters
> the pin (a partial month is a legitimately-short month, but pinning a
> still-growing month breaks idempotent re-fetch). Architect confirms the clamp
> rule (Q-CE-6).

### R2 — Second-venue cross-check corpus (venue-dependence test)

Per Q-CE-2's resolution, ONE cross-check corpus:

- **(Recommended path)** `data/coinbase/` — BTC-USD hourly, longest window that
  overlaps ≥2 Binance corpora (proposal: 2020-01 → 2026-06, the deepest window
  where a *direct* Binance-vs-Coinbase hourly comparison is possible on the same
  symbol). New `fetch_coinbase_klines` bin (mirror of the Binance bin: same
  parquet schema, `--emit-revision-manifest`, idempotent skip). Cross-check
  scope = **BTC only** (the price-discovery-leader pair; the venue-dependence
  question is "does the verdict change on a different venue's *same-asset*
  price series", not "re-run the whole universe on Coinbase").
- **(Fallback path)** `data/kraken-daily/` — BTC-USD daily, via the REST OHLC
  forward-paged `since`; coarser but reachable.

**Reconciliation requirement:** the report MUST state the Binance-vs-second-venue
**price agreement** on the overlap window (e.g. median absolute hourly-close %
deviation) — the venue-trust map's claim is that cross-venue deviations stay
inside fee-defined no-arb bands and mean-revert fast; the cross-check *verifies*
that on our own data before trusting the second-venue verdict.

### R3 — Verdict re-run (the bake-off + null-CI on the extended corpora)

Re-run, per corpus, `write_report=false` throughout (anchors 119/119 by
construction — run `scripts/verify_anchors.sh` before AND after):

1. **Full bake-off** (`run_bakeoff` / `run_field_and_rank`) — all arms the
   corpus *supports* (see R4 honest-subset matrix), incl. ensembles + shorts
   where data supports, judged by the FROZEN `RobustnessMode::Bootstrap` gate +
   the buy-and-hold benchmark.
2. **The P2-2 null-falsification** (`compute_scorecard` DSR second layer)
   applied to each new-corpus crown: every corpus's crowned arm (if `ActiveWins`)
   must fail `crown_clears_dsr`, exactly as `null_data_no_crown.rs` asserts on
   synthetic nulls. On REAL corpora this becomes: *if any arm crowns on a new
   regime/venue, does DSR certify it?* — the honest credibility check.
3. **`MinBTL` before/after** — quantify how the additional years + independent
   regimes move the honest minimum-backtest-length bar (`scorecard.min_btl_years`
   is already computed per run; the report tabulates it across corpora and
   states the aggregate improvement).

**This feature adds NO backtest math and NO new arm.** It adds corpora + a
harness that runs the *existing* pipeline over them + the report. If the
architect finds the re-run needs a small runner/CLI seam (e.g. a
`--corpus <dir>` selector on an existing bench/bin), that is in scope; a new
strategy, overlay, or gate change is NOT.

### R4 — Honest per-corpus arm-availability matrix (define, don't silently drop)

Some arms need exogenous data that does not exist for every corpus. The report
MUST carry an explicit **arm × corpus availability matrix** and run the honest
subset per corpus rather than silently dropping arms:

| Arm class | Data need | 1718 | 2020 | 2122 | 2324 (base) | 2526 | Coinbase-xchk |
|-----------|-----------|:----:|:----:|:----:|:-----------:|:----:|:-------------:|
| Price-only singles (SMA/MACD/RSI/Bollinger + Donchian/vol_breakout/ROC where shipped) | OHLCV only | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ (BTC) |
| Pre-registered vote-ensembles | OHLCV only | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ (BTC) |
| Short / long-short arms (`_ls`, `always_short`) | OHLCV + funding const | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ (BTC) |
| `v0.dvol_regime` | Deribit DVOL, BTC/ETH only, ≥2021-03 | ❌ (no DVOL) | ❌ (no DVOL) | ⚠️ needs fetch 2021-04→ | ✅ (on disk 2023-24) | ⚠️ needs fetch | ❌ (BTC-USD ≠ DVOL symbol path) |
| `v0.macro_riskon` | yahoo-macro DXY/GSPC/TNX daily | ❌ (macro starts 2021) | ❌ (macro starts 2021) | ✅ (2021-22 on disk) | ✅ (on disk) | ⚠️ needs fetch 2025-26 | ❌ (venue-xchk is price-only) |
| Perp-basis / funding MN arms | binance-basis + funding | ❌ | ❌ | ❌ (basis is 2023-24 only) | ✅ (on disk) | ❌ | ❌ |

Legend: ✅ runs as-is · ⚠️ runs only if the developer additionally fetches the
exogenous corpus for that window (architect scopes whether to; DVOL/macro
back-fills are cheap, additive, and reuse the existing idempotent fetchers) ·
❌ arm is legitimately ABSENT for that corpus (the graceful-degradation path
`run_bakeoff` already implements — DVOL/macro arms warn + run warm-up-only =
buy-and-hold proxy, never crash; ADR-0072 / ADR-0073). The report frames an
absent arm as "not evaluable on this corpus (no <data>)", NOT as a silent drop
or a pass.

> **Exogenous back-fill decision (Q-CE-3):** DVOL (BTC/ETH, 2021-04→) and macro
> (2025-26) are cheap idempotent additive fetches that would let the DVOL/macro
> arms run on the 2122 + 2526 corpora. Analyst lean: **fetch them** (the whole
> point of P2 is wider evidence; a warm-up-only proxy arm is not a real test of
> the DVOL/macro thesis). Bounded: DVOL 2021-2022 + 2025-2026 for BTC/ETH; macro
> 2025-2026 for the 3 tickers. Existing pinned DVOL/macro SHAs are NOT mutated —
> new years are new files (the DVOL fetcher writes per-year, macro per-month;
> both additive).

### R5 — Survivorship honesty (extending to 2017 makes it MORE acute)

The DATA-quality panel already carries an always-present survival note
(`leaderboard/state.rs:174-180`, `strings::LEADERBOARD_DATA_QUALITY_SURVIVAL_NOTE`):
*"coins that failed to reach today are absent from this universe; results
overstate the expected outcome for a randomly chosen NEW coin."* Extending back
to 2017 makes this **worse**, and the report MUST say so explicitly:

- The 2017-18 corpus's BTC/ETH/BNB are the **survivors of the survivors** — the
  2017-18 top-10-by-cap universe was full of coins that no longer exist
  (BCC/BCH forks, dozens of ICO tokens, several that were top-10 in 2018 and are
  now near-zero). A "these 3 coins over 2017-18" backtest is an **extreme**
  survivorship selection; the report frames any 2017-18 result as *"conditioned
  on the three largest survivors — the most favourable possible slice"*, not as
  "how a 2017 coin pick would have done".
- The 2020 corpus's 7 symbols similarly exclude every 2020 coin that later died.
- **No new code needed** — the shipped survival note already fires for every
  bake-off. R5 is a *report-framing* requirement: the re-run report's
  survivorship section states the per-corpus selection severity in words, and
  (analyst lean, Q-CE-4) the earliest corpora carry a STRONGER worded caveat
  than the default note (an explicit "survivor-of-survivors" line for 1718).

### R6 — Cost realism across eras (register E-2 caveat; do NOT change the default)

The cost model default is a flat-bps effective spread (`SlippageModel`,
`base_bps=8`; the P1-6 `VolScaledSpread` widens 2-3× in stress but stays
opt-in). **2017-era spreads/fees differ materially from 2024** — early-Binance
spreads were wider and more volatile, maker/taker fee schedules differed, and
liquidity was thinner. The flat-bps default is **less honest for 2017-20 than
for 2023-24**.

- **Do NOT change the default** (operator-standing: cost-model bands are frozen;
  changing them mid-verdict-re-run would confound the regime comparison).
- **Register E-2:** the re-run report MUST carry an **era-cost caveat**: "the
  flat 8 bps effective-spread default is calibrated to modern deep-liquidity
  major-venue conditions; on the 2017-20 corpora true costs were plausibly
  higher and more variable, so an active arm's *net* edge on those eras is if
  anything OVER-stated — i.e. a Fragile/BenchmarkWins verdict there is
  conservative-correct, and any *rare* active-win there should be read against
  an optimistic cost assumption." This strengthens (never weakens) a
  ship-passive conclusion; it only cautions against over-reading a flip on an
  old-era corpus.
- Q-CE-5 (architect + operator): whether to ALSO run each old-era corpus once
  under the opt-in `VolScaledSpread` as a robustness sensitivity (analyst lean:
  yes as a report annex — it is opt-in, anchor-safe, and directly quantifies the
  era-cost sensitivity — but the primary verdict stays on the frozen default for
  cross-regime comparability).

### R7 — Anchor safety + gate freeze (non-negotiable, by construction)

- Every re-run path runs `write_report=false` → NO anchored CLI report body is
  produced → `scripts/verify_anchors.sh` stays **119/119**. Run it before AND
  after every commit (anchors keyed by NAME not filename — a link/typo fix in a
  `reports/` file would break the body-SHA gate; this feature writes only
  NON-anchored `reports/fetch-*.md` and `reports/backtest-*.md`).
- The FROZEN gate (`classify_verdict` / `verdict_bands` /
  `compute_robustness_flag` / the ADR-0066 benchmark exemption) is **byte-untouched**.
  This is NOT a band proposal. BenchmarkWins/AllFragile reachability UNCHANGED.
- Existing pinned corpora SHAs (`3a8b96c4…` `data/binance/`, `4f390622…`
  `data/binance-2122/`, `data/binance-broaduni/`, DVOL, macro, basis, funding)
  are **byte-immutable**; P2 adds new `--out` dirs only. `ci.yml.deferred` stays
  parked.

### R8 — Acceptance criteria (the tester's contract for the re-run report)

The re-run report (a NEW non-anchored `reports/backtest-<date>-p2-verdict-rerun.md`)
PASSES iff it carries all of:

- **AC1** — a **per-corpus × per-arm verdict table**: for each new corpus, every
  *supported* arm's `RobustnessFlag` + `RecommendationOutcome` + Sharpe vs
  buy-and-hold. Absent arms shown as "not evaluable (no <data>)", never blank.
- **AC2** — **null-CI results on the extended corpora**: for every corpus whose
  crown is `ActiveWins`, the `crown_clears_dsr` verdict (must be `false` for a
  credible ship-passive claim; a `true` on a real corpus is the honest signal to
  surface loudly, mirroring the `null_data_no_crown.rs` falsification condition).
- **AC3** — **`MinBTL` before/after**: the aggregate `min_btl_years` on the
  old (2-regime) evidence base vs the extended base, with the improvement stated
  in years, and a plain-language read of whether the extended base now *meets*
  the honest MinBTL bar.
- **AC4** — **explicit wobble list**: any arm whose verdict FLIPS across
  corpora/venues (e.g. Fragile on 2324 but Robust on 2020, or a
  BenchmarkWins→ActiveWins flip on Coinbase-vs-Binance), named with the corpus
  pair and the flip direction. An empty wobble list ("no arm's verdict flipped;
  ship-passive holds uniformly") is a valid, strong result.
- **AC5** — **venue reconciliation stat** (R2): the Binance-vs-second-venue BTC
  price agreement on the overlap window.
- **AC6** — **survivorship + era-cost caveats stated** (R5 + R6): the
  survivor-of-survivors framing per old-era corpus + the E-2 era-cost caveat, in
  words, in the report.
- **AC7** — **anchors 119/119 + spec-lint PASS(0)** verified in the report's
  gate section; new corpora `REVISION.toml` internal-consistency test green
  (mirror `binance_2122_revision_consistency`); a SKIP-safe smoke read per new
  corpus (returns early on machines without the gitignored parquets).
- **AC8** — the honest **top-line verdict sentence**: one line stating whether
  ship-passive HOLDS across all new regimes/venues, WOBBLES (with the pointer to
  AC4), or BREAKS — chosen by the data, not pre-written.

## Design

**Design lock: [ADR-0084](../../architecture/adr/0084-p2-corpus-set-coinbase-adapter-verdict-rerun.md)**
(P2 corpus set + Coinbase second-venue adapter + multi-corpus verdict-rerun
harness). This section is the operator-readable summary; ADR-0084 D1–D8 are the
binding record. No gate change, no anchor-additive re-emission, no single-coin
engine clamp change.

### Q-CE-1..7 decisions (one line each)

- **Q-CE-1 (corpus windows + subsets) — RATIFY unchanged (ADR-0084 D1).** The
  exact analyst 3-corpus set: `data/binance-1718` (2017-08→2018-12, BTC/ETH/BNB),
  `data/binance-2020` (2020, the 7 pre-2020 listers), `data/binance-2526`
  (2025-01→last-closed-month, all 10). Bounded; honest full-coverage subsets; no
  add/trim.
- **Q-CE-2 (second venue — THE key decision) — COINBASE-HOURLY, RATIFIED (ADR-0084
  D2).** Kraken REST OHLC hourly deep-history is INFEASIBLE (720-candle total cap,
  no backward `since` paging). Coinbase gives a true apples-to-apples *hourly*
  cross-check on the same windows, reuses the windowed-pagination pattern, and is
  already `VenueTrust::HighReconcilable`. **Correction to A3:** the shipped
  `coinbase.rs` is a live-WS feed and CANNOT backfill → a NEW `fetch_coinbase_klines`
  bin + `coinbase_klines.rs` lib are required. Kraken-daily fallback + Kraken-CSV
  are OUT of P2.
- **Q-CE-3 (exogenous back-fill) — FETCH, bounded (ADR-0084 D3).** DVOL
  (2021-22 + 2025-26, BTC/ETH) + macro (2025-26, DXY/GSPC/TNX), additive per-year/
  per-month, existing pinned SHAs byte-identical → `v0.dvol_regime` / `v0.macro_riskon`
  genuinely evaluable on 2122 + 2526. Perp-basis/funding NOT back-filled (stay
  legitimately absent).
- **Q-CE-4 (survivorship caveat) — STRONGER prose, no code (ADR-0084 D6).** The
  1718/2020 corpora carry an explicit survivor-of-survivors caveat in the re-run
  report; the shipped survival note already fires for every bake-off.
- **Q-CE-5 (era-cost annex) — YES, opt-in annex (ADR-0084 D7).** Register E-2;
  primary verdict on the frozen flat-8-bps default (cross-regime comparability);
  1718 + 2020 also re-run once under the opt-in `VolScaledSpread` (ADR-0081) as a
  supplementary sensitivity annex.
- **Q-CE-6 (2526 end-clamp) — CONFIRMED (ADR-0084 D5).** Developer clamps
  `binance-2526` (and `data/coinbase`) `--end` to the last fully-closed UTC month
  at fetch time; the exact end month is recorded in the fetch report.
- **Q-CE-7 (re-run harness seam) — DEDICATED `p2_verdict_rerun` harness (ADR-0084
  D4).** NOT a `--corpus` selector: `run_bakeoff` hardcodes `data/binance` in the
  `pub(crate)` `preload_bakeoff_binance_bars`, so the harness composes two proven
  pieces — arbitrary-corpus `ReplayFeed::new(<root>).merge_symbols` (per
  `realdata_simple_strategy_bear_survey.rs:168`) + `null_data_no_crown.rs::run_field_and_rank`'s
  exact per-arm sequence — with ZERO shipped-runner change and `write_report=false`.

### Designed seams

**Seam 1 — the Coinbase fetcher (ADR-0084 D2.a).** New
`crates/data/src/coinbase_klines.rs` (library) + `crates/data/src/bin/fetch_coinbase_klines.rs`
(CLI glue), a direct mirror of `binance_klines.rs` + `fetch_binance_klines.rs`.
The **one real seam** is the venue shim, confined to `coinbase_klines.rs`:

| Concern            | Binance                                  | Coinbase (the shim)                                          |
|--------------------|------------------------------------------|-------------------------------------------------------------|
| On-disk symbol dir | `BTCUSDT` (canonical `Symbol`)           | **`BTCUSDT`** (normalized) — REST call uses `coinbase_symbol_map(&sym)`→`BTC-USD` |
| Endpoint           | `/api/v3/klines?…&limit=1000`            | `/products/{product-id}/candles?start=<ISO8601>&end=<ISO8601>&granularity=3600` |
| Page size          | 1000 candles                             | **300** candles (>300 → rejected)                           |
| Candle order       | `[open_time,open,high,low,close,vol,close_time,…]` | `[time,low,high,open,close,volume]` — map positionally into the shared `Kline` |
| Timestamp unit     | millis                                   | **seconds** → ×1000; `close_time = open_time + granularity_ms − 1` |
| `trade_count`      | real                                     | absent → `0` (the `coinbase.rs:299` sentinel)               |
| Pace               | 200 ms                                   | ≥200 ms (Coinbase ~10 req/s public → 5 req/s safe)          |

Everything else is reused verbatim: the shared `binance_klines::Kline` struct,
`write_parquet` (same 8-col `replay_feed.rs` schema), `should_skip` content-SHA
idempotency, `data::revision::write_revision_manifest`, `expected_bars_per_month`,
`--emit-revision-manifest`. A new `paginate_coinbase_candles` mirrors
`paginate_klines` with the 300-window step + forward sub-windows within each month
(iterating months back to listing IS the deep-history paging). A new
`CoinbaseKlineFetcher` trait + `HttpCoinbaseKlineFetcher` + a mock mirror the
Binance testability seam (no socket in unit tests).

**Seam 2 — corpus pinning (ADR-0084 D1 + D2.b + D5).** Four new `--out` dirs, each
with its own `REVISION.toml` via `--emit-revision-manifest`, mirroring the
`data/binance-2122/` convention exactly (fetch command recorded in a non-anchored
`reports/fetch-*.md`; per-symbol bar totals; aggregate SHA; the "must stay"
existing SHAs `3a8b96c4…` + `4f390622…` recorded). Existing pins byte-immutable —
P2 adds new dirs only. The DVOL/macro back-fills (D3) are additive per-year/
per-month writes into the EXISTING `data/deribit-dvol/` + `data/yahoo-macro/` roots;
their existing pinned file SHAs are byte-identical (new years/months are new files).

**Seam 3 — the re-run harness (ADR-0084 D4).** `crates/backtest/tests/p2_verdict_rerun.rs`.
For each `(corpus_root, symbol(s), supported_arm_field)` from the R4 matrix:
load bars via `ReplayFeed::new(corpus_root, true).merge_symbols(…)`; run the
null-CI's `run_field_and_rank` verbatim (every fn = the production fn `run_bakeoff`
calls; `write_report=false`); collect `FieldOutcome` (`ranking` + `scorecard` +
`candidates`). DVOL/macro arms thread `dvol_override` / `macro_regime_series` via
the SAME public `resolve_dvol_override` / `load_macro_regime_series` fns, pointed
at the back-filled corpora. Absent arms (R4 ❌) are **not added to that corpus's
field** — reported as "not evaluable (no <data>)", never a silent drop, never a
warm-up-only proxy masquerading as an evaluation. SKIP-safe per corpus. The harness
emits the AC1-AC8 report data (in-memory `FieldOutcome`s → the tester's report).

**Seam 4 — the report contract (ADR-0084 D8; R8 AC1-AC8).** A NEW, NON-anchored
`spec/v3/advisor-corpus-expansion/reports/backtest-<date>-p2-verdict-rerun.md`,
tester-authored, carrying: AC1 per-corpus×per-arm verdict table
(`RobustnessFlag` + `RecommendationOutcome` + Sharpe-vs-B&H; absent arms shown, never
blank); AC2 null-CI `crown_clears_dsr` per `ActiveWins` crown (must be `false` for a
credible ship-passive claim; a `true` on a real corpus is the honest signal to
surface loudly, mirroring `null_data_no_crown.rs`); AC3 `MinBTL` before/after
(`scorecard.min_btl_years` aggregated, improvement in years); AC4 explicit wobble
list (any verdict flip across corpora/venues; empty = valid strong result); AC5
Binance-vs-Coinbase BTC price agreement on the overlap window; AC6 survivorship +
E-2 era-cost caveats in words; AC7 gate section (anchors 119/119 + spec-lint
PASS(0) + new-corpus REVISION consistency test + SKIP-safe smoke); AC8 the top-line
HOLDS/WOBBLES/BREAKS sentence chosen by the data.

### What this feature deliberately does NOT do

No new backtest math, no new arm, no gate/band change (`bakeoff/{robustness,rank}.rs`,
`classify_verdict`, `verdict_bands`, the ADR-0066 benchmark all byte-untouched — NOT
a band proposal). No shipped-runner change. No live trading (the €200 stays
SIMULATED). No Kraken adapter. No full-universe Coinbase re-run (BTC-only, bounded).
No reconstruction of delisted coins (survivorship handled by prose framing).

### ADR

**ADR-0084 authored + registered atomically** (README `## Registry` row +
frontmatter `updated:` in the same edit pass; `scripts/adr_registry_check.py`
green in both `--pre-commit` and bare mode). No anchor-additive re-emission owed;
the 9 `spec/anchors.toml` SHAs are untouched.

## Backtest Scenarios

The re-run IS the scenario set. All runs `write_report=false` (anchors 119/119 by
construction); the FROZEN `RobustnessMode::Bootstrap` gate + the buy-and-hold
benchmark judge every arm. Determinism: each corpus's field uses a fixed seed base
(the null-CI `run_field_and_rank(seed_u64)` contract; `ChaCha20Rng`), recorded in
the report.

**Primary matrix (frozen flat-8-bps cost default) — the R4 honest arm×corpus
availability drives the field per corpus:**

| Scenario | Corpus | Symbols | Supported arms (R4) |
|----------|--------|---------|---------------------|
| S1 | `data/binance-1718` | BTC/ETH/BNB | price-only singles + vote-ensembles + short/`_ls` (NO DVOL/macro/basis) |
| S2 | `data/binance-2020` | the 7 pre-2020 listers | price-only singles + vote-ensembles + short/`_ls` (NO DVOL/macro/basis) |
| S3 | `data/binance-2122` | 10 | singles + ensembles + short/`_ls` + **DVOL** (back-filled 2021-22) + **macro** (on disk) — NO basis |
| S4 | `data/binance` (2324 base) | 10 | full field (singles + ensembles + short/`_ls` + DVOL + macro + basis) — the reference/baseline |
| S5 | `data/binance-2526` | 10 | singles + ensembles + short/`_ls` + **DVOL** (back-filled 2025-26) + **macro** (back-filled 2025-26) — NO basis |
| S6 | `data/coinbase` | BTC only | price-only singles + vote-ensembles + short/`_ls` (venue cross-check; DVOL/macro N/A) |

**Sensitivity annex (opt-in `VolScaledSpread`, ADR-0081 — E-2 quantification):**

| Scenario | Corpus | Note |
|----------|--------|------|
| S7 | `data/binance-1718` | same field as S1, re-run once under `VolScaledSpread`; report Δverdict vs S1 |
| S8 | `data/binance-2020` | same field as S2, re-run once under `VolScaledSpread`; report Δverdict vs S2 |

**Per-scenario assertions (mirroring `null_data_no_crown.rs`'s two-layer contract,
applied to REAL corpora):** for every corpus whose crown is `ActiveWins`, record
`scorecard.crown_clears_dsr` (AC2 — a `true` on a real corpus is the honest
loud-surface signal, not an auto-fail). Aggregate `scorecard.min_btl_years` across
the old 2-regime base {2122, 2324} vs the extended base {1718, 2020, 2122, 2324,
2526} (AC3). Cross-corpus verdict-flip detection → the wobble list (AC4). S6 also
computes the Binance-vs-Coinbase BTC median-absolute hourly-close % deviation on
the overlap window (AC5). Empty wobble list + `crown_clears_dsr==false` everywhere
+ `MinBTL` improved = "ship-passive holds; stronger claim" (a valid, expected,
shippable top-line, AC8).

## Implementation

**Developer 2026-07-10 (T1-T3, T6-investigated, T9, T10 — see `tasks.md` for
file:line + test-command + output-line citations per row).** Scope was T1-T3
(Coinbase fetcher) + T2 dry-run + T6 (exogenous back-fill) + T9 (verdict-rerun
harness) + T10 (gates); T4 (the 4 multi-hour corpus fetches) and T7/T8 (which
are blocked on T4's manifests) are explicitly the orchestrator's / tester's
scope and are NOT ticked.

**Files:**
- `crates/data/src/coinbase_klines.rs` (new lib) — the Coinbase Exchange REST
  backfiller: `coinbase_product_id_for_symbol` (symbol→product-id shim),
  `parse_coinbase_candle` (the four D2.a mappings), `paginate_coinbase_candles`
  (300-candle forward-sub-window pager), `CoinbaseKlineFetcher` trait +
  `HttpCoinbaseKlineFetcher` (with the discovered-required `User-Agent` header)
  + mock, reuses `binance_klines::{Kline, write_parquet}` verbatim.
- `crates/data/src/bin/fetch_coinbase_klines.rs` (new bin) — CLI mirror of
  `fetch_binance_klines.rs`: `--symbols` (canonical `BTCUSDT`), `--start`,
  `--end`, `--interval` (only `1h` wired), `--out`, `--force`,
  `--emit-revision-manifest`, plus `[EARLIEST-SERVED]` reporting for A2.
- `crates/data/src/lib.rs` — module registration + re-export.
- `crates/backtest/src/dvol_data.rs` — `EXPECTED_DVOL_REVISION_SHA` re-pinned
  after the T6 DVOL back-fill (2021/2022/2025/2026 added, 4→12 files).
- `crates/backtest/tests/p2_verdict_rerun.rs` (new harness) — composes
  `ReplayFeed::merge_symbols` arbitrary-corpus loading with a generalized
  `run_field_and_rank` reproducing `run_bakeoff`'s exact per-arm sequence;
  `build_field(ArmSupport)` encodes the R4 matrix; S1-S8 scenario fns (S4
  un-ignored as the always-on smoke against the existing `data/binance`
  corpus; S1/S2/S3/S5/S6/S7/S8 SKIP-safe on absence).
- `crates/backtest/Cargo.toml` — `[[test]] p2_verdict_rerun` with
  `required-features = ["realdata", "yahoo"]`.

**Two real bugs found + fixed during testing (not design reversals — the
DELIVERED behaviour matches ADR-0084's design intent in both cases):**

1. **Symbol mapping.** ADR-0084 D2.a named the reused
   `crate::coinbase::coinbase_symbol_map` for `BTCUSDT → BTC-USD`. That
   shipped helper checks `USDC → USDT → USD` suffixes in that order, so
   `coinbase_symbol_map(&Symbol::new("BTCUSDT"))` actually returns
   `"BTC-USDT"` — a real, thinner, non-blessed Coinbase product — not
   `"BTC-USD"`. The ADR's own worked example used a DIFFERENT input
   (`BTCUSD`, where the helper IS correct). Fixed via a new dedicated
   `coinbase_product_id_for_symbol` (strip-then-fixed-append `-USD`), which
   delivers the ADR's actual design intent.
2. **Missing `User-Agent` header.** Discovered live during the T2 dry-run:
   Coinbase Exchange rejects requests without one (`HTTP 400`); `reqwest`'s
   default client sets none. Fixed with a fixed `User-Agent` constant on
   every request.

**T2 dry-run + A2 result:** a live 3-day BTC-USD hourly fetch (2024-01-01 →
2024-01-03) succeeded post-fix (72 candles; `ReplayFeed` read all 72 with sane
BTC prices $42.4k-$42.9k). The A2 earliest-served-candle probe (bounded, ~10
small live calls bisecting 2015-2018) found BTC-USD hourly is served starting
**2015-08** — deeper than the ADR's "~2015-16" estimate; **A2 confirmed, no
window narrowing needed** for the proposed 2020-01→last-closed-month
`data/coinbase` window.

**T6 findings:** DVOL genuinely needed the back-fill (2021-22 + 2025-26;
history starts ~2021-04, so 2021 is legitimately short) and received it live
— `data/deribit-dvol/` grew from 4 to 12 parquet files, the 4 pre-existing
files verified byte-identical via `shasum -a 256 -c` before/after. **Macro is
a NO-OP on this machine** — `data/yahoo-macro/` already covers 2021-01 through
2026-06 for all 3 tickers; no fetch was run for zero new data.
`crates/backtest/src/dvol_data.rs::EXPECTED_DVOL_REVISION_SHA` (a hard-pinned
constant over the WHOLE manifest aggregate, distinct from `spec/anchors.toml`'s
9 regression anchors) was updated and proven via the `#[ignore]`d
`real_corpus_load_smoke` test against the real, grown corpus.

**Gates (T10):** `cargo build`, `cargo clippy -- -D warnings`, `cargo fmt
--check` all clean on `-p data -p backtest` (incl. `--tests` and the
`realdata,yahoo` features the new harness needs). `verify_anchors.sh` 119/119
(run before AND after every edit this session). `spec_lint.py` PASS(0).
`adr_registry_check.py` exit 0. FROZEN gate files
(`bakeoff/{robustness,rank,scorecard,mod}.rs`) confirmed byte-untouched via
`git diff --stat`.

**T4/T5 handed off** — see the developer's `HANDOFF →` message for the exact
ready-to-run per-corpus commands, a `watch` probe, and the post-fetch
verification command.

## Verification

**Tester (2026-07-10) — T7, T8, and THE full multi-corpus verdict re-run.**
AC1-AC8 report: [`reports/backtest-2026-07-10-p2-verdict-rerun.md`](reports/backtest-2026-07-10-p2-verdict-rerun.md)
(NEW, non-anchored). Corpus-consistency tests:
`crates/data/tests/p2_corpora_revision_consistency.rs` (T7, 4/4 un-ignored,
un-ignored PASS — `cargo test -p data --test p2_corpora_revision_consistency`)
and `crates/data/tests/p2_corpora_replayfeed_smoke.rs` (T8, 4/4 `--ignored`
PASS — `cargo test -p data --test p2_corpora_replayfeed_smoke -- --ignored
--nocapture`). THE RUN — `cargo test -p backtest --features realdata,yahoo
--test p2_verdict_rerun -- --include-ignored --nocapture` — **15/15 passed,
0 failed, 238.79s (~4.0 min).**

**Headline answer (data-driven, not pre-written):** ship-passive **WOBBLES**
on the older, thinner-liquidity crypto eras (2017-18, 2020, and to a lesser
extent 2021-22 show materially more `ActiveWins` crowns — 19/32 primary
symbol-runs, 16 of which clear the DSR≥0.95 credibility check) but **HOLDS**
on the most recent regime (2025-26, matching the existing 2023-24 baseline —
8/10 `BenchmarkWins`) and on the Coinbase second-venue cross-check relative to
its own Binance-era counterpart (AC5: median price deviation 3-8 bps across
all 4 overlap windows, confirming the two venues track tightly). The era-cost
`VolScaledSpread` annex (S7/S8) explains exactly 1 of 10 tested symbol-runs
(DOGEUSDT/2020) as a pure cost-sensitivity flip; the rest of the older-era
gradient survives that stress-test. `MinBTL` before/after (AC3): the evidence
base grew from 3.99 years (2 regimes, SHORT of the honest 6.36-year bar by
2.36 years) to 7.90 years (5 regimes + 1 venue cross-check, now MEETING the
bar with +1.54 years margin) — the cleanest "stronger claim" result. Full
per-corpus tables, the wobble list, survivorship + era-cost caveats, and
corpus provenance SHAs are in the report; both outcomes (holds/wobbles) are
reported honestly per this feature's own framing — neither was suppressed
nor manufactured.

**Gates:** `scripts/verify_anchors.sh` 119/119 BEFORE and AFTER (the report
is new and non-anchored, no `anchors.toml` edit); `scripts/spec_lint.py`
PASS(0); `cargo fmt --check -p data` + `cargo clippy -p data --tests -- -D
warnings` clean on the two new T7/T8 test files. FROZEN gate
(`bakeoff/{robustness,rank,scorecard,mod}.rs`) byte-untouched this session
(zero edits — the tester only READ these files via the developer's existing
`p2_verdict_rerun.rs` harness).

**VERDICT → PASS.** `HANDOFF → analyst (informational, non-blocking)` on the
wobble-list finding — a follow-on product-copy/research question (should "no
active edge" be scoped to "in today's deep-liquidity market" vs "in any
crypto era, ever"?), not a gate failure.

## Open questions (for the architect M-T1 + operator)

- **Q-CE-1 (corpus windows + symbol subsets)** — confirm the R1 set:
  `binance-1718` (2017-08→2018-12, BTC/ETH/BNB), `binance-2020`
  (2020, the 7 pre-2020 listers), `binance-2526` (2025-01→last-closed-month, all
  10). Any window/subset the architect wants added or trimmed to keep it bounded?
  (Analyst lean: this exact 3-corpus set — it captures mania/crash + COVID +
  recent with honest full-coverage subsets and no data lake.)
- **Q-CE-2 (second venue — THE key decision)** — Coinbase-hourly (Recommended,
  durable: true hourly venue cross-check, reuses the paginate-window pattern,
  Coinbase already trust-blessed) vs Kraken-daily (fallback: coarser but the
  REST OHLC path reaches it) vs Kraken-hourly-via-CSV (heaviest, analyst lean:
  OUT of P2). Kraken REST OHLC hourly deep-history is **confirmed infeasible**
  (720-candle cap, no backward `since` paging).
- **Q-CE-3 (exogenous back-fill scope)** — fetch DVOL (BTC/ETH 2021-22 + 2025-26)
  + macro (2025-26) so the DVOL/macro arms are genuinely evaluable on those
  corpora (analyst lean: yes, bounded + additive), or accept them as
  not-evaluable (warm-up-only proxy) on the corpora that lack them?
- **Q-CE-4 (survivorship caveat strength)** — do the 2017-18 / 2020 corpora carry
  a STRONGER worded "survivor-of-survivors" caveat than the default shipped
  survival note (analyst lean: yes, in the report prose — no code change), or is
  the default note sufficient?
- **Q-CE-5 (era-cost sensitivity annex)** — also run the old-era corpora once
  under the opt-in `VolScaledSpread` as a report annex quantifying era-cost
  sensitivity (analyst lean: yes, annex-only; primary verdict stays on the
  frozen flat-bps default for cross-regime comparability), or default-only + the
  E-2 caveat in words?
- **Q-CE-6 (2526 end-date clamp)** — confirm the developer clamps `--end` to the
  last fully-closed UTC month at fetch time (no partial trailing month in the
  pin). (Analyst lean: yes — pinning a still-growing month breaks idempotent
  re-fetch.)
- **Q-CE-7 (re-run harness seam)** — a `--corpus <dir>` selector added to an
  existing bench/bin, vs a dedicated `p2_verdict_rerun` harness (mirroring
  `null_data_no_crown.rs`'s "reproduce `run_bakeoff`'s exact sequence" pattern
  so GARCH/OU-style corpora-flexibility is possible). Which is the minimal
  honest seam? (Analyst lean: a dedicated harness — it keeps the multi-corpus
  loop + the per-corpus arm-matrix logic out of the shipped runner and mirrors
  the proven null-CI structure.)

## Assumptions (challengeable by architect/developer)

- **A1** — the Binance klines endpoint still serves 2017-08 spot history for
  BTC/ETH (confirmed: `data.binance.vision` monthly archives start 2017-08 for
  BTCUSDT; the REST `startTime` pages back to the same). If a symbol's early
  months return empty, the fetcher warns + skips (no crash) and that symbol is
  simply excluded from that corpus's subset.
- **A2** — Coinbase `get-product-candles` (300/call, backward-pageable) reaches
  BTC-USD to ≥2020 (it in fact reaches ~2015-16); the developer confirms the
  earliest served candle during the fetch and records it in the fetch report.
- **A3** — the existing pipeline's parquet schema + `ReplayFeed` reader are
  venue-agnostic on the OHLCV columns, so a Coinbase corpus in the same schema
  is consumable by `resolve_bakeoff_bars` without engine changes (architect
  verifies the `Venue`/`Symbol` wiring — `BTC-USD` vs `BTCUSDT` symbol-string
  handling is the one likely seam). **[architect, ADR-0084 D2.a] RESOLVED +
  amended:** confirmed venue-agnostic (`merge_symbols` reads by column name). The
  seam is resolved by storing the on-disk symbol dir as the **canonical `BTCUSDT`**
  (not `BTC-USD`) and mapping to the product-id `BTC-USD` only for the REST call
  via the existing `coinbase_symbol_map` — so the corpus reads with the same
  `Symbol::new("BTCUSDT")` the engine uses, zero engine change. A3's second clause
  ("architect verifies") is now discharged. **Correction to the reuse assumption:**
  the shipped `coinbase.rs` is a live-WS feed, NOT a REST backfiller → a new
  `fetch_coinbase_klines` bin + `coinbase_klines.rs` lib are required (only
  `coinbase_symbol_map` is reused).
- **A4** — `write_report=false` on the whole re-run keeps anchors 119/119 by
  construction (same contract as the shipped advisor bake-off + the P2-2 null-CI,
  both of which never touch the anchored report path).
- **A5** — the DVOL/macro/basis exogenous corpora on disk are 2023-24 (DVOL,
  basis) / 2021-2026 (macro); back-filling DVOL/macro to other windows is an
  additive per-year/per-month fetch that leaves existing pinned SHAs byte-identical.

## Changelog

- 2026-07-09 (analyst): brief created for **P2 of the ratified remediation
  plan** (`backlog.md` § Remediation plan). Availability reality-check
  (code + API grounded): existing Binance fetcher pages back to full listing
  history + is idempotent for gapped months; **Kraken REST OHLC hourly
  deep-history is INFEASIBLE** (720-candle cap, no backward `since` paging) →
  Coinbase-hourly recommended as the durable second-venue cross-check (reuses
  the paginate-window pattern; Coinbase already trust-blessed in the shipped
  DATA-quality panel), Kraken-daily the fallback. Proposed 3 new Binance
  corpora (1718 BTC/ETH/BNB, 2020 the-7-pre-2020-listers, 2526 all-10) with
  honest per-window symbol subsets driven by uneven listing dates. Honest
  per-corpus arm-availability matrix (DVOL/macro/basis arms legitimately absent
  on the eras that lack the exogenous data — graceful-degradation, not silent
  drop). Registered E-2 era-cost caveat (do NOT change the flat-bps default).
  AC1-AC8 = the tester's re-run-report contract (per-corpus×per-arm verdict
  table, null-CI, MinBTL before/after, wobble list, venue reconciliation,
  survivorship + era-cost caveats). 7 open questions Q-CE-1..7 for the architect
  M-T1. Anchors 119/119 + spec-lint PASS(0) by construction (write_report=false;
  frozen gate byte-untouched; existing pinned SHAs immutable). REQ-V3-P2-CORPUS-EXPANSION-001
  created in trace.toml (state=proposed, ADR-0082-compliant).
- 2026-07-09 (architect): **M-T1 design lock — [ADR-0084](../../architecture/adr/0084-p2-corpus-set-coinbase-adapter-verdict-rerun.md)**
  authored + registered atomically (README `## Registry` row + frontmatter in the
  same edit pass; `adr_registry_check.py` green `--pre-commit` + bare). Q-CE-1..7
  all resolved (§ Design): Q-CE-1 RATIFY the 3-corpus set unchanged; **Q-CE-2
  Coinbase-hourly RATIFIED** (Kraken hourly INFEASIBLE — 720-cap/no-backward-paging;
  a NEW `fetch_coinbase_klines` bin + `coinbase_klines.rs` lib required because the
  shipped `coinbase.rs` is a live-WS feed that CANNOT backfill — the ONE seam is the
  Coinbase→canonical-`BTCUSDT` symbol + `[time,low,high,open,close,vol]` seconds→millis
  schema shim, correcting A3); Q-CE-3 FETCH DVOL(2021-22+2025-26)+macro(2025-26)
  additive; Q-CE-4 stronger survivorship prose; Q-CE-5 opt-in `VolScaledSpread`
  era-cost annex (E-2, primary verdict on the frozen default); Q-CE-6 last-closed-
  UTC-month clamp; **Q-CE-7 a DEDICATED `p2_verdict_rerun` harness** (NOT a `--corpus`
  selector — the runner hardcodes `data/binance` in `preload_bakeoff_binance_bars`;
  the harness composes arbitrary-corpus `ReplayFeed` + `null_data_no_crown.rs::run_field_and_rank`,
  ZERO runner change, `write_report=false`). Four seams designed (fetcher bin /
  corpus pinning / re-run harness / AC1-AC8 report contract); § Backtest Scenarios
  = S1-S8 (6 primary corpora + 2 era-cost annex). `tasks.md` authored (ordered
  developer‖ lane, no UI lane — the DATA-quality panel already handles venue
  display). NO gate/band change (`bakeoff/{robustness,rank}.rs`/`classify_verdict`/
  `verdict_bands`/ADR-0066 byte-untouched — NOT a band proposal); existing pins
  byte-immutable (additive `--out` dirs); anchors 119/119 + spec-lint PASS(0) by
  construction; the 9 anchors.toml SHAs untouched (no anchor-additive re-emission).
- 2026-07-10 (developer): T1-T3 (Coinbase fetcher lib + bin + export), T2
  live dry-run + A2 probe, T6 exogenous back-fill (DVOL live-fetched, macro
  found already-complete/no-op), T9 (`p2_verdict_rerun.rs` harness, SKIP-safe
  + S4-smoke proven), T10 (full gate sweep green) — see `tasks.md` for
  file:line + test-command + output-line per row. status arch-done→dev-done;
  version 3.1.0→3.2.0. **Two real bugs found + fixed during testing (not
  design reversals):** (1) `coinbase_symbol_map` maps `BTCUSDT→BTC-USDT`
  (checks USDC→USDT→USD suffixes in that order), not `BTC-USD` as ADR-0084's
  prose implied for that input (the ADR's own worked example used the
  DIFFERENT input `BTCUSD`, where the helper IS correct) — fixed via a new
  `coinbase_product_id_for_symbol` delivering the ADR's actual design intent;
  (2) Coinbase Exchange requires a `User-Agent` header (discovered live during
  the T2 dry-run) — fixed with a fixed constant on every request. **A2
  RESOLVED:** BTC-USD hourly reaches back to 2015-08 (deeper than "~2015-16"),
  no window narrowing needed. **T6:** DVOL back-filled live (2021-22+2025-26,
  4→12 files, pre-existing files byte-identical via `shasum -a 256 -c`,
  `EXPECTED_DVOL_REVISION_SHA` re-pinned + proven via `real_corpus_load_smoke`);
  macro is a NO-OP on this machine (already covers 2021-01→2026-06). T4 (the 4
  multi-hour corpus fetches) + T7/T8 (blocked on T4's manifests) explicitly
  NOT done — orchestrator/tester scope; exact commands + watch probe +
  post-fetch verification handed off in the developer's `HANDOFF →` message.
  Gates: build/clippy(-D warnings)/fmt-check/anchors(119/119 before+after)/
  spec-lint(PASS-0)/adr_registry_check(exit 0) all green; FROZEN gate files
  byte-untouched (`git diff --stat`). HANDOFF → tester (or orchestrator runs
  T4 first, then tester completes T7/T8 + authors the AC1-AC8 re-run report).
  status proposed→arch-done; trace row → arch-done. HANDOFF → developer.
- 2026-07-10 (tester): **T4 already landed** (orchestrator fetch job ahead of
  this session, SHAs independently re-verified) — completed **T7**
  (`crates/data/tests/p2_corpora_revision_consistency.rs`, 4/4 un-ignored
  green) + **T8** (`crates/data/tests/p2_corpora_replayfeed_smoke.rs`, 4/4
  `--ignored` green, era-sanity bounds grounded in real on-disk price ranges)
  + **THE full S1-S8 multi-corpus verdict re-run**
  (`cargo test -p backtest --features realdata,yahoo --test p2_verdict_rerun
  -- --include-ignored --nocapture`: 15/15 passed, 0 failed, 238.79s). Authored
  the AC1-AC8 report
  [`reports/backtest-2026-07-10-p2-verdict-rerun.md`](reports/backtest-2026-07-10-p2-verdict-rerun.md)
  (NEW, non-anchored). **Headline finding, reported plainly per the feature's
  own no-suppression framing:** ship-passive WOBBLES on 2017-18/2020/2021-22
  (19/32 primary symbol-runs `ActiveWins`, 16 clear DSR≥0.95) but HOLDS on
  2025-26 (matching the 2023-24 baseline, 8/10 `BenchmarkWins`) and on the
  Coinbase venue cross-check (AC5: 3-8 bps median price agreement vs Binance
  across all 4 overlap windows). The `VolScaledSpread` era-cost annex (S7/S8)
  explains exactly 1 of 10 tested symbol-runs (DOGEUSDT/2020) as a pure
  cost-sensitivity flip — the rest of the older-era gradient survives that
  stress-test. `MinBTL` before/after (AC3): evidence base 3.99→7.90 years (2→5
  regimes + 1 venue cross-check), now MEETING the honest 6.36-year bar
  (previously SHORT by 2.36 years). Also root-caused (non-blocking, confirmed
  pre-existing on the byte-untouched 2324 baseline too, NOT a P2 regression) a
  `min_btl_years=0.00`/`n_eff=NaN` scorecard-math characteristic driven by
  `sharpe=NaN` on `_ls`/`always_short` arms propagating through Rust's
  `f64::max(NaN,x)==x` semantics — flagged as a future hardening item, out of
  P2's scope, no gate/band file touched. Gates: `verify_anchors.sh` 119/119
  before AND after; `spec_lint.py` PASS(0); `cargo fmt --check -p data` +
  `cargo clippy -p data --tests -- -D warnings` clean. status dev-done→tester-done;
  version 3.2.0→3.3.0; trace row → tested. VERDICT → PASS. HANDOFF → analyst
  (informational, non-blocking) on the wobble-list product-copy question.
- 2026-07-10 (developer): **P2 follow-on bug fix — scorecard `n_eff=NaN`/
  `min_btl_years=0.00` NaN-swallow hardening** (the tester's own flagged item
  above; `spec/dev-notes/p2-wobble-thesis-analysis-2026-07-10.md` § (d)).
  **Root cause, pinned exactly:** `compute_sharpe_hourly`
  (`crates/backtest/src/stats/mod.rs:52`, a frozen M-DEV-1 verbatim lift, out
  of scope to edit) guards a non-positive STARTING equity but not a
  positive-to-NEGATIVE crossing WITHIN one bar; `v0.sma_cross_ls` /
  `v0.always_short` (in every corpus's field via `default_short_field()`) can
  drive equity through zero, so `(curr / prev).ln()` computes `ln(negative)
  = NaN` for that window, and `Sharpe = NaN` survives into
  `CandidateKpis.sharpe` untouched. That `NaN` then poisoned TWO independent
  moment computations in `bakeoff/scorecard.rs`: (1) `n_eff()`'s
  mean/variance/correlation chain → `n_eff = NaN`, which `min_btl()`'s
  `n_eff.max(1.0 + f64::EPSILON)` silently clamped to `~1.0` per IEEE 754
  `f64::max(NaN, x) == x` (so `min_btl_years` read `~4.4e-16 ≈ "0.00"` —
  looked like zero, was actually a non-zero epsilon from a silent
  substitution, not an honest computed value); (2) `sharpe_variance()`
  (`dsr()`'s cross-trial variance `V` input) → `NaN`, which `dsr()`'s
  `(sharpe_variance_annualised / hpy).max(0.0)` silently clamped to `V=0.0`
  — a SECOND, previously-undiscovered manifestation of the same bug class:
  `deflated_sharpe` was NOT merely unaffected by the NaN (as it first
  appeared, since `crown_sr`'s `.fold(NEG_INFINITY, f64::max)` correctly
  drops NaN by construction) — it was ALSO silently computed off a wrong,
  artificially-zero `V`, over-crediting every crown.
  **Fix (`crates/backtest/src/bakeoff/scorecard.rs`, module doc "Degenerate-
  Sharpe hardening" section):** non-finite Sharpes are excluded from every
  moment-based statistic (`n_eff`, `sharpe_variance`) at the point of
  computation, NOT silently clamped after the fact; `n_candidates` (the "N
  tried" field size) is UNCHANGED — still counts every arm run, per the DSR/
  MinBTL literature's trial-count definition. `n_eff`/`min_btl`/`dsr` each
  additionally carry an explicit `is_nan()` guard (defense in depth) instead
  of relying on `f64::max`'s implicit NaN-propagation rule. No redesign, no
  ONC, closed-form preserved verbatim (ADR-0075 D4 untouched).
  **Proof (before/after S4, byte-untouched 2324 baseline, identical smoke
  command):** `n_candidates=25 n_eff=NaN deflated_sharpe=0.9947
  min_btl_years=0.00 crown_clears_dsr=true` → `n_candidates=25 n_eff=25.00
  deflated_sharpe=0.1979 min_btl_years=6.44 crown_clears_dsr=false`
  (`n_eff` finite + at the correct closed-form value for a genuinely
  low-correlation 23-arm field, ρ̄≈−0.045 verified by hand; `min_btl_years`
  a real 6.44y, not an epsilon artefact).
  **Blast-radius honesty (`--include-ignored` full matrix re-run, 15/15
  passed, 794.88s, byte-identical pass count to the tester's original run):**
  **`outcome` and crowned-arm-name are BYTE-IDENTICAL on all 32/32 primary
  symbol-runs — the FROZEN-gate report-only contract held exactly as
  designed.** `crown_clears_dsr` flips `true→false` on **17 of 32** primary
  rows (all 19 `ActiveWins` crowns' DSR values drop materially — e.g. S2
  LINKUSDT 0.9999→0.7762, S3 AVAXUSDT 1.0000→0.8127 — because the true,
  non-zero cross-trial Sharpe variance now feeds `dsr()` instead of the
  silently-zeroed `V=0.0`); **AC2's rollup changes from "16 of 19 (84%)
  `ActiveWins` crowns clear DSR" to "0 of 19 (0%) clear DSR"** post-fix — the
  DSR check is now materially MORE conservative on every row, which is the
  textbook-correct direction (a wider true variance makes the multiple-
  testing bar harder to clear, not easier). The S2→S8 DOGEUSDT era-cost
  OUTCOME FLIP (AC4) and all 3 named S7/S8 crown-swap rows are unchanged in
  crown identity (DSR values drop the same way, direction-consistent).
  **AC1 (`RecommendationOutcome`), AC3 (`MinBTL` 3.99→7.90y — computed
  independently from the corpus WINDOWS, never touched the buggy per-run
  field), AC4 (wobble list), and AC8 (top-line HOLDS/WOBBLES verdict) are
  ALL UNCHANGED** — none of them read the per-run `crown_clears_dsr`/`n_eff`
  fields this fix touches; only AC2's specific DSR-clear-rate numbers move.
  **New regression tests** (`crates/backtest/src/bakeoff/scorecard.rs`, 10
  new `#[test]`s incl. `compute_scorecard_s4_shape_two_nan_sharpes_stays_
  honest` pinning the exact S4 field shape) + all pre-existing scorecard
  tests green (26/26); gate-identity tests
  `scorecard_does_not_change_ranking` + `turnover_does_not_change_ranking`
  both green (proving the fix touches no ranking path). Gates:
  `cargo test -p backtest --lib` 238/238 (0 failed, 11 pre-existing ignored,
  unrelated); `cargo clippy -p backtest --features realdata,yahoo
  --all-targets -- -D warnings` clean; `cargo fmt --check -p backtest`
  clean; `verify_anchors.sh` 119/119 before AND after; `spec_lint.py`
  PASS(0). FROZEN gate (`bakeoff/{robustness,rank,mod}.rs`, `write_report`
  paths) byte-untouched — only `scorecard.rs` edited. version 3.3.0→3.3.1.
  No trace-state change (bug fix within ADR-0075's existing design, per the
  task brief's guard). HANDOFF → tester (verify-and-tick per the honest-tick
  rule; no ticks claimed here beyond this feature.md changelog entry, which
  is self-verified by the citations above).
