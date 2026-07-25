---
slug: universe-method-diagnosis-2026-06-02
status: draft
owner: analyst
updated: 2026-06-02
tags: [universe, cross-sectional, factor-structure, correlation, dispersion, rank-ic, method-vs-universe, robustness, go-no-go, post-program-diagnosis, broader-universe-spike, time-series-momentum]
related:
  - spec/carry-strategy/reports/test-2026-06-02-carry-strategy.md
  - spec/carry-strategy/presentations/carry-strategy-2026-06-02.md
  - docs/dev-notes/frame-diagnostic-2026-05-31.md
  - docs/dev-notes/robustness-decision-rule-2026-05-30.md
  - spec/product.md
---

# Universe vs Method diagnosis — why all three cross-sectional families failed

> **Mandate (research + scoping, no build).** The robustness program returned a
> uniform negative: momentum (path + param fragile), mean-reversion (param
> fragile), and carry/funding (FAMILY-UNIFORM-FRAGILE on BOTH 2023 + 2024) are
> all dominated by passive equal-weight buy-and-hold on the same 10-symbol
> universe. The frame-diagnostic (2026-05-31) already established the BH bar is
> FAIR and the signals are weak. This note asks the next question: **is the
> binding LIMITER the universe, the cross-sectional method, or (already shown)
> the signals?** Every number below is computed from the banked OHLCV via the
> SAME `ReplayFeed::merge_symbols` reader the harness uses (revision pin
> `3a8b96c4…`), not asserted. The disambiguating experiment is scoped + costed
> at the end for the operator's go/no-go.

---

## 0. TL;DR — the headline diagnosis (with confidence)

**The limiter is a DUAL structural property of {universe × cross-sectional
method}, NOT primarily the signals (those were already shown weak), and NOT a
degenerate single-factor universe.** Two computed facts, both holding across
2023 AND 2024, jointly explain all three failures at once:

| Finding | 2023-FY | 2024-FY | What it means |
|---|---|---|---|
| **Avg pairwise return corr** (45 pairs, 1h log-ret) | **0.631** | **0.683** | High co-movement, but ~32–37% variance is still NOT common → universe is **not** a degenerate 1-factor block |
| **Avg R² vs equal-weight index** (1-factor share) | **0.667** | **0.715** | ~67–71% of each name's variance is common market beta; ~29–33% idiosyncratic → moderate single-factor dominance, **not** ~1.0 |
| **Cross-sectional rank IC** (does relative-strength rank PERSIST?) | **≈ 0** at every horizon (range −0.049…+0.023) | **≈ 0** at every horizon (range −0.070…+0.015) | **THE decisive number.** Relative-strength ranking has **no forward persistence** → cross-sectional *ranking itself* has nothing to exploit, regardless of which signal builds the rank |

**Read:** the original hypothesis — "the universe is ~1 factor (nearly all common
beta), a structural ceiling on cross-sectional alpha" — is **partially refuted**:
at avg R² ≈ 0.67–0.71 there IS meaningful idiosyncratic variance (a genuine
~30% cross-sectional component exists). So a structural-ceiling-by-pure-beta
story does **not** fully hold. **The sharper, better-supported finding is the
rank-IC ≈ 0 result: the idiosyncratic dispersion that exists is essentially
*unpredictable* from the relative-strength rank at every horizon and in both
years.** Cross-sectional ranking is the binding constraint — there is dispersion
to harvest, but the ranking signal does not persist long enough to harvest it
net of any friction. This is **signal-agnostic** (it is a property of the rank,
not of momentum-vs-MR-vs-carry), which is exactly why momentum, its inverse
(MR), AND carry all failed uniformly.

**Confidence:**
- Avg-correlation + avg-R² numbers: **HIGH** (8 758 / 8 783 aligned hourly
  returns per year; deterministic; two independent years agree).
- Rank-IC ≈ 0 as the binding constraint: **MEDIUM-HIGH** (consistent across 6
  horizons × 2 years, all within ±0.07 of zero with no stable sign; the
  small-negative short-horizon tilt is consistent with MR's failure too). The
  one caveat: rank IC is computed on raw forward returns, not on the exact
  fee-and-rebalance-timed P&L the engine books — it is the *upper-bound
  information content* of the ranking, and it is already ≈ 0, so the engine
  cannot do better. That makes it a strong necessary-condition argument.
- "Universe is the limiter" vs "method is the limiter": the two are
  **entangled** and cannot be fully separated from these statics alone — which
  is precisely what the § 4 disambiguating experiment is designed to break.

**One-line operator framing:** *"It is not that the coins are one single blob —
there is ~30% room between them. It is that nothing about which coin led
yesterday tells you which leads tomorrow. Cross-sectional ranking, on this
universe, is sorting noise — so every ranking strategy we tried sorted noise."*

---

## 1. What was run (reproducibility)

A throwaway read-only example, `crates/data/examples/universe_diag.rs` (NOT a
committed bin, NOT anchored), loads the 10 USDT pairs through
`data::ReplayFeed::merge_symbols(..., Timeframe::OneHour)` — the **identical
reader path** `crates/backtest/src/realdata.rs` uses — so every number traces to
the banked parquets under revision `3a8b96c43f2d…`. It computes, per calendar
year, on aligned 1h log-returns over the timestamp-intersection of all 10 names:

1. **M1** — average pairwise Pearson correlation (45 unique pairs).
2. **M2** — cross-sectional return dispersion (std across the 10 names at each
   bar): time-mean + p10/p50/p90.
3. **M3** — 1-factor decomposition: per-name R² = squared correlation with the
   equal-weight index return (the BH proxy); average R² = common-beta share.
4. **M4** — cross-sectional rank IC: at each non-overlapping L-bar step, rank the
   10 names by trailing-L cumulative return, correlate (Spearman) against the
   forward-L cumulative return; average over the year. This is the
   signal-agnostic test of whether relative-strength rank *persists*.

```
Run:  cargo run -p data --example universe_diag -- 2023
      cargo run -p data --example universe_diag -- 2024
```

Universe (from `data/binance/` + the train_garch/regime/forecast bins, all
identical): `ADAUSDT, AVAXUSDT, BNBUSDT, BTCUSDT, DOGEUSDT, DOTUSDT, ETHUSDT,
LINKUSDT, SOLUSDT, XRPUSDT`. Aligned bars: **8 758 returns (2023)**, **8 783
returns (2024)**.

> Cleanup contract: `crates/data/examples/universe_diag.rs` is a disposable
> diagnostic. It may be kept as a re-runnable universe-structure probe (it is
> read-only, depends only on the banked data + the public `ReplayFeed` API, and
> touches no anchor) OR deleted — operator's call. Throwaway output lived in
> `/tmp/uni-diag/`. No anchors touched; no `data/binance` bytes touched.

---

## 2. The evidence (real computed numbers)

### M1 — pairwise correlation: high, but not a single blob

| | 2023-FY | 2024-FY |
|---|---|---|
| Avg pairwise corr (45 pairs) | **0.6306** | **0.6832** |
| Min pair | 0.4750 (SOL/XRP) | 0.5346 (SOL/XRP) |
| Max pair | 0.8607 (BTC/ETH) | 0.8118 (BTC/ETH) |

The universe co-moves strongly (avg 0.63–0.68) but the *minimum* pair is still
only ~0.48–0.53 — there is a real spread of co-movement. A degenerate
one-factor universe would show avg corr → 0.9+. **This is a moderately-correlated
large-cap basket, not a single asset wearing ten tickers.**

### M2 — cross-sectional dispersion: there IS raw material to rank

| | 2023-FY | 2024-FY |
|---|---|---|
| Time-mean dispersion (std across 10 names/bar) | **0.352%/bar** | **0.421%/bar** |
| p10 / p50 / p90 | 0.136% / 0.269% / 0.652% | 0.185% / 0.337% / 0.736% |
| ratio: avg single-name σ ÷ avg dispersion | 2.06 | 2.16 |

At any given hour the 10 names spread by ~0.35–0.42% (std), and the typical
single-name move is only ~2× the cross-sectional spread. **Dispersion is
non-trivial** — cross-sectional ranking is NOT starved of raw material. (If
dispersion were ~0, ranking would be moot; it is not. This is the first half of
why the pure "universe ceiling" story does not fully hold.)

### M3 — 1-factor decomposition: ~67–71% common beta, ~30% idiosyncratic

| | 2023-FY | 2024-FY |
|---|---|---|
| **Avg R² vs EW index** | **0.667 (66.7% common)** | **0.715 (71.5% common)** |
| Highest-R² name | ETH 0.749 | ADA 0.786 |
| Lowest-R² name | XRP 0.489 | XRP 0.563 |

Per-name idiosyncratic share ranges ~0.21–0.51. **Roughly a third of each name's
variance is its own.** That idiosyncratic third is exactly the space a
cross-sectional strategy would need to monetize. It exists. So the limiter is
NOT "there is no idiosyncratic return to differentiate the names." The limiter is
whether that idiosyncratic component is *predictable from the ranking signal* —
which is M4.

### M4 — cross-sectional rank IC ≈ 0 at every horizon (the decisive finding)

Mean Spearman rank IC between trailing-L-bar return rank and forward-L-bar
return, non-overlapping windows:

| Lookback L (bars) | 2023 mean rank IC | 2024 mean rank IC |
|---|---|---|
| 3 (3h) | −0.0487 | −0.0361 |
| 9 (9h ≈ carry L) | −0.0073 | −0.0061 |
| 24 (1d) | −0.0485 | +0.0146 |
| 60 (2.5d ≈ momentum tier-1) | −0.0335 | −0.0296 |
| 168 (1wk) | +0.0227 | −0.0698 |
| 720 (30d ≈ momentum long) | −0.0127 | −0.0259 |

**Every IC is within ±0.07 of zero, most within ±0.05, with no stable sign across
horizons or years.** A tradeable cross-sectional momentum signal typically shows
a persistent positive IC of +0.03…+0.10 *with a stable sign*; a tradeable
reversal shows a persistent *negative* IC. This universe shows **neither** — the
sign flips between horizons and years, and the magnitude is at the noise floor.
The faint negative tilt at the shortest horizon (L=3: −0.049/−0.036) is the only
consistent structure, and it is (a) tiny and (b) the *reversal* direction —
consistent with MR being marginally "less wrong" than momentum but still far too
weak to clear fees, exactly as the MR family verdict found.

**This is the load-bearing number.** It says: irrespective of whether you build
the rank from price-momentum, price-reversal, or funding-carry, the rank does not
forecast forward relative performance on this universe. The signals were not
"weak by bad luck" — the *ranking channel itself* carries ≈ 0 forward
information here.

---

## 3. Diagnosis — universe vs method vs signals

### 3.1 Signals (already shown — NOT re-litigated)
Momentum FRAGILE even at 0 bps (frame-diagnostic E2), MR FRAGILE, carry
FAMILY-UNIFORM-FRAGILE on two years. The signals are weak. **This note does not
add to that; it explains WHY they were uniformly weak.**

### 3.2 Universe — PARTIAL limiter, NOT a clean ceiling
- The original "~1 factor ⇒ structural ceiling" hypothesis is **partially
  refuted**: avg R² 0.67–0.71 (not ~1.0), avg corr 0.63–0.68 (not ~0.9+),
  non-trivial dispersion (0.35–0.42%/bar). There is a genuine ~30% idiosyncratic
  cross-section.
- BUT the high common-beta share is still a **headwind**: ~67–71% of every name's
  move is the same market factor, which (i) is exactly what equal-weight BH
  already captures for free, and (ii) means a long-only cross-sectional tilt is
  mostly re-buying the market beta it cannot escape — corroborating the carry
  verdict's "directional price exposure overwhelms the funding premium" read and
  the momentum/MR "you're just churning the common drift" read.
- **Verdict on universe:** a contributing structural headwind (high beta share,
  and only 10 large-caps so the idiosyncratic cross-section is shallow), but
  **not** a hard mathematical ceiling. A broader/more-dispersed universe could
  plausibly *raise* the idiosyncratic share — see § 3.4.

### 3.3 Method (cross-sectional ranking) — the BINDING constraint
- M4 is signal-agnostic and ≈ 0 at every horizon, both years. **The
  cross-sectional ranking channel carries ~no forward information on this
  universe.** This single fact explains the *uniformity* of the three failures in
  a way the signal-weakness story cannot: three different signals (trend, reverse-
  trend, funding) all feed the SAME ranking channel, and that channel is empty.
- This is the cleanest available explanation: it is not three independent
  coincidental signal failures; it is one shared dead channel.

### 3.4 Why "universe" and "method" are entangled (and how to separate them)
Rank IC ≈ 0 could be caused by **either** "ranking is the wrong method (even a
good universe wouldn't rank-predict at 1h)" **or** "this *particular* 10-coin
universe is too beta-dominated / too shallow for ranking to find persistent
idiosyncratic winners." The statics cannot separate these. The § 4 experiment is
designed to cut exactly this knot with the cheapest possible test.

---

## 4. The disambiguating experiment (scoped + costed) — RECOMMENDED

**The single cheapest experiment that most cleanly separates the three limiters
is a TIME-SERIES (single-asset, NOT cross-sectional) absolute-momentum / trend
test on the SAME banked universe, run through the SAME block-bootstrap robustness
harness, scored against the SAME frozen decision rule.**

### 4.1 Why this is the right disambiguator
- It **removes the ranking channel entirely** (no cross-sectional sort). Each
  asset is traded long/flat on its OWN trailing-return sign (time-series momentum
  / absolute momentum — the single most-cited crypto effect that is NOT
  cross-sectional). The portfolio is the equal-weight average of the per-asset
  time-series rules.
- **Logic table that makes it a clean separator:**

  | TS-momentum result | Conclusion it forces |
  |---|---|
  | TS-momentum clears the bar (≥ MARGINAL) where x-sec failed | **METHOD was the limiter** — ranking-across-names was the dead channel; time-series direction on these same assets carries an edge. Pivot the product to time-series, not cross-sectional. |
  | TS-momentum ALSO FRAGILE | Limiter is **deeper than method** — the same 10 assets carry no robust directional OR relative edge at 1h. Strongly implicates the **universe + horizon** (these large-caps at 1h are efficient-ish); next move is the broader-universe spike (§ 4.4), not another method. |

  Either outcome is decision-grade and cheap. This is strictly more informative
  than building a 4th cross-sectional family (which M4 predicts will also fail).

### 4.2 Cost (by direct analogy to the proven 6×200 sweeps)
- The carry/momentum θ-surfaces ran **6 cells × 200 paths ≈ 20–31 s wall-clock**
  on the canonical box (carry: 30.7 s / 28.4 s; both committed). The harness, the
  bootstrap, the BH control, the decision-rule classifier, and the anchor
  machinery already exist and are proven.
- A time-series-momentum surface is the **same shape**: a small θ-grid
  (e.g. 6 cells over {trailing-lookback × entry/exit band}) × N=200 paths × 2
  years. **Compute ≈ identical: ~20–35 s per year, ~1 min for both years.**
- **The cost is NOT compute — it is the ~1.5–3 dev-days to add a `TimeSeries`
  (absolute-momentum, long/flat-per-asset) score/sizing path to the strategy
  crate** and wire a `--score-source ts-momentum` arm into `param_robustness_sweep`,
  WITH the mandated day-1 baseline-divergence e2e test (CLAUDE.md non-negotiable:
  every sizing-modifier ships with the ≥1 bp divergence-from-baseline e2e from
  day 1, per the v3-vol-overlay no-op precedent). The existing carry build is the
  template — it added exactly one `ScoreSource` arm + falsifiers + a 2-year
  anchored surface for comparable effort.

### 4.3 Pre-registration (lift the frozen rule, do not move it)
Score the TS-momentum surface against `robustness-decision-rule-2026-05-30.md`
§ 0 verbatim (5-signal weakest-link composite; void-if-`gbm-smoke`-or-per-symbol-
independent pre-flight). Pre-register the read BEFORE the run, per the
scientific-integrity discipline that the whole program has followed (and per the
fabricated-"Sharpe 1.40" cautionary precedent — every number must trace).

### 4.4 The broader-universe spike (the cheaper fallback / parallel option)
If the operator would rather test the **universe** axis first (or instead):
- **Feasibility is HIGH and cheap.** `crates/data/src/bin/fetch_binance_klines.rs`
  takes arbitrary `--symbols` (comma-separated) over arbitrary `--start/--end` at
  `1h` — it is the exact tool that produced the current 10 symbols. Extending to
  e.g. 30–50 mid-cap USDT pairs is a fetch + a REVISION.toml regen, no new code.
  (Funding data for carry, by contrast, is banked for only the same 10 names in
  `data/binance-funding/` — a broader carry universe would need a parallel
  funding fetch via `fetch_binance_funding`.)
- **The cheapest test is to re-run THIS diagnostic** (`universe_diag.rs`, ~10 s)
  on the broader symbol list and read M3 (avg R²) + M4 (rank IC). **If a broader,
  smaller-cap universe drops avg R² toward ~0.4–0.5 AND lifts rank IC to a
  persistent ±0.03+**, that is direct evidence the universe was the limiter and a
  cross-sectional rebuild on the broader universe is warranted. If R²/IC look the
  same as the large-caps, the universe is exonerated and the method/horizon is the
  culprit. **This is a ~0.5-day data spike + a 10-second re-run — the highest
  information-per-dollar move on the board**, and it requires NO strategy code.

### 4.5 Recommended sequencing (durable-over-quick)
**(Recommended) Run the § 4.4 broader-universe diagnostic spike FIRST (~0.5 day,
no code), THEN the § 4.1 time-series-momentum harness experiment (~1.5–3 dev-days
+ ~1 min compute).** Rationale: the spike is nearly free and can *immediately*
exonerate or implicate the universe before any strategy code is written; its
result sharpens whether the TS experiment should run on the current 10 or a
broader set. This ordering avoids building a TS path on a universe that the spike
might reveal is the actual limiter (which would spawn a "now redo it on the
broader universe" follow-on — the exact rework the durable-first rule exists to
prevent).

**If-budget-tightens fallback:** run ONLY the § 4.4 spike (~0.5 day, zero
strategy code). It cannot by itself prove the method works, but it can cheaply
kill or keep the "broaden the universe" thesis, and it directly tests the
operator's headline question ("is the universe the limiter?") for almost nothing.
Defer the TS-momentum build until the spike says the universe is NOT the sole
limiter.

---

## 5. Proposed next feature (STUB — for operator go/no-go; NOT greenlit, NO build)

> Per the trace.toml ownership rule, the analyst creates the `[[req]]` row when a
> feature enters `proposed`. **This stub is deliberately NOT yet committed as a
> `proposed` row** — the operator's go/no-go decides whether it is greenlit. On
> approval, the analyst will create the row and a `spec/<slug>/feature.md`.

**Slug (proposed):** `time-series-momentum-robustness` (working title)

**Why:** All three *cross-sectional* families are retired; M4 shows the
cross-sectional ranking channel carries ≈ 0 forward information on this universe.
The cleanest, harness-reusing next test is whether a **time-series** (per-asset
absolute-momentum, long/flat) edge exists where the cross-sectional one did not —
which both disambiguates method-vs-universe and tests the single most-cited crypto
effect not yet examined. Pre-registered against the frozen decision rule.

**Scope (minimal viable):**
- New `ScoreSource::TimeSeriesMomentum` (or a long/flat-per-asset sizing arm) in
  `crates/strategy` — per-asset trailing-return sign, NOT a cross-sectional sort.
- `--score-source ts-momentum` arm in `param_robustness_sweep` with a small
  LOCKED θ-grid (lookback × entry/exit band), reusing run_path + DistributionSummary
  + BlockBootstrapPathGen + BH control verbatim.
- **Day-1 mandated gate:** a baseline-equity-divergence e2e (≥ 1 bp vs the
  un-traded baseline) per the CLAUDE.md non-negotiable + the no-look-ahead +
  two-run byte-identity falsifiers (the carry build is the template).
- Two anchored surfaces (2023 + 2024) under a new namespace; scored against the
  frozen § 0 decision rule, pre-registered.

**Pre-condition (gate the build on the spike):** run the § 4.4 broader-universe
diagnostic first; if it shows the universe is the dominant limiter, re-scope this
feature to run on the broader universe before building.

**Expected cost:** ~1.5–3 dev-days + ~1 min compute (by analogy to the carry
build + 6×200 sweep).

**Expected outcome (honest prior):** MEDIUM. Time-series momentum is the most
robust documented crypto effect, but the same 1h large-cap universe that killed
the cross-sectional families may be efficient enough at the directional level
too. The value is in the *clean disambiguation* (§ 4.1 logic table) regardless of
sign — a FRAGILE result here is itself decision-grade (it implicates universe +
horizon and routes to the broader-universe rebuild).

---

## 6. Assumptions & limits (challengeable)

1. **Rank IC is computed on raw forward returns, not on fee/rebalance-timed P&L.**
   It is the *upper bound* on the ranking's information content — and it is already
   ≈ 0, so the engine cannot extract more. This strengthens (does not weaken) the
   "method is the binding constraint" read.
2. **Non-overlapping windows** were used for M4 to keep the per-window ICs
   independent; overlapping windows would give more samples but autocorrelated
   estimates. The conclusion (IC ≈ 0 with unstable sign) is not a close call, so
   the windowing choice does not threaten it.
3. **Equal-weight index as the single factor.** A cap-weighted or
   PCA-first-component factor would shift per-name R² slightly, but the
   equal-weight index is exactly the BH benchmark the whole program scores
   against, so it is the decision-relevant factor. Avg R² 0.67–0.71 would not
   collapse to ~1.0 under any reasonable single-factor choice on a 10-name basket
   with a 0.48 min pair correlation.
4. **1h bars, 2 banked years (2023 + 2024).** Findings are specific to this
   granularity and these two regimes. A different horizon (daily) or out-of-sample
   year could differ — but the program's entire scope is this banked data, and the
   robustness harness cannot synthesize regimes the source years did not contain
   (per the decision-rule § 5 scope bound).
5. **"Universe vs method" entanglement is real and is the reason for the § 4
   experiment** — this note narrows the field to {universe × method} and rules the
   signals out as the *primary* cause, but does not claim to have fully separated
   universe from method from statics alone. That separation is the experiment's job.

---

## Spike results (broader universe)

> **Spike mandate (operator-approved, ~0.5 day, RESEARCH only — no strategy
> build).** The § 4.4 broader-universe confirmation step. Question: does a
> broader, more-dispersed mid-cap universe (a) LOWER the common-beta share
> (avg R²), and (b) REVIVE cross-sectional rank IC to a persistent ±0.03+?
> Every number below traces to freshly-fetched banked OHLCV under a NEW,
> independent revision pin — NO fabrication (per the fabricated-"Sharpe 1.40"
> precedent). The original 10-symbol `data/binance` pin `3a8b96c4…` was NOT
> touched.

### S.0 TL;DR — the verdict

**The cross-sectional ranking METHOD is the limiter, NOT the universe.** Moving
to a broader, lower-cap, deliberately-more-dispersed universe **(a) DID lower the
common-beta share materially** (avg R² 0.715 → 0.598 on 2024, a ~12-point drop;
avg pairwise corr 0.683 → 0.582) — confirming the large-cap universe *was* a
genuine beta headwind as § 3.2 suspected — **but (b) did NOT revive
cross-sectional rank IC.** Rank IC stayed ≈ 0 at every horizon in both years and
at both universe sizes, with the SAME faint-negative short-horizon tilt and no
stable positive (momentum-direction) sign. Lowering common-beta by ~12 points
bought **zero** rank-persistence. This is the clean disambiguation the spike was
designed to deliver:

> **The idiosyncratic cross-section got bigger, and the ranking signal still
> could not forecast it.** The dead channel is the ranking method, not the basket
> it ranks. → **Proceed to the TIME-SERIES-MOMENTUM build as the method fix; do
> NOT re-attempt a cross-sectional family on the broader universe.**

### S.1 What was fetched (reproducibility)

`fetch_binance_klines` (the same tool + schema that produced the 10-symbol set)
into a **separate output directory `data/binance-broaduni/`** so the existing
`data/binance/REVISION.toml` pin (`3a8b96c4…`) stays byte-immutable. New
self-contained manifest `data/binance-broaduni/REVISION.toml`, aggregate SHA
**`518b4d4091ebe67d5e7828c5ac67e3e35f937e6cb9c775c235368ba2a5ba07e2`** (covers
both years; the 2024-only intermediate pin was `2ed2bf677e…`).

- **Selection method:** Binance `exchangeInfo` → filtered to `TRADING` USDT spot
  pairs; excluded the existing 10 + all stablecoins; required full-2024 hourly
  history (probed via a 1-bar klines call at `startTime=2024-01-01`, dropped
  JUP/PYTH which list late-Jan/Feb-2024); ranked the survivors by current 24h
  quote volume and took the **top 35 mid-caps** (the next liquidity tier below the
  large-cap 10). This is deliberately a lower-cap, more-dispersed tier — exactly
  the set most likely to lower common-beta if the universe were the limiter.
- **2024-FY (35 symbols, all fetched cleanly — 420/420 month-parquets, 0 gaps):**
  `NEARUSDT, ZECUSDT, WLDUSDT, XLMUSDT, SUIUSDT, FETUSDT, TRXUSDT, INJUSDT,
  ICPUSDT, PEPEUSDT, JTOUSDT, LTCUSDT, SEIUSDT, HBARUSDT, ORDIUSDT, AAVEUSDT,
  FILUSDT, CHZUSDT, DASHUSDT, UNIUSDT, APTUSDT, BCHUSDT, ARBUSDT, OPUSDT,
  ALGOUSDT, TIAUSDT, APEUSDT, LDOUSDT, DYDXUSDT, CFXUSDT, SANDUSDT, GALAUSDT,
  ETCUSDT, ATOMUSDT, CRVUSDT`.
- **2023-FY (27 symbols — the subset of the 35 with full-2023 history; 324/324
  month-parquets, 0 gaps):** the 35 minus the 8 that listed mid-2023
  (`WLDUSDT, SUIUSDT, PEPEUSDT, JTOUSDT, SEIUSDT, ORDIUSDT, ARBUSDT, TIAUSDT`).
  2023 was added because it was cheap and gives a true out-of-year replication on
  the SAME 27 names. A full-35 intersection for 2023 is impossible (JTO lists
  2023-12-07 → the 35-name 2023 window collapses to ~December), which is why the
  2023 read is on the 27-name like-for-like subset.
- **No fetch failed**; every requested symbol-month returned a full bar count
  (744/720/696 per month as expected for 1h). Aligned-bar intersection: **8 783
  returns (2024)**, **8 758 returns (2023)** — same depth as the 10-symbol run.

> **Reproduce** (read-only, ~10 s each; probe extended to take `--root` +
> `--symbols`, 10-symbol default behavior preserved & regression-checked —
> baseline 2024 reproduces avg corr 0.6832 / 8 783 returns byte-identically):
> ```
> cargo run -p data --example universe_diag -- 2024 --root data/binance-broaduni \
>   --symbols NEARUSDT,ZECUSDT,WLDUSDT,XLMUSDT,SUIUSDT,FETUSDT,TRXUSDT,INJUSDT,ICPUSDT,PEPEUSDT,JTOUSDT,LTCUSDT,SEIUSDT,HBARUSDT,ORDIUSDT,AAVEUSDT,FILUSDT,CHZUSDT,DASHUSDT,UNIUSDT,APTUSDT,BCHUSDT,ARBUSDT,OPUSDT,ALGOUSDT,TIAUSDT,APEUSDT,LDOUSDT,DYDXUSDT,CFXUSDT,SANDUSDT,GALAUSDT,ETCUSDT,ATOMUSDT,CRVUSDT
> cargo run -p data --example universe_diag -- 2023 --root data/binance-broaduni \
>   --symbols NEARUSDT,ZECUSDT,XLMUSDT,FETUSDT,TRXUSDT,INJUSDT,ICPUSDT,LTCUSDT,HBARUSDT,AAVEUSDT,FILUSDT,CHZUSDT,DASHUSDT,UNIUSDT,APTUSDT,BCHUSDT,OPUSDT,ALGOUSDT,APEUSDT,LDOUSDT,DYDXUSDT,CFXUSDT,SANDUSDT,GALAUSDT,ETCUSDT,ATOMUSDT,CRVUSDT
> ```

### S.2 Axis (a) — common-beta share: LOWERED (universe WAS a beta headwind)

Avg R² vs equal-weight index (common-beta share) and avg pairwise correlation,
broader universe vs the 10-symbol baseline:

| Universe | Year | Avg pairwise corr | Avg R² vs EW idx | Min pair | Dispersion %/bar |
|---|---|---|---|---|---|
| **10 large-cap** (baseline) | 2023 | 0.631 | **0.667** | 0.475 (SOL/XRP) | 0.352% |
| **10 large-cap** (baseline) | 2024 | 0.683 | **0.715** | 0.535 (SOL/XRP) | 0.421% |
| **27 mid-cap** | 2023 | 0.557 | **0.576** | 0.352 (TRX/CFX) | 0.571% |
| **27 mid-cap** | 2024 | 0.592 | **0.612** | 0.306 (FET/TRX) | 0.627% |
| **35 mid-cap** | 2024 | 0.582 | **0.598** | 0.266 (TRX/JTO) | 0.698% |

**Read:** every broader-universe cut LOWERS avg R² by ~10–14 points (0.715 →
0.598 on the headline 2024 35-name cut) and LOWERS avg pairwise corr by ~0.09–0.10.
Idiosyncratic share rose from ~28% to ~40% (and the per-name spread widened: TRX
is nearly market-neutral at R²=0.22 / β=0.30; ZEC, HBAR, XLM, JTO all sit at
R² 0.44–0.48). Cross-sectional dispersion ~doubled (0.42% → 0.70%/bar). **The
broader universe is materially less beta-dominated and has more raw dispersion to
rank — the § 3.2 "high common-beta is a contributing headwind" suspicion is
CONFIRMED.** So axis (a) = YES.

### S.3 Axis (b) — cross-sectional rank IC: did NOT revive (still ≈ 0)

Mean Spearman rank IC (trailing-L rank vs forward-L return, non-overlapping
windows), broader universe vs the 10-symbol baseline:

| Lookback L | 10-sym 2023 | 10-sym 2024 | 27-sym 2023 | 27-sym 2024 | 35-sym 2024 |
|---|---|---|---|---|---|
| 3 (3h) | −0.049 | −0.036 | −0.054 | −0.043 | −0.041 |
| 9 (9h) | −0.007 | −0.006 | −0.024 | −0.021 | −0.021 |
| 24 (1d) | −0.049 | +0.015 | −0.054 | −0.062 | −0.051 |
| 60 (2.5d) | −0.034 | −0.030 | −0.040 | −0.039 | −0.034 |
| 168 (1wk) | +0.023 | −0.070 | −0.057 | −0.039 | −0.021 |
| 720 (30d) | −0.013 | −0.026 | **+0.088** | −0.026 | +0.005 |

**Read:** on the broader universe rank IC is STILL pinned at the noise floor.
Across the three broader-universe columns, **14 of 18 horizon-cells are negative**
(the reversal direction), and NONE of the negative cells is large enough to be
tradeable net of fees. There is **no persistent positive (momentum-direction)
IC** — the signature a cross-sectional momentum strategy would need. If anything
the faint short-horizon negative tilt (L=3: −0.041…−0.054) is now slightly MORE
consistent across horizons than on the large-caps, i.e. the broader universe is,
if anything, marginally more reversal-flavoured and even less momentum-rankable.
Lowering common-beta by ~12 points moved rank IC by essentially nothing. So
axis (b) = **NO**.

**Honest flag on the one positive outlier:** the 27-sym 2023 L=720 cell is
**+0.088** — the single positive momentum-direction reading in the table. It sits
on **n=11 non-overlapping 30-day windows** (one calendar year ÷ 30d), i.e. it is
a near-meaningless sample, and it does NOT replicate (27-sym 2024 L=720 = −0.026;
35-sym 2024 L=720 = +0.005; 10-sym both years negative). It is exactly the
"sign-flips-between-years, magnitude-at-the-noise-floor" pattern the original M4
documented — not evidence of a long-horizon cross-sectional momentum effect. I
am calling it out rather than burying it (fabricated-Sharpe-precedent discipline).

### S.4 Verdict — METHOD-limiter (not universe-limiter)

Mapping to the spike's pre-registered decision:

| Axis | Result | |
|---|---|---|
| (a) Broader universe LOWERS common-beta (avg R²)? | **YES** (0.715 → 0.598) | universe *was* a real headwind |
| (b) Broader universe REVIVES rank IC to persistent ±0.03+? | **NO** (still ≈ 0, no stable positive sign, both years) | ranking channel still empty |

**Both halves must hold to implicate the universe as the binding limiter. Half (b)
fails decisively. → VERDICT: the cross-sectional RANKING METHOD is the limiter
regardless of universe.** The universe was a *contributing* headwind (a is real),
but removing ~12 points of common-beta did not make the ranking forecast forward
relative performance — the idiosyncratic cross-section grew and remained
*unpredictable from the rank*. This is the § 4.1 "NO" branch, now confirmed on
real broader-universe data rather than inferred from statics. **Confidence: HIGH**
— the result holds across two universe sizes (27, 35) and two independent years,
and the one dissenting cell (S.3) is an 11-sample non-replicating outlier.

### S.5 Firmed-up recommendation for the TS-momentum build

1. **PROCEED to the `time-series-momentum-robustness` build as the method fix
   (§ 5 stub), now with the universe-axis pre-condition CLEARED.** The spike has
   exonerated "just broaden the universe" as a cross-sectional fix: a more-dispersed
   universe does not revive ranking. The next legitimate experiment is the one that
   *removes the ranking channel entirely* — per-asset absolute-momentum / trend
   (long/flat on each asset's OWN trailing-return sign), exactly as § 4.1 scoped.

2. **Target universe for the TS-momentum build: the ORIGINAL 10-symbol large-cap
   set under `data/binance` (pin `3a8b96c4…`).** Rationale (durable-over-quick):
   - The disambiguation is now done; the TS build's *job* is the clean method
     test (does per-asset direction carry an edge where ranking did not), and the
     10-symbol set is where the entire robustness program (momentum/MR/carry
     surfaces, the frozen decision rule, the BH control, the anchors) already
     lives. Running TS-momentum on the SAME 10 keeps it directly comparable to the
     three retired cross-sectional families and reuses the existing banked pin +
     funding data with **zero new data-plumbing risk**.
   - The broader universe's value was *diagnostic* (it answered the universe-vs-
     method question), not as a trading set: the higher idiosyncratic share is
     irrelevant to a time-series rule, which trades each asset's own trend and does
     not benefit from cross-name dispersion. So there is no TS-specific reason to
     pay the broader-universe plumbing cost (funding not banked for the 35; new pin
     to wire into the harness/anchors).
   - **If — and only if — the TS-momentum result is itself FRAGILE on the 10**, the
     next move per the § 4.1 logic table is the broader-universe + horizon axis;
     and at that point this spike's `data/binance-broaduni` set (pin `518b4d40…`)
     is already banked and ready to feed a broader TS retest without a re-fetch.
     i.e. the broader data is now a pre-positioned fallback, not wasted.

3. **Do NOT spin up a 4th cross-sectional family on the broader universe.** M4 +
   this spike jointly predict it would also fail; that would be the exact
   "build-then-discover-the-channel-is-dead" rework the durable-first rule exists
   to prevent.

### S.6 Spike artifacts & cleanup

- **New banked data:** `data/binance-broaduni/` (744 month-parquets: 35 symbols ×
  2024 + 27 symbols × 2023) + its self-contained `data/binance-broaduni/REVISION.toml`
  (aggregate SHA `518b4d40…`). This is a NEW data tree; it does not touch or
  supersede `data/binance`. Keep it banked (it is the pre-positioned fallback for a
  broader TS retest per S.5.2); it can be deleted later if the TS build clears on
  the 10 and the broader axis is never needed — operator's call.
- **Probe:** `crates/data/examples/universe_diag.rs` extended additively with
  optional `--root` + `--symbols` flags (10-symbol large-cap default preserved &
  regression-verified; clippy-clean under `--all-targets -D warnings`). Still a
  disposable read-only diagnostic per its existing cleanup contract.
- **Untouched:** `data/binance/REVISION.toml` (`3a8b96c4…`), `data/yahoo/`,
  `data/binance-funding/`, all `spec/*/reports/` anchors, `crates/ui/`.

---

## Changelog

- 2026-06-02 (analyst, broader-universe spike): ran the § 4.4 confirmation spike.
  Fetched 35 liquid mid-cap USDT pairs (2024-FY) + the 27 with full-2023 history
  (2023-FY) via `fetch_binance_klines` into a SEPARATE `data/binance-broaduni/`
  tree (new pin `518b4d40…`; existing `3a8b96c4…` untouched). Extended
  `universe_diag.rs` additively with `--root`/`--symbols` (10-symbol default
  preserved, regression-checked). Result: broader universe LOWERS avg R²
  0.715→0.598 / avg corr 0.683→0.582 (axis a = YES, universe was a real beta
  headwind) but rank IC stays ≈ 0 with no stable positive sign across two universe
  sizes and two years (axis b = NO). **VERDICT: cross-sectional ranking METHOD is
  the limiter, not the universe** (HIGH confidence). Firmed recommendation: proceed
  to the `time-series-momentum-robustness` build on the ORIGINAL 10-symbol set
  (`data/binance`); the broader tree is a pre-positioned fallback if TS-momentum is
  itself fragile on the 10. The one positive outlier (27-sym 2023 L=720 = +0.088,
  n=11 windows) flagged as a non-replicating noise-floor sample, not buried.
- 2026-06-02 (analyst, universe-method-diagnosis): post-program diagnosis after
  the three-family cross-sectional negative (momentum/MR/carry all dominated by
  BH). Computed from banked OHLCV via the harness's own `ReplayFeed` reader
  (revision `3a8b96c4…`), throwaway `crates/data/examples/universe_diag.rs`:
  avg pairwise corr 0.631 (2023) / 0.683 (2024); avg R² vs EW index 0.667 / 0.715;
  cross-sectional dispersion 0.35% / 0.42% per bar; **cross-sectional rank IC ≈ 0
  at every horizon (±0.07, no stable sign) in both years.** Diagnosis: original
  "~1 factor ceiling" hypothesis PARTIALLY REFUTED (R² 0.67–0.71, not ~1.0; real
  ~30% idiosyncratic cross-section); the BINDING constraint is the **cross-sectional
  ranking method** (rank IC ≈ 0 is signal-agnostic → explains the uniformity of all
  three failures), with high common-beta share as a contributing universe headwind.
  Recommended disambiguator: a TIME-SERIES (per-asset absolute-momentum) experiment
  on the proven block-bootstrap harness (~1.5–3 dev-days + ~1 min compute), gated
  behind a near-free broader-universe diagnostic spike (~0.5 day, no code) that
  re-runs universe_diag.rs on a 30–50-name mid-cap set to read M3/M4. Proposed (not
  greenlit) feature stub `time-series-momentum-robustness` for operator go/no-go;
  `[[req]]` row deliberately deferred until approval per trace.toml ownership rule.
