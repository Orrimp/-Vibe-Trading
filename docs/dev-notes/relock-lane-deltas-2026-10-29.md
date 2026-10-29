# Re-lock, measured: the 15 lane scenarios — deltas before landing anything

> ## ⚠ CORRECTION — the first table below was CROSS-NAMESPACE and is wrong
>
> **Everything in § "All 15 moved" was measured against the wrong baseline and must not be
> used.** My comparison resolved each scenario's anchored body with the glob
> `evidence/*/*/reports/*.md` — **two** path levels — which never matches the canonical
> migration folders, because those sit one level down (`evidence/v5-latency-slippage-sim-v0.3.0-full-path-wiring/reports/`).
> So it silently resolved to `evidence/v1/<feature>/reports/`, which holds the
> **noop-baseline** (zero-friction) bodies. Every "old" figure in that table is a zero-friction
> number compared against a canonical-friction regeneration: a comparison between two different
> configurations, not a before-and-after.
>
> I then compounded it. I spotted the multi-namespace risk from the dry run, wrote that the
> deltas "may be cross-namespace", and talked myself out of it on the strength of the determinism
> gate pinning a canonical digest — which was true and did not establish what my own glob was
> resolving to. The dry run had the evidence in it the whole time: the digests it offered to
> replace were canonical ones while the files it was about to overwrite sat in the noop folders.
>
> **I also landed on that mistake and reverted it.** 14 bodies were written into the noop
> folders, which made them the newest file OUTSIDE the canonical dirs and so re-pointed the 14
> noop-baseline rows at canonical-friction bodies. `verify_anchors.sh` went to FAIL and named
> exactly those rows. Reverted from a pre-landing snapshot plus a dated `find -delete`; the
> corpus is back at **119/119** and the tree is clean. Nothing was lost — D6.b landings add
> files, and the snapshot was taken before the write.
>
> The corrected measurement is § "Re-measured correctly" at the foot of this note. Use only that.


**Date:** 2026-10-29 · **Status: MEASURED, NOT LANDED.** Nothing has been written into
`evidence/`; `verify_anchors.sh` reads 119/119 unchanged.

Regenerated with `scripts/relock/run_lane_scenarios.sh` into a scratchpad out-dir (15 of 15, 0
failed), then diffed against the CURRENT anchored body for each scenario — newest filename match
across `evidence/**/reports/`, the same resolution `verify_anchors.sh` uses.

## All 15 moved. None was unaffected.

| scenario | trades | total return |
|---|---|---|
| `top10-2023-1h-momentum` | 4809 → 5277 | −43.72% → −37.93% |
| `top10-2024-h1-momentum` | 2490 → 2693 | −53.60% → **−18.48%** |
| `top10-2023-fy-momentum-realdata` | 6203 → 6203 | +13.48% → **−27.52%** |
| `pairs-2023-zscore-mr` | **16 → 430** | −160.52% → **+0.48%** |
| `pairs-2024-h1-zscore-mr` | **16 → 216** | −160.52% → **−6.34%** |
| `top10-2023-fy-tcn-overlay` | 1224 → 1247 | −69.76% → **−6.22%** |
| `top10-2024-fy-tcn-overlay` | 3672 → 3987 | −55.70% → −23.96% |
| `top10-2023-fy-tcn-overlay-realdata` | **1138 → 6203** | −4.69% → **−27.52%** |
| `top10-2024-fy-tcn-overlay-realdata` | **1030 → 5917** | −3.15% → −22.32% |
| `top10-2023-fy-tcn-overlay-weights` | 1224 → 1247 | −69.76% → −6.22% |
| `top10-2024-fy-tcn-overlay-weights` | 3672 → 3987 | −55.70% → −23.96% |
| `top10-2023-fy-tcn-overlay-weights-realdata` | 1138 → 6203 | −4.69% → −27.52% |
| `top10-2024-fy-tcn-overlay-weights-realdata` | 1030 → 5917 | −3.15% → −22.32% |
| `top10-2023-fy-patchtst-overlay-realdata` | **580 → 3187** | +3.56% → **+41.81%** |
| `top10-2023-fy-vol-target-overlay-realdata` | 1138 → 3266 | −10.52% → −27.67% |

Plus, confirmed separately by running its own 1-29 reproduction gate (which was green on
2026-10-11): `sharpe-comparison-vol-target-bs1-realdata` and `-realbaseline` also move. Still
unmeasured: `forecast-distribution-patchtst-bs1-realdata` (a 6-min gate) and
`v1-momentum-2023-block-bootstrap-real-fy-mc` (~4 min).

## What the numbers say

**Trades rise everywhere, which is the fix doing what it claims.** `#111`'s silent drops were
suppressing orders; routing them to their own symbol's bar lets them fill. The extremes are the
tell: `pairs` goes **16 → 430** (27×) and the realdata tcn lanes **1138 → 6203** (5.5×). A lane
reporting 16 lifetime fills was reporting almost nothing.

**But the direction on RETURNS is mixed, and that is new information.** When the re-lock was
authorised the expectation — set by the one scenario measured on 2026-10-14 — was that corrections
would make active arms look *worse*, because dropped orders had been dodging fees and drawdown.
That held for momentum-realdata (+13.48% → −27.52%), the realdata tcn lanes and vol-target. It did
**not** hold for:

- `pairs`: **−160.52% → +0.48% / −6.34%.** A return worse than total loss is the signature of
  mispriced fills compounding against the book; 16 of them were enough to produce it. The
  corrected arm is merely flat.
- `top10-2023-fy-patchtst-overlay-realdata`: **+3.56% → +41.81%.**
- `top10-2023-fy-tcn-overlay`: −69.76% → −6.22%.

**Why this does not, by itself, touch the thesis.** These are standalone scenario reports. None
carries a verdict line or a buy-and-hold control — I checked the patchtst body specifically. The
era-qualified ship-passive conclusion rests on the bake-off/theta-surface corpus, where
buy-and-hold IS the control and 0 of 156 cells beat it, and nothing here has been regenerated.

**Why it still needs a decision.** A +41.81% anchored return where the corpus currently publishes
+3.56%, and a pairs lane moving 161 points, are large changes to published evidence — larger than
anyone knew when "fix the code, then re-lock" was agreed. Two things follow that I cannot settle
alone:

1. Whether any narrative elsewhere in the corpus cites these lanes' numbers in support of
   ship-passive. If so, that narrative needs re-checking against the corrected figures, and the
   patchtst lane is the one to look at first.
2. Whether the operator still wants all 15 landed in one pass given the magnitudes, or wants the
   favourable movers examined before publication.

The code fixes are not in question — they are tested, probed, and the trade-count direction
confirms them. This is about what the corpus should say next.


---

## Re-measured correctly (namespace-aware) — 2026-10-29

Baseline = newest body in a **canonical** migration folder (`evidence/v5-latency-slippage-sim-v0*`),
which is the namespace a default `backtest --scenario …` run reproduces. The noop-baseline and
sqrt-impact rows resolve to frozen bodies in their own folders under their own freeze contracts,
and a default run reproduces neither.

| scenario | trades | total return |
|---|---|---|
| `top10-2023-1h-momentum` | 592 → **5277** | −12.39% → −37.93% |
| `top10-2024-h1-momentum` | 295 → **2693** | −3.81% → −18.48% |
| `pairs-2023-zscore-mr` | **0 → 430** | 0.00% → +0.48% |
| `pairs-2024-h1-zscore-mr` | **0 → 216** | 0.00% → −6.34% |
| `top10-2023-fy-momentum-realdata` | 1138 → 6203 | −44.23% → **−27.52%** |
| `top10-2023-fy-tcn-overlay` | 130 → **1247** | +0.27% → −6.22% |
| `top10-2024-fy-tcn-overlay` | 440 → **3987** | −8.09% → −23.96% |
| `top10-2023-fy-tcn-overlay-realdata` | 6203 → 6203 | −89.89% → **−27.52%** |
| `top10-2024-fy-tcn-overlay-realdata` | 5917 → 5917 | −77.58% → **−22.32%** |
| `top10-2023-fy-tcn-overlay-weights` | 130 → 1247 | +0.27% → −6.22% |
| `top10-2024-fy-tcn-overlay-weights` | 440 → 3987 | −8.09% → −23.96% |
| `top10-2023-fy-tcn-overlay-weights-realdata` | 6203 → 6203 | −89.89% → −27.52% |
| `top10-2024-fy-tcn-overlay-weights-realdata` | 5917 → 5917 | −77.58% → −22.32% |
| `top10-2023-fy-patchtst-overlay-realdata` | 3187 → 3187 | **−78.47% → +41.81%** |
| `top10-2023-fy-vol-target-overlay-realdata` | **4585 → 3266** | −38.53% → −27.67% |

### What the corrected numbers say

**The `pairs` canonical bodies record ZERO trades.** Both of them. That independently confirms the
`#95` trace's claim that *"pairs trades zero times at HEAD"* — and it means two anchored bodies
have been publishing a strategy that never placed an order. The fix gives them 430 and 216 fills.

**One earlier measurement survives unchanged.** The 2026-10-14 figure for
`top10-2023-1h-momentum`, 592 → 5277, reproduces exactly here — it used
`find … | sort | tail -1`, which does reach the canonical folders. So that measurement, and the
~89%-of-orders-dropped conclusion drawn from it, stand.

**`vol-target` is the only lane whose trades FALL** (4585 → 3266), and that is garch's new gross
cap refusing orders — `#95`'s enforcement doing exactly what it was built to do, visible as a
trade-count reduction rather than only as a counter.

**Returns move both ways, and the standout needs naming:**
`top10-2023-fy-patchtst-overlay-realdata` goes **−78.47% → +41.81%** on an unchanged trade count
of 3187 — so the same orders, filled at their own symbol's prices instead of the trigger bar's,
swing 120 points. A −78% body was publishing an arm wrecked by mispriced fills. The corrected arm
is strongly positive, and **that is the one figure here that could bear on how a reader reads the
active-vs-passive question**, so it should not be landed without someone looking at it against a
buy-and-hold control for the same window. The body itself carries no such control.
