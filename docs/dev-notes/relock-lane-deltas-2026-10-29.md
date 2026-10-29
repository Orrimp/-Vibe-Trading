# Re-lock, measured: the 15 lane scenarios — deltas before landing anything

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
