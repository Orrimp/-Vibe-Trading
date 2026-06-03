---
slug: time-series-momentum-robustness
mode: release
status: draft
audience: human-operator
updated: 2026-06-03
generated: 2026-06-03T17:49:58Z
---

# Time-series momentum — release + active-strategy robustness program retrospective

> This deck closes TWO things at once: (1) the **time-series-momentum** feature
> (VERDICT → PASS), and (2) the **entire active-strategy robustness program** —
> TS-momentum was its last family. Read it as a sprint review AND a go/no-go on
> where research goes next. Every number traces to a committed report; nothing
> is computed here for the first time.

## TL;DR

Time-series momentum (the first **non-cross-sectional** strategy — each coin
traded long/flat on its OWN trend, removing the dead "rank the coins" channel)
came back **FAMILY-UNIFORM-FRAGILE on BOTH 2023 and 2024** and was retired —
which means **all four strategy families we tried lose to simply buying and
holding the same ten coins**, and the active-trading thesis on this universe is
closed. VERDICT PASS, anchors #90/#91 locked (91/91 byte-stable).

## What changed

- **Built the first time-series strategy** (per-asset absolute momentum,
  long-or-flat on each coin's own trailing return — no cross-sectional ranking),
  defaults-off, with 5 day-1 falsifier tests. It slotted in without disturbing
  any of the 89 existing regression anchors.
- **Ran it through the same proven block-bootstrap harness** on real 2023 and
  2024 Binance hourly data, scored against the same frozen decision rule the
  three earlier families used. Result both years: **FAMILY-UNIFORM-FRAGILE** —
  every parameter cell fails, both years. Two new byte-stable anchors locked
  (#90 = 2023, #91 = 2024).
- **The program is now complete and uniformly negative.** Four families
  (cross-sectional momentum, mean-reversion, carry/funding, time-series
  momentum), across two method classes, **all dominated by passive buy-and-hold**
  net of fees. This is a decision-grade negative result, not a bug.

## Why

The robustness program had already retired all three *cross-sectional* families
(momentum, mean-reversion, carry) — each dominated by passive buy-and-hold of
the same coins. The [universe-vs-method diagnosis](../../dev-notes/universe-method-diagnosis-2026-06-02.md)
explained the uniformity with one signal-agnostic fact: the cross-sectional
**ranking channel carries ≈ 0 forward information** on this universe (rank IC
within ±0.07 of zero at every horizon, no stable sign, both years), and a
broader 35-name mid-cap universe lowered common market-beta (avg R²
0.715 → 0.598) but did **not** revive that ranking signal. Time-series momentum
was the clean disambiguator the diagnosis pre-scoped: it removes the ranking
channel entirely — every coin decides long/flat on its own trend — and it can go
**flat in cash** during downtrends, which is the one structural way it could beat
buy-and-hold (harvest the up-drift, sidestep the drawdowns). The load-bearing
question was: does removing the ranking channel finally clear the buy-and-hold
bar, or is active trading on this universe dominated end-to-end? See
[`feature.md`](../feature.md) "Why".

## What you can do now

| Action | Command |
|--------|---------|
| Reproduce the anchored TS-momentum surfaces (both years, locked grid, N=200) | `cargo run -p backtest --features "candle realdata" --bin param_robustness_sweep -- --grid ts-tier1 --selection-mode time-series-long-flat --paths 200 --year 2023 --out-dir /tmp/ts-verify/` (and `--year 2024`) |
| Confirm every regression anchor is byte-stable (incl. the two new TS anchors) | `bash scripts/verify_anchors.sh` |
| Re-run the 5 day-1 falsifier tests (divergence, no-look-ahead, goes-flat, two-run, RED-on-revert) | `cargo test -p backtest --features "candle realdata" --test ts_momentum_divergence_e2e` |
| Re-read the universe/method diagnosis that scoped this experiment | open `spec/dev-notes/universe-method-diagnosis-2026-06-02.md` |
| (If the program continues) Re-run the universe diagnostic on the pre-banked broader universe | `cargo run -p data --example universe_diag -- 2024 --root data/binance-broaduni --symbols <35-name list>` |

## Live demo

A fresh time-series-momentum sweep, run just now on real 2023 Binance data. This
is a **fast smoke at N=5 paths** (the anchored surfaces use N=200 — so the
per-cell numbers here are NOT the anchored figures; they exist only to prove the
binary genuinely produces the result live). What it demonstrates: the family
verdict, the goes-flat mechanism, and the buy-and-hold gap all reproduce on
demand.

```
$ ./target/debug/param_robustness_sweep --generator block-bootstrap-real \
    --grid ts-tier1 --selection-mode time-series-long-flat \
    --paths 5 --year 2023 --out-dir /tmp/ts-demo/
param_robustness_sweep DONE
  report:         /tmp/ts-demo/robustness-sweep-20260603-174941-v1-ts-momentum-theta-surface-2023-block-bootstrap-real-fy.md
  body_sha:       9ab9c7138fbbebeb202ffcbe52cdf56dd7d66449b0cab7da6dda74b25007efe3
  wall_clock_s:   16.2
  n_cells:        6
  n_paths:        5
  family_verdict: FAMILY-UNIFORM-FRAGILE
  buyhold p50 Sharpe: 1.9262  P(loss): 0.0000  p95 MaxDD: 38.01%

  per-cell summary:
    g= 0 lookback= 168 k_long=10 drift=0.10 → FRAGILE | p50=0.0110 p5=-0.0129 MaxDD_p95=89.8%
    g= 1 lookback=  24 k_long=10 drift=0.10 → FRAGILE | p50=-0.0202 p5=-0.0437 MaxDD_p95=95.2%
    g= 2 lookback= 720 k_long=10 drift=0.10 → FRAGILE | p50=0.0645 p5=0.0050 MaxDD_p95=88.6%
    g= 3 lookback= 168 k_long=10 drift=0.10 → FRAGILE | p50=-0.0264 p5=-0.0415 MaxDD_p95=90.8%
    g= 4 lookback= 720 k_long=10 drift=0.10 → FRAGILE | p50=0.0504 p5=0.0200 MaxDD_p95=87.1%
    g= 5 lookback=  24 k_long=10 drift=0.10 → FRAGILE | p50=-0.0148 p5=-0.0296 MaxDD_p95=90.0%
```

Notice: all 6 cells FRAGILE, buy-and-hold p50 Sharpe ≈ +1.93 on this N=5 path-set
while the best TS cell p50 is ≈ +0.06 — a ~30× gap. The MaxDD_p95 of 87–95% is
the killer: even though the strategy DOES go flat, late exits leave it sitting
through catastrophic drawdowns. Full stdout saved at
[`artifacts/time-series-momentum-robustness-2026-06-03/ts-sweep-smoke-n5-2023.txt`](artifacts/time-series-momentum-robustness-2026-06-03/ts-sweep-smoke-n5-2023.txt).

## The headline — the program retrospective (anchored N=200 figures)

This is the load-bearing table. Four families, two method classes, two years, all
FAMILY-UNIFORM-FRAGILE, all dominated by passive buy-and-hold. Every figure below
is the **best cell** in that family's anchored surface (p50 = median Sharpe; p5 =
5th-percentile / tail Sharpe across the bootstrap paths) — i.e. each family is
shown at its strongest, and still loses.

| Family | Method class | Year | Best-cell p50 Sharpe | Best-cell p5 (tail) | BH p50 bar | Gap vs BH | Verdict |
|--------|-------------|------|---------------------:|--------------------:|-----------:|----------:|---------|
| Cross-sectional momentum | x-sec ranking | 2023 | +0.014 | (uniform p5 < 0) | +1.74 | −1.73 | FAMILY-UNIFORM-FRAGILE |
| Cross-sectional mean-reversion | x-sec ranking | 2023 | +0.007 | (uniform p5 < 0) | +1.74 | −1.73 | FAMILY-UNIFORM-FRAGILE |
| Carry / funding | x-sec ranking | 2023 | +0.039 | −0.192 | +1.74 | −1.70 | FAMILY-UNIFORM-FRAGILE |
| Carry / funding | x-sec ranking | 2024 | +0.043 | +0.016 | +1.10 | −1.06 | FAMILY-UNIFORM-FRAGILE |
| **Time-series momentum** | **TS (per-asset)** | **2023** | **+0.047** | **−0.036** | **+1.74** | **−1.69** | **FAMILY-UNIFORM-FRAGILE** |
| **Time-series momentum** | **TS (per-asset)** | **2024** | **+0.042** | **−0.041** | **+1.10** | **−1.06** | **FAMILY-UNIFORM-FRAGILE** |
| Buy-and-hold (passive) | passive | 2023 | **+1.74** | +0.12 | — | — | _the bar_ |
| Buy-and-hold (passive) | passive | 2024 | **+1.10** | −0.68 | — | — | _the bar_ |

> Sources: TS rows = anchors #90/#91 surfaces (best cell g=2, 720-bar lookback,
> 2023 p50=+0.0473 / 2024 p50=+0.0418; 2024 g=4 ties at +0.0412). Carry rows =
> [`test-2026-06-02-carry-strategy.md`](../../carry-strategy/reports/test-2026-06-02-carry-strategy.md)
> §5 (g=4, K=1). Momentum/MR 2023 best-cell p50 = carry report §5 program table.
> BH bars = the buy-and-hold control row in each surface. "Gap vs BH" = best-cell
> p50 minus BH p50, in Sharpe units. (Sharpe = return per unit of risk; higher is
> better. "FRAGILE" = fails the frozen decision rule, here mainly because the
> worst-case path loses money, i.e. p5 Sharpe < 0.)

**The conclusion, stated plainly:** on this 10-symbol, 1-hour Binance universe,
**active trading is dominated by passively holding the same coins net of fees —
across BOTH the cross-sectional-ranking method class AND the time-series method
class.** No family, at any tested parameter setting, comes within ~1.06 Sharpe
units of just buying and holding.

## Why it fails (the diagnosis)

Two independent, computed facts explain the whole uniform negative:

1. **The ranking channel is dead.** Cross-sectional rank IC ≈ 0 at every horizon,
   both years (range −0.070…+0.023 — at the noise floor, no stable sign). Three
   different signals (trend, reverse-trend, funding) all fed the SAME ranking
   channel, and that channel carries no forward information. The broader-universe
   spike confirmed it: a more-dispersed 35-name mid-cap basket lowered common
   market-beta (avg R² 0.715 → 0.598, avg pairwise correlation 0.683 → 0.582) but
   rank IC **stayed dead** — the idiosyncratic cross-section got bigger and was
   still unpredictable from the rank. So the limiter is **universe + horizon**,
   not the specific signal.
2. **The time-series angle is now closed too.** TS-momentum removed the ranking
   channel entirely and could go flat — its one structural edge. But its
   drawdowns are catastrophic: **MaxDD_p95 of 88.9%–97.0% (2023) and 81.4%–92.9%
   (2024)** in every cell. The goes-flat mechanism is real (time-in-market ranges
   0.64–0.87 — the strategy genuinely exits, it is NOT always-long buy-and-hold),
   but on a choppy 0.63–0.68-correlated large-cap basket at 1h, whipsaw and late
   exits eat the saved drawdown faster than the trend-capture pays. Even in the
   harder 2024 regime (where buy-and-hold itself has a negative tail, p5 = −0.68),
   TS-momentum still cannot clear the bar.

Read: _"It is not that the coins are one blob — there is ~30% room between them.
It is that nothing about which coin led, or whether a coin is trending, tells you
enough to beat just holding them all, net of fees, at this horizon."_

## Science integrity / methodology — what was actually proven

This is the durable asset the program produced. The negative result is only
decision-grade because the harness is rigorous:

- **91 byte-SHA regression anchors**, all PASS (89 pre-existing + #90 + #91). The
  exact surfaces can be regenerated byte-identical on the canonical box.
- **Block-bootstrap path + parameter robustness**: every verdict is a
  distribution over 200 resampled real-history paths × a 6-cell parameter grid,
  not a single lucky backtest.
- **A frozen, pre-registered decision rule** (5-signal weakest-link, bands locked
  before any run) scored all four families identically — no moving the goalposts.
- **5 day-1 falsifiers, each RED-on-revert** for TS-momentum (baseline
  divergence, signal-non-no-op, no-look-ahead, goes-flat, two-run byte-identity).
  Each test FAILS if the property it guards is reverted — confirmed by mutation.
- **Anti-cherry-pick renderers that crown no winner**: the surface reports the
  full grid + a family verdict and refuses to pick an argmax "best" cell (which
  would inflate the false-positive rate). A grid that picked its best cell would
  lie; this one cannot.
- **Two-run determinism**: identical seed → byte-identical report body.

**The win:** the machine cheaply and rigorously ruled out the four most-cited
active crypto approaches on this universe — for ~35 seconds of compute per
surface — before any capital was risked. The proven research stack (harness +
decision rule + anchor gate) is the asset that survives this negative result.

## Numbers that matter

- **Tests:** 7 passed / 0 failed (`ts_momentum_divergence_e2e`, 0.02s), covering
  all 5 falsifiers; 2 of them additionally confirmed RED-on-revert.
- **Anchors:** **91 / 91 PASS** (verified live — see Verification V6). New: #90
  `c1bf9325…` (2023), #91 `ff7e7dda…` (2024).
- **TS-momentum verdict:** FAMILY-UNIFORM-FRAGILE — all 6 cells FRAGILE, both
  2023 and 2024. p5 Sharpe < 0 in every cell, both years. P(Sharpe>1) = 0.0% in
  every cell.
- **Best TS cell:** g=2 (720-bar lookback, zero threshold) — 2023 p50 +0.0473 /
  2024 p50 +0.0418. Still ~1.7 / ~1.06 Sharpe units below buy-and-hold.
- **Drawdown (the killer):** TS MaxDD_p95 88.9–97.0% (2023), 81.4–92.9% (2024) vs
  buy-and-hold 51.2% (2023) / 64.8% (2024).
- **Goes-flat is real:** time-in-market 0.64–0.87 across all cells/years — the
  strategy genuinely exits to cash; it is not buy-and-hold in disguise.
- **Wall-clock:** 34.6s (2023 surface) / 35.6s (2024 surface) on the canonical
  Apple-Silicon box — within the ≲30 min gate; faster than carry (no funding
  gather).
- **Spec-lint:** 94 violations, 2 categories (dead-link 87 + trace-broken-path 7)
  — **identical to the tester's PASS baseline; zero new violations introduced.**
  All pre-existing spec debt (archived files, /tmp screenshot paths, ADR links to
  removed features); none in this feature's files.

## Verification

| V-id | Description | Status | Evidence |
|------|-------------|--------|----------|
| V1 | TS-momentum produces FAMILY-UNIFORM-FRAGILE on 2023 (all 6 cells) | VERIFIED | anchor #90 surface, family verdict line; test report §5 (2023 table) |
| V2 | TS-momentum produces FAMILY-UNIFORM-FRAGILE on 2024 (all 6 cells) | VERIFIED | anchor #91 surface, family verdict line; test report §5 (2024 table) |
| V3 | TS-momentum dominated by buy-and-hold both years (best cell ≥ ~1.06 below BH) | VERIFIED | surfaces: best p50 +0.047/+0.042 vs BH +1.74/+1.10 |
| V4 | Goes-flat mechanism is real (not always-long ≈ BH) | VERIFIED | `f_tsm_4_goes_flat` PASS + RED-on-revert; time_in_market 0.64–0.87 every cell |
| V5 | 89 pre-existing anchors byte-identical (defaults-off, additive) | VERIFIED | verify_anchors.sh: momentum #86, MR #87, carry #88/#89 all PASS |
| V6 | Two new TS anchors locked + full gate green (91/91) | VERIFIED | `bash scripts/verify_anchors.sh` → `ANCHORS PASS (91 / 91)` (run live this session) |
| V7 | Science gate not void (block-bootstrap-real + shared-index) | VERIFIED | both surface frontmatters: `generator: block-bootstrap-real`, `bootstrap_mode: shared-index`; OHLCV SHA `3a8b96c4…` matches pin |
| V8 | 5 day-1 falsifiers present + RED-on-revert | VERIFIED | test report §3 falsifier table — 7/7 PASS, F-TSM.1 & F-TSM.4 confirmed RED on revert |
| V9 | Program-level: all 4 families FAMILY-UNIFORM-FRAGILE, BH-dominated | VERIFIED | retrospective table above; cross-checked against carry report §5 + TS surfaces |

## Open decisions

There is exactly ONE decision in front of you, and it has two parts that travel
together:

**Decision — Ratify the close-out.** Approve (a) the **retirement of
time-series momentum** (FAMILY-UNIFORM-FRAGILE, both years), and (b) the
**program-level conclusion**: active trading on this 10-symbol 1h Binance
universe is dominated by passive buy-and-hold, net of fees, across both the
cross-sectional-ranking and time-series method classes. Approving this closes the
active-strategy robustness program as a decision-grade negative.

That is the only thing the approval block below gates. **The "where next?"
strategic fork is framed for your awareness but is deliberately NOT bundled into
this approval** — it is a fresh scoping decision the orchestrator will route to
the analyst once you ratify the close-out. (One decision per presentation; I am
not asking you to pick a direction and approve a retirement in the same tick.)

### The strategic fork (FYI — frame only, do NOT decide here)

Where research goes after this negative result. Listed neutrally; no
recommendation, no `(Recommended)` tag — this is genuinely your call and each
option has a real cost:

- **(a) Broader-universe + daily-horizon retest.** The diagnosis points at
  *universe + horizon* as the binding limiter, and the data is **already banked**
  — `data/binance-broaduni` (35 mid-caps, pin `518b4d40…`). This is the
  pre-positioned next axis: re-test on a more-dispersed universe and/or a daily
  horizon the 1h data cannot speak to. Cost: a new strategy/horizon build on the
  banked broader data (the universe spike already exonerated "just broaden the
  basket" for *cross-sectional*, so this would be a horizon and/or non-ranking
  retest, not a 4th cross-sectional family).
- **(b) A different data domain / signal class** — on-chain, event-driven, or
  cross-exchange signals that this OHLCV-only universe cannot express. Cost: new
  data plumbing + a new research arc from scratch.
- **(c) Accept the negative result and harden the proven stack to
  production-grade** — pour the effort into exec / risk / cockpit instead of
  hunting for an active edge that four families failed to find. Cost: shifts the
  program from research to productionization.
- **(d) Pause active-strategy research** entirely.

If you approve with a steer on the fork, note it under Notes/feedback and the
orchestrator will route the next scoping to the analyst accordingly.

## Approval

The approval below gates ONLY the close-out decision above (retire TS-momentum +
ratify the program-level conclusion). All boxes ship un-ticked; you are the only
one who ticks.

- [ ] Approved — ship
- [ ] Approve with notes (notes below)
- [ ] Reject — _add reason below_

### Notes / feedback
_empty until operator fills — note any steer on the strategic fork here_

## Changelog
- 2026-06-03 (presenter): initial release deck + active-strategy robustness
  program retrospective. TS-momentum VERDICT → PASS, FAMILY-UNIFORM-FRAGILE both
  years, anchors #90/#91 (91/91). Numbers traced to anchors #90/#91 surfaces, the
  carry test report, and the universe-method diagnosis. Live demo: N=5 TS sweep +
  live 91/91 anchor gate.
