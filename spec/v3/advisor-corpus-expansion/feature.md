---
slug: advisor-corpus-expansion
status: proposed
owner: analyst
updated: 2026-07-09
version: 3.1.0
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
_architect fills this (M-T1). Resolve Q-CE-1..7; lock the venue choice (Q-CE-2)
and the exogenous-back-fill scope (Q-CE-3); confirm the corpus windows +
symbol-subsets (Q-CE-1) and the 2526 end-date clamp (Q-CE-6); decide the runner
seam for the re-run (a `--corpus <dir>` selector vs a dedicated
`p2_verdict_rerun` bench/bin); confirm whether any ADR amendment is owed (analyst
lean: likely just a small "P2 corpus-set + Coinbase adapter" ADR — no
anchor-additive re-emission, no gate change, no clamp change to the single-coin
engine)._

## Backtest Scenarios
_analyst + architect fill this using the backtest/scenario template — the re-run
is itself the scenario set: {1718, 2020, 2122, 2324, 2526} × {supported arms} +
the Coinbase BTC cross-check, all `write_report=false`._

## Implementation
_developer fills this (fetches AFTER the architect designs; emits the
copy-pasteable `watch -n N '<probe>'` block for the multi-hour multi-corpus
fetch + re-run per the long-running-task memory)._

## Verification
_tester links to the re-run report + the corpus-consistency tests here._

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
  handling is the one likely seam).
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
