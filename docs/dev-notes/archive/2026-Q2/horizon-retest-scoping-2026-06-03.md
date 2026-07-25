---
slug: horizon-retest-scoping-2026-06-03
status: draft
owner: analyst
updated: 2026-06-03
tags: [horizon, daily, 4h, resample, sharpe-annualization, block-bootstrap, statistical-power, time-series-momentum, carry, robustness, scoping, go-no-go, post-program-direction]
related:
  - spec/time-series-momentum-robustness/presentations/time-series-momentum-robustness-2026-06-03.md
  - docs/dev-notes/universe-method-diagnosis-2026-06-02.md
  - docs/dev-notes/robustness-decision-rule-2026-05-30.md
  - _bmad-output/planning-artifacts/architecture/decisions/0051-monte-carlo-determinism-and-distribution-report-anchoring.md
  - spec/product.md
---

# Horizon retest — scoping (the untested axis)

> **Mandate (research + scoping, NO build).** The active-strategy robustness
> program returned a uniform negative: four families (cross-sectional momentum,
> mean-reversion, carry, time-series momentum), two method classes, all
> FAMILY-UNIFORM-FRAGILE on the **10-symbol 1h Binance universe**, all dominated
> by passive buy-and-hold. The [universe-method diagnosis](universe-method-diagnosis-2026-06-02.md)
> spiked the universe axis (broader 35-name mid-caps → rank IC still ≈ 0) and named
> the remaining limiter "**universe + horizon**". Every family ran at **1h** —
> the HORIZON is the one untested variable. This note scopes the horizon retest:
> WHICH horizon, WHICH data approach, WHICH families, the experiment design, and
> the cost — with real numbers computed from the repo + the banked 1h data. The
> proposed feature stub at the end is for the operator's go/no-go; no `[[req]]`
> row is created and nothing is built until the operator greenlights.

---

## 0. TL;DR — the recommendation (with confidence)

| Question | Recommendation | Confidence |
|---|---|---|
| **WHICH horizon?** | **Daily (1d) as the headline; 4h as a same-build second grid.** Run **both** off one resample pass. | HIGH on "test daily"; MEDIUM on the power caveat below |
| **DATA approach** | **Deterministically RESAMPLE the banked 1h bars in-memory** (open=first, high=max, low=min, close=last, volume=sum). NO fetch, NO new revision — reuses pin `3a8b96c4…`. | HIGH (resample is clean; ratios are exact integers; UTC-aligned) |
| **WHICH families first** | **FOCUSED first test: TS-momentum + carry** at the chosen horizon (highest horizon-sensitivity priors). Defer x-sec momentum/MR to a second pass. | MEDIUM-HIGH |
| **Decision rule transfer** | **§ 0 Sharpe bands transfer as-is ONLY IF the per-bar→annual Sharpe scalar is corrected** for the coarser bar. The harness hardcodes a **1h-baked** `SQRT_HPY` constant (= √8575) in `compute_sharpe_hourly` / `compute_sortino_hourly`, and `compute_calmar` hardcodes `years = (n−1)/8760`. This is a **required code change**, not a config flag. | HIGH (read straight off the source) |
| **Cost** | **~seconds per surface** (far fewer bars than 1h). The dev-cost is the resample function + the annualization fix + day-1 falsifiers, not compute. | HIGH on compute; the dev-days estimate is in § 5 |

**One-line operator framing:** *"Every strategy we tried traded hour-by-hour.
Trend-following is classically a daily-to-weekly effect, and funding settles every
8 hours — so trading on the SAME coins but deciding once a day (or once every four
hours) is the one knob we never turned. We can turn it for almost free by
re-bucketing the hourly data we already have — but we must first fix one
hour-specific number in the Sharpe formula, or every result will be silently
inflated."*

**The load-bearing power caveat (read before approving daily):** daily gives only
**~365 bars/year**. The block bootstrap resamples *blocks of bars*; with ~365
source returns the resampled paths are far less diverse than at 1h (~8 760), and
the auto block-length (Politis–White) will eat a large fraction of the series per
block. Daily is still worth testing (it is the highest-prior horizon for trend),
but the p5/p95 tail estimates will be **noisier** and the FRAGILE/MARGINAL boundary
must be read with more latitude (exactly the § 6 latitude the decision rule already
reserves for small N). **4h (~2 190 bars/year) is the statistical-power sweet
spot** — coarse enough to test the horizon thesis, fine enough that the bootstrap
still has real resampling diversity. This is why the recommendation is *both*, off
one resample pass: daily for the strongest prior, 4h as the powered cross-check.

---

## 1. Evidence — bar counts (computed, not asserted)

The banked 1h data is `data/binance/` (10 large-cap USDT pairs, pin
`3a8b96c43f2d…`), monthly parquet at `<SYM>/<YEAR>/<MM>.parquet`, `interval=1h`
(REVISION.toml metadata). The sweep's `main()` hardcodes the per-symbol 1h bar
count by year (`param_robustness_sweep.rs:2149`): **2023 → 8 760**, **2024 →
8 784** (leap year, 366×24). These match the calendar exactly (365×24 = 8 760;
366×24 = 8 784), and the universe-diagnosis run confirmed the 10-name set is
essentially gap-free (8 758 / 8 783 aligned *returns* after the timestamp
intersection trims the two boundary bars).

Resampling the 1h grid (the ratios are **exact integers** — this is the clean-cut
that makes the resample deterministic and boundary-safe):

| Horizon | Bars/symbol/year (2023) | Bars/symbol/year (2024) | Ratio from 1h | Block-bootstrap power |
|---|---:|---:|---:|---|
| **1h** (current) | 8 760 | 8 784 | 1:1 | strong (baseline) |
| **4h** | **1 460** | **1 464** | 6:1 | **good — recommended power sweet spot** |
| **1d** | **365** | **366** | 24:1 | thin — testable but noisier tails |

`8760/6 = 1460.0`, `8784/6 = 1464.0`, `8760/24 = 365.0`, `8784/24 = 366.0` — all
exact, because 1h divides 4h and 1d cleanly on Binance's UTC-aligned grid.

### 1.1 Why bar count gates bootstrap power (the mechanism)

The stationary block bootstrap (`crates/data/src/synth/bootstrap.rs`,
Politis–Romano 1994) builds `T−1` source log-returns from `T` source bars, then
draws geometric-length blocks (mean length L via Politis–White on the
universe-average |return| series) and re-stitches them into each path. Two
consequences of a small `T`:

1. **Resampling diversity collapses.** With `T ≈ 365` daily returns, there are
   far fewer distinct blocks to recombine than with `T ≈ 8 760` hourly returns.
   The N=200 resampled paths overlap each other much more, so the *effective*
   sample behind the p5/p95 tail is smaller than N suggests — the tail estimate
   is noisier even at the same N.
2. **The auto block length swallows the series.** L is chosen on the return
   series; daily crypto returns are more autocorrelated per bar than hourly, so
   the Politis–White L can be a non-trivial fraction of 365 — meaning each path is
   a stitch of only a handful of blocks. (4h at ~1 460–1 464 bars keeps L a small
   fraction of the series, preserving block diversity.)

This is a *power* concern, not a *validity* concern: a daily bootstrap is still a
valid fair adversary (the shared-index co-movement guard, FP-C1.5, is horizon-
independent — it operates on whatever return matrix it is handed). It just has
wider error bars. **Mitigation if daily is run: bump N for the daily grid** (e.g.
N=500 or N=1000 — cheap, see § 5 cost) to claw back tail stability, and read the
daily verdict with the § 6 small-N latitude the decision rule already grants.

---

## 2. DATA approach — resample, do NOT re-fetch (HIGH confidence)

**A deterministic in-memory resample of the banked 1h bars is feasible, clean, and
strongly preferred over a fresh fetch.** It reuses pin `3a8b96c4…` (no new
REVISION.toml, no new anchor-namespace data dependency) and is bit-for-bit
reproducible.

### 2.1 The resample is clean (boundary + gap analysis)

- **OHLCV aggregation** is the standard bar-rollup: for each coarse bucket,
  `open = first 1h open`, `high = max 1h high`, `low = min 1h low`,
  `close = last 1h close`, `volume = Σ 1h volume`. `trade_count = Σ` (or drop —
  not load-bearing for the strategy, which reads `close`).
- **UTC-day alignment is exact.** Binance 1h bars open at `HH:00:00 UTC`
  (`open_time` is Unix-ms on the hour). 1h→1d buckets by
  `floor(open_time_ms / 86_400_000)` → each daily bar spans `00:00…23:00 UTC`
  (24 source bars). 1h→4h buckets by `floor(open_time_ms / 14_400_000)` → 4h bars
  align to `00/04/08/12/16/20 UTC` (6 source bars). Because the year starts at
  `YYYY-01-01T00:00:00Z` (the `TimeSpan::full_year` boundary in `realdata.rs`),
  the first bucket is full and aligned — no partial leading bucket.
- **Gap handling.** The 10-name set is gap-free to the 99.5% tolerance the loader
  already enforces (`RealDataError::MissingData`, ≥ 99.5% present). Exact integer
  ratios (24:1, 6:1) mean every bucket has its full complement of source bars; a
  defensive resample should still aggregate whatever bars fall in the bucket (so a
  rare missing 1h bar degrades that bucket's volume, not the bucket boundary). The
  intersection grid the universe-diag already uses (`universe_diag.rs:141`) is the
  proven pattern for cross-symbol alignment.

### 2.2 Where the resample plugs in (single integration point)

The 1h coupling is **localized**: `merge_symbols(&symbol_paths, Timeframe::OneHour)`
in `realdata.rs:227` (and the `Timeframe::OneHour` stamp in `read_parquet_bars`).
The parquet reader reads the **actual** `open_time`/`close_time` from the file
content — the `Timeframe` argument is metadata stamped onto each `Bar`, NOT a
re-bucketing instruction. So a resample is a post-load fold over the merged 1h
`Vec<Bar>`, producing a coarser `Vec<Bar>` stamped `Timeframe::FourHours` /
`Timeframe::OneDay` (**both variants already exist** in `crates/core/src/bar.rs`
— no new enum variant needed). The bootstrap consumes whatever bars it is handed
(it computes returns from `close`), so feeding it resampled bars is transparent to
the resampling math.

> **Note on the bootstrap's synthetic timestamps:** `bootstrap.rs` stamps its
> *output* bars with `Timeframe::OneHour` and `Duration::hours(i)` regardless of
> source cadence (the synthetic-path timestamps are a 1h ladder from a 2023 epoch).
> This is **cosmetic** for the strategy (which keys off `close` and bar ordering,
> not wall-clock spacing) and for the per-bar Sharpe (which is per-return, not
> per-wall-second). It does NOT need to change for the horizon retest — but the
> developer should be aware the output `tf` field will read `OneHour` on a daily
> path unless they also thread the resampled `tf` through `synthetic_bars`/the
> bootstrap timestamp ladder (a legibility nicety, not a correctness requirement).
> Flag for the architect to rule on (OQ-2 below).

### 2.3 Carry's funding data resamples for free

The carry signal reads funding via an **as-of join**: `funding_data.rs:386`
binary-searches the rightmost settlement `≤ bar_ts` (8h settlement cadence, keyed
on Binance `fundingTime`, banked at `data/binance-funding/` pin `bf1ede44…`).
This join is **timestamp-driven, not bar-index-driven** — it attaches the
in-force funding rate to *whatever* bar timestamps exist. So a 4h or daily bar
grid reuses the exact same funding parquets via the same as-of lookup, no funding
re-fetch and no funding-resample. (At a daily horizon each bar spans three 8h
settlements; the as-of join naturally takes the most-recent one at the bar's
open_ts — the architect should confirm whether carry wants "last settlement" vs
"trailing-mean over the bar's settlements", OQ-4 below.)

---

## 3. Sharpe-annualization — the ONE required code change (HIGH confidence)

**This is the load-bearing methodological finding and the gate on whether the § 0
decision rule transfers.** The harness's Sharpe is computed **per-bar** and
annualized by a **hardcoded constant that assumes 1 bar = 1 hour**. At a coarser
horizon the same constant silently **inflates** the annualized Sharpe by a fixed
multiplicative factor.

### 3.1 The exact constants (read off `crates/backtest/src/stats/mod.rs`)

- `compute_sharpe_hourly` (line 40) and `compute_sortino_hourly` (line 70):
  ```
  const SQRT_HPY: f64 = 92.601_295_098_46;   // = √8575 (92.601295² = 8574.9999)
  sharpe = mean_log_return / std_log_return * SQRT_HPY
  ```
  `SQRT_HPY² = 8575.0` — i.e. the harness annualizes as if a year is **8 575
  hourly periods** (a hand-entered ≈√(23.5·365); close to but not exactly √8760).
  The exact origin is immaterial; what matters is it is a **fixed per-bar→annual
  scalar baked for 1h cadence**.
- `compute_calmar` (line 111): `years = (n − 1) / 8760.0` — a *separate* 1h-baked
  constant for CAGR.

### 3.2 The correct per-horizon scalars

The annualization factor is `√(periods_per_year)`:

| Horizon | periods/year | Correct √(periods/year) | Harness uses (1h const) | Inflation if uncorrected |
|---|---:|---:|---:|---:|
| 1h | 8 760 | 93.595 (√8760) | 92.601 (√8575) | — (the baseline; ~0.5% low by design) |
| **4h** | **2 190** | **46.797** (√2190) | 92.601 | **≈ 2.0×** (Sharpe doubled) |
| **1d** | **365** | **19.105** (√365) | 92.601 | **≈ 4.9×** (Sharpe ~5× too big) |

If the harness ran daily bars through the unmodified `compute_sharpe_hourly`, a
true Sharpe of 0.04 would print as ≈0.20, and a true 0.3 as ≈1.5 — which would
**spuriously clear the § 0 ROBUST bands**. This is precisely the class of silent
error the project's fabricated-"Sharpe 1.40" precedent and the
robustness-decision-rule pre-registration exist to prevent. **It must be fixed
before any horizon surface is scored.**

### 3.3 The fix (architect to lock the exact form)

Parameterize the annualization by bars-per-year (or by a `Timeframe`/cadence
argument), replacing the two hardcoded `SQRT_HPY` constants and the `8760.0`
Calmar divisor:
```
periods_per_year(tf) = { OneHour: 8760, FourHours: 2190, OneDay: 365 }   // 8784/2196/366 on leap years
sharpe = mean / std * sqrt(periods_per_year(tf))
calmar_years = (n − 1) / periods_per_year(tf)
```
**Anchor impact (must be flagged to the architect):** `compute_sharpe_hourly` is
a **verbatim-lifted, anchor-load-bearing** calculator (R-NR.5: re-imported by
`bin/threshold_sweep.rs` so its body-SHA stays byte-identical, and it feeds the 91
locked surfaces). The horizon work MUST NOT mutate the 1h path's output. The
clean approach is an **additive horizon-aware variant** (e.g.
`compute_sharpe(equity, periods_per_year)` with `compute_sharpe_hourly` retained
as a `periods_per_year = 8760`-or-8575 wrapper) so the existing 1h anchors are
byte-unchanged by construction — the same defaults-off / additive discipline every
prior family used. The leap-year subtlety (2024 = 8 784 / 2 196 / 366) should be
threaded too, since the sweep already special-cases 2024.

> **Decision-rule § 0 transfer verdict:** the seven Sharpe/drawdown read-bands
> transfer **as-is** to a lower frequency *once the annualization is horizon-
> correct*. The bands are dimensionless properties of the annualized distribution
> (p5<0 = tail loses money; p95-MaxDD; prob-of-loss; P(Sharpe>1)); none is
> hour-specific EXCEPT through the annualization scalar. Fix the scalar and the
> ruler is valid at 4h/daily with no band changes. **prob-of-loss, P(Sharpe>0/>1)
> counts, and MaxDD are annualization-invariant** (MaxDD is a path property; the
> probability counts are on raw final-equity and on the *sign-preserving* Sharpe,
> which the scalar does not flip) — so even those are safe. Only the *magnitude*
> bands (p5/p50 Sharpe ≥ thresholds) depend on the scalar being right.

---

## 4. WHICH families — focused first test: TS-momentum + carry (MEDIUM-HIGH)

The horizon-sensitivity priors are not uniform across the four families. Ranked by
"how likely is a coarser horizon to change the verdict":

| Family | Horizon-sensitivity prior | Rationale |
|---|---|---|
| **Time-series momentum** | **HIGHEST** | Trend-following is classically a daily/weekly effect (the canonical TSMOM literature is monthly/daily). At 1h it was whipsaw-dominated (MaxDD_p95 88–97%); a daily decision cadence is the textbook home for absolute momentum and directly attacks the whipsaw diagnosis. |
| **Carry / funding** | **HIGH** | Funding settles every **8h**. A 1h rebalance over-trades relative to the settlement cadence; a 4h or daily horizon **aligns the decision frequency with the settlement frequency** (8h ≈ 2× a 4h bar, ≈⅓ a daily bar) — the natural cadence to harvest a settlement-paid premium without churn. The carry 1h grid already used 8h/24h rebalance overrides *on top of* 1h bars; a 4h/daily bar makes that native. |
| Cross-sectional momentum | LOW | The diagnosis killed this via rank IC ≈ 0 at **every** horizon (3h…30d, both years, both universe sizes). A coarser bar does not revive a dead ranking channel — rank IC was already computed at daily-equivalent lookbacks (L=24 = 1d) and stayed ≈ 0. |
| Cross-sectional mean-reversion | LOW | Same dead ranking channel; the faint structure was at the *shortest* (3h) horizon (reversal-flavoured), which a coarser bar moves *away* from. |

**Recommendation: run TS-momentum + carry FIRST** at the chosen horizon(s).
Rationale (durable-over-quick): these two carry the entire horizon-sensitivity
prior; testing them first answers the load-bearing question ("does *any* method
clear BH at a coarser horizon?") for the least compute and the least new code,
and a clean negative on the two highest-prior families would be strong evidence
the horizon axis is also exhausted. Spending the first pass on x-sec momentum/MR —
which the rank-IC evidence already predicts will fail at any horizon — would be the
"build-then-discover-the-channel-is-dead" rework the durable-first rule exists to
prevent.

**Cost trade-off vs all four:** the x-sec momentum/MR grids are *already wired*
(`--grid tier1` / `mr-tier1`, `--direction`), so adding them to a second pass is
cheap *compute* (~seconds). But each additional family-year surface is another
anchored body to lock + falsifier review + a presenter line — the cost is review
surface, not CPU. Defer them to a fast follow-on *iff* the TS/carry first pass
shows any non-fragile cell (which would make "does the horizon revive the ranking
channel too?" worth the extra surfaces). If TS+carry are uniform-fragile at the
coarser horizon, the program can close the horizon axis without re-running the two
families the diagnosis already exonerated.

---

## 5. Experiment design

### 5.1 θ-grids — re-pick lookbacks in BARS (the critical scaling fix)

**`lookback_minutes` is interpreted as a BAR COUNT, not wall-clock minutes**
(`crates/strategy/src/cross_sectional/config.rs:137`: *"Lookback window in bars
(≥ 1)"*; line 139: *"Rebalance cadence in bars (≥ 1)"*). The field name is a
historical misnomer. Therefore a θ-grid lookback does NOT auto-scale with the
horizon — the SAME number means a 6×- or 24×-longer wall-clock window at a coarser
bar:

| Lookback value (bars) | Wall-clock @1h | Wall-clock @4h | Wall-clock @1d |
|---:|---|---|---|
| 24 | 1 day | 4 days | 24 days |
| 168 | 1 week | 4 weeks | 24 weeks |
| 720 | 30 days | 120 days | **2 years (> data!)** |

The 1h TS grid's `{24, 168, 720}` bars = `{1d, 1wk, 30d}`. To preserve the SAME
*wall-clock* hypothesis space at a coarser horizon, re-pick the bar counts:

- **Daily (1d) grid (suggested, architect to lock):** lookbacks in *days* —
  e.g. `{5, 20, 60}` bars = `{1wk, 1mo, 1qtr}`, the classic TSMOM windows. (720
  bars at daily = 2 years, larger than the 365-bar series — **must not** carry the
  1h grid over verbatim; this is a correctness bound, not a preference.)
- **4h grid (suggested):** lookbacks in 4h-bars — e.g. `{42, 180, 540}` bars =
  `{1wk, 30d, 90d}` to mirror the 1h grid's wall-clock span at the new resolution.
- **Carry:** the carry grid's `lookback_minutes` encodes **L = funding
  settlements** (not bars), and `rebalance_minutes_override` is in **minutes of
  wall-clock**. At a coarser bar the rebalance override should map to the bar
  cadence (e.g. native 4h/daily rebalance) and L stays in settlement units —
  architect to confirm the carry grid's L + rebalance semantics under resampling
  (OQ-4).
- **Keep the grid 6 cells** (the proven shape; one body-SHA per surface) spanning
  the same axes (lookback × entry_threshold for TS; L × rebalance × k_long for
  carry), just re-centred on horizon-appropriate wall-clock windows.

### 5.2 N (paths)

- **4h: N=200** (the proven default; ~1 460 bars keeps bootstrap diversity high).
- **Daily: N=500 or N=1000** to offset the thin 365-bar series (§ 1.1). Compute is
  trivial at daily (§ 5.4), so the larger N is nearly free and directly buys back
  tail stability. Architect to lock N per § 0 of the decision rule (N is a hashed
  body field — it is part of each anchor's identity).

### 5.3 Buy-and-hold bar — recomputed at the new frequency automatically

The BH control (`run_buyhold_path`, `param_robustness_sweep.rs:1042`) buys
equal-weight at bar 0 and marks-to-market each bar — it is **horizon-agnostic by
construction** (it consumes whatever bars the path has). Its *Sharpe* is then
annualized by the same (now horizon-corrected) scalar, so the BH bar is
recomputed correctly at 4h/daily with no special handling beyond the § 3 fix.
**Sanity gate:** BH total-return over the year is horizon-invariant (same start/end
prices), so the resampled BH *total return* must match the 1h BH total return to
rounding — a cheap correctness assertion for the developer's day-1 test. (BH
*Sharpe* and *MaxDD* WILL differ across horizons — fewer, larger bars → different
per-bar vol and a coarser drawdown path — and that is expected, not a bug.)

### 5.4 Cost — grid × N × per-path (HIGH confidence on compute)

The 1h 6×200 sweeps ran **~35 s** (TS surfaces: 34.6 s / 35.6 s; carry: 30.7 s /
28.4 s) — committed numbers. Per-path cost scales ~linearly with bar count
(generate + replay + metric the bars), so a coarser horizon is **much faster per
path**:

| Horizon | Bars/path | Approx per-surface (6 cells) | At larger N |
|---|---:|---|---|
| 1h (reference) | ~8 760 | ~35 s @ N=200 | — |
| **4h** | ~1 460 (⅙) | **~6 s @ N=200** (≈35 s ÷ 6) | — |
| **1d** | ~365 (1/24) | **~1.5 s @ N=200** (≈35 s ÷ 24) | **~4–8 s @ N=500–1000** |

**Total for the focused first test** (TS + carry, 2 years each, daily + 4h = 8
surfaces): on the order of **~1 minute of compute, end to end.** Compute is a
rounding error. **The real cost is dev-time:** the resample function (~0.5 day),
the additive horizon-aware annualization fix + its anchor-neutrality test (~0.5–1
day), the re-picked θ-grids + the day-1 baseline-divergence e2e per the CLAUDE.md
non-negotiable (~0.5–1 day), and 8 anchored surfaces + falsifier review. Estimate
**~2–3 dev-days** for the focused TS+carry first pass (by analogy to the carry/TS
builds, minus the new-strategy work — the strategies already exist; this is a
horizon/data-path change, not a new family).

> **Watch recipe (for the developer's eventual N=500 daily or any >2-min job —
> copy-paste to the operator terminal):**
> ```bash
> watch -n 10 '
> PID=$(pgrep -f param_robustness_sweep | head -1)
> [ -z "$PID" ] && echo "param_robustness_sweep not running" && exit
> N=$(ls /tmp/horizon-verify/robustness-sweep-*.md 2>/dev/null | wc -l | tr -d " ")
> ELAPSED=$(ps -o etime= -p "$PID" 2>/dev/null | tr -d " ")
> echo "surfaces landed: ${N}; elapsed ${ELAPSED}"
> '
> ```
> (At ~6 s/surface for 4h this will mostly show the build, not the run — the watch
> matters only for the larger-N daily grids or a full four-family sweep.)

### 5.5 Science gate (carry forward, unchanged)

Every horizon surface must keep the proven gates: `generator: block-bootstrap-real`
+ `bootstrap_mode: shared-index` (the FP-C1.5 fair-adversary guard is horizon-
independent), the data revision pin in the body, two-run byte-determinism on the
canonical box, and the day-1 baseline-equity-divergence e2e (≥ 1 bp vs un-traded
baseline) per the v3-vol-overlay no-op precedent. The decision rule is **pre-
registered against § 0 verbatim** BEFORE any horizon run, exactly as all four 1h
families were — and the § 3 annualization fix is itself a pre-registration item
(the corrected scalar is fixed first, the numbers scored against it).

---

## 6. Assumptions & limits (challengeable by architect/operator)

1. **Resample = simple OHLCV rollup on the UTC grid.** Exact integer ratios (24:1,
   6:1) and on-the-hour Binance opens make this boundary-safe; no interpolation,
   no fractional buckets. If the architect prefers a calendar-aware resampler
   (e.g. session boundaries) that is overkill for UTC-continuous crypto.
2. **The horizon does not revive the dead ranking channel** (x-sec momentum/MR) —
   rank IC was already ≈ 0 at daily-equivalent lookbacks (L=24) both years, both
   universes. This is why x-sec is deprioritized; if the operator wants the full
   four-family horizon sweep for completeness, it is cheap compute but more review
   surface (§ 4).
3. **Daily has real but acceptable power loss.** 365 bars is thin for a block
   bootstrap; the mitigation is larger N + § 6-latitude reading, not abandoning
   daily (it is the highest-prior horizon for trend). 4h is the powered cross-
   check. If the operator wants ONE horizon only, the durable choice is **daily**
   (highest prior, classic TSMOM home) with N=1000; the cheaper/safer-power choice
   is **4h** with N=200. The recommendation runs both because the resample pass is
   shared and the compute delta is seconds.
4. **The annualization fix is additive and anchor-neutral.** If the architect finds
   a cleaner refactor that also touches the 1h path, the 91 anchors gate it — any
   1h body-SHA change is a REGRESSION and blocks the build (CLAUDE.md non-
   negotiable). The safe path is a new horizon-aware fn + the existing
   `compute_sharpe_hourly` retained verbatim as the 1h wrapper.
5. **Block-bootstrap cannot synthesize regimes the source year lacks** (decision-
   rule § 5 scope bound) — a horizon change re-buckets the SAME 2023/2024 returns;
   it does not add new market history. The horizon retest asks "does a coarser
   decision cadence on the same history beat BH?", not "what happens in an unseen
   regime."
6. **Open questions for the architect** (OQ-1 the exact horizon-aware
   annualization signature + leap-year handling; OQ-2 whether to thread the
   resampled `tf` through the bootstrap timestamp ladder or leave it cosmetically
   1h; OQ-3 the exact re-picked θ-grid bar counts per horizon; OQ-4 carry's L +
   rebalance + as-of-funding semantics under a coarser bar; OQ-5 N per horizon).

---

## 7. Proposed next feature (STUB — for operator go/no-go; NOT greenlit, NO build)

> Per the trace.toml ownership rule, the analyst creates the `[[req]]` row when a
> feature enters `proposed`. **This stub is deliberately NOT yet committed as a
> `proposed` row** — the operator's go/no-go decides whether it is greenlit. On
> approval, the analyst creates the row + `spec/<slug>/feature.md`.

**Slug (proposed):** `horizon-retest-robustness`

**Why:** All four families are FAMILY-UNIFORM-FRAGILE at **1h**; the diagnosis
named "universe + horizon" the binding limiter and the universe spike exonerated
"just broaden the basket". The horizon is the one untested axis. Trend-following is
classically a daily/weekly effect and funding settles 8-hourly, so a coarser
decision cadence on the SAME coins is the highest-prior untried knob — and it is
testable for ~seconds of compute by resampling the banked 1h data (pin
`3a8b96c4…`), provided the 1h-baked Sharpe annualization is first corrected.

**Scope (minimal viable, focused first pass):**
- A deterministic **1h→{4h, 1d} OHLCV resampler** (in-memory fold over the merged
  `Vec<Bar>`; open=first/high=max/low=min/close=last/volume=Σ; UTC-bucketed by
  integer division; reuses pin `3a8b96c4…`, no fetch, no new revision).
- An **additive horizon-aware Sharpe/Sortino/Calmar annualization** (parameterized
  by bars-per-year / `Timeframe`), with `compute_sharpe_hourly` retained verbatim
  as the 1h wrapper so the **91 existing anchors are byte-unchanged**.
- A `--horizon {1h,4h,1d}` (or `--resample`) arm in `param_robustness_sweep`
  wiring the resampler + the correct annualization + a re-picked horizon-
  appropriate θ-grid (lookbacks in BARS, re-centred on sensible wall-clock
  windows) for **TS-momentum + carry**.
- **Day-1 mandated gates:** a baseline-equity-divergence e2e (≥ 1 bp) per the
  CLAUDE.md non-negotiable; a resample-correctness test (BH total-return invariant
  across horizons; exact bucket counts 1460/365 etc.); an annualization-correctness
  unit test (4h scalar = √2190, daily = √365); two-run byte-determinism.
- **Anchored surfaces** under a new namespace `horizon-retest-robustness`, scored
  against the frozen § 0 decision rule (pre-registered, with the corrected scalar
  fixed first).

**Pre-condition / sequencing:** the § 3 annualization fix is the **gate** — no
horizon surface is scored until the corrected scalar is in and its anchor-
neutrality (1h byte-identity) is proven.

**Expected cost:** **~2–3 dev-days + ~1 min compute** for the focused TS+carry
first pass (daily + 4h, 2 years; 8 surfaces). x-sec momentum/MR are a cheap-compute
follow-on (~seconds) *iff* the first pass shows any non-fragile cell.

**Expected outcome (honest prior):** **MEDIUM** for a non-fragile TS-momentum cell
at daily (the textbook TSMOM home; the 1h whipsaw diagnosis is exactly what a
daily cadence attacks) — this is the single most-likely-to-flip experiment left.
**LOW-MEDIUM** for carry (the 8h-settlement alignment is a real structural
argument, but the 1h carry grid already used 8h/24h rebalance overrides and still
failed). **LOW** for x-sec at any horizon (dead ranking channel). Either way the
result is decision-grade: a fragile result at the highest-prior horizon would
**close the horizon axis** and, with the universe already exonerated, would
effectively exhaust the OHLCV-only active-strategy search on this data — routing
the program to the deck's fork (b) different data domain / (c) productionize the
proven stack.

---

## Changelog

- 2026-06-03 (analyst, horizon-retest-scoping): scoped the horizon axis — the one
  untested variable after the 4-family 1h uniform-negative. Computed real bar
  counts from the banked 1h data + sweep source (2023=8 760, 2024=8 784;
  resample-exact 4h=1 460/1 464, 1d=365/366). Verified the resample is clean
  (integer 6:1/24:1 ratios, UTC-aligned, both `Timeframe::FourHours`/`OneDay`
  already exist; single integration point at `merge_symbols(...,OneHour)`; funding
  as-of join resamples for free). **Found the load-bearing annualization defect:**
  `compute_sharpe_hourly`/`compute_sortino_hourly` hardcode `SQRT_HPY=√8575` and
  `compute_calmar` hardcodes `years=(n−1)/8760` — 1h-baked, would inflate Sharpe
  ≈2.0× at 4h / ≈4.9× at daily; the § 0 decision rule transfers as-is ONLY after
  an additive horizon-aware annualization fix (anchor-neutral — 91 anchors must
  stay byte-identical). Recommendation: **daily (headline) + 4h (powered cross-
  check) off one resample pass; TS-momentum + carry first; resample not re-fetch;
  N=500–1000 for daily to offset the thin 365-bar series; ~2–3 dev-days + ~1 min
  compute.** Flagged `lookback_minutes` is interpreted as BARS not minutes →
  θ-grids must be re-picked per horizon (720 bars at daily = 2yr > data). Proposed
  (not greenlit) feature stub `horizon-retest-robustness` for operator go/no-go;
  `[[req]]` row deferred until approval per trace.toml ownership rule.
</content>
</invoke>
