---
slug: time-series-momentum-robustness
scenario: v1-ts-momentum-theta-surface-2024-block-bootstrap-real-fy
generated: 2026-11-07T14:51:05Z
wall_clock_s: 47.5
host: M022517718D
pid: 87017
git_commit: bdc2c12a1cee61a9b5d3701331c49253e0ca4a51
data_revision_sha: 3a8b96c43f2d8980fd8039303197ff3ac5d01e8f9cebaecdf74c853622dbbfc7
---
# Time-Series Momentum θ-Surface — Parameter-Robustness Sweep — v1-ts-momentum-theta-surface-2024-block-bootstrap-real-fy

## Ensemble parameters (shared across all θ-cells)

| Field                    | Value                                                   |
|--------------------------|----------------------------------------------------------|
| master_seed              | 0xC0FFEE                                          |
| fill_seed                | 0xC0FFEE                                          |
| n_paths                  | 200                                                 |
| sub_seed_rule            | "master + j*0x9E3779B9 (SAME paths across cells, ADR-0051 D6.1)" |
| reduction_rule           | "index-order mean/std; total_cmp sort; type-7 linear pct" |
| generator                | block-bootstrap-real                                        |
| bootstrap_mode           | shared-index                                         |
| block_length_policy      | auto                                    |
| selected_block_length_L  | 282 (θ-independent — same L for all cells per OQ-3)      |
| source_revision_sha      | 3a8b96c43f2d8980fd8039303197ff3ac5d01e8f9cebaecdf74c853622dbbfc7                                    |
| held_constant            | selection_mode=time_series_long_flat score_source=vol_adjusted_return direction=momentum rebalance_minutes=60 exposure_cap=1.00 k_long=10(inert) vol_floor=inert k_short=0 size=equal_weight |

## TS-momentum θ-grid definition (6-cell, LOCKED § D-TSM.3-LOCKED — changing this changes the SHA)

grid_definition:
  g=0 lookback=168 entry_threshold=0 k_long=10 drift=0.10
  g=1 lookback=24 entry_threshold=0 k_long=10 drift=0.10
  g=2 lookback=720 entry_threshold=0 k_long=10 drift=0.10
  g=3 lookback=168 entry_threshold=0.02 k_long=10 drift=0.10
  g=4 lookback=720 entry_threshold=0.02 k_long=10 drift=0.10
  g=5 lookback=24 entry_threshold=0.02 k_long=10 drift=0.10

## θ-surface (per-cell distribution + verdict)

Notation: p5/p50/p95 Sharpe at {:.6}; prob_loss / P(Sharpe>1) at {:.6}; p95_maxdd at {:.2}%.
Spread = p95_sharpe − p5_sharpe (interpretive, NOT verdict-forcing).
Verdict: FRAGILE/MARGINAL/ROBUST via 5-signal weakest-link (frozen decision-rule § 0 bands).
Portfolio exposure cap (ADR-0089 D2, all-or-nothing): 0 rebalance(s) refused across every cell and path of this surface. A refusal skips the WHOLE rebalance and leaves the book unchanged for that bar; 0 means the cap never bound, which is the expected reading wherever the grid's gross target stays below it.

Ruined paths: **0 of 1200** — no path's equity reached zero or below at any bar. Stated rather than left to inference: equity <= 0 is clamped to 1e-6 before the metrics, so `p95_maxdd` near 100% would otherwise be indistinguishable from ruin (bug-log #127).

time_in_market = fraction of bars where ≥1 long position was held (mean across N paths, D-TSM.6.4).

| g  | lookback | threshold | k_long | drift | p5_sharpe | p50_sharpe | p95_sharpe | prob_loss | P(Sharpe>1) | p95_maxdd | spread   | time_in_market | verdict  | notes |
|----|----------|-----------|--------|-------|-----------|------------|------------|-----------|-------------|-----------|----------|----------------|----------|-------|
|  0 |      168 | 0.00      |     10 | 0.10 | -0.549558 | 0.392219  | 1.680981  | 0.275000 | 0.285000    | 50.67%   | 2.230539 | 0.8169          | FRAGILE  |  |
|  1 |       24 | 0.00      |     10 | 0.10 | -1.289262 | -0.394767  | 0.914702  | 0.665000 | 0.040000    | 60.64%   | 2.203963 | 0.8298          | FRAGILE  |  |
|  2 |      720 | 0.00      |     10 | 0.10 | -0.609816 | 0.445746  | 1.964836  | 0.305000 | 0.290000    | 45.67%   | 2.574652 | 0.7882          | FRAGILE  |  |
|  3 |      168 | 0.02      |     10 | 0.10 | -0.495028 | 0.434162  | 1.825531  | 0.260000 | 0.310000    | 45.19%   | 2.320558 | 0.7587          | FRAGILE  |  |
|  4 |      720 | 0.02      |     10 | 0.10 | -0.653759 | 0.488664  | 1.898133  | 0.310000 | 0.270000    | 45.34%   | 2.551893 | 0.7621          | FRAGILE  |  |
|  5 |       24 | 0.02      |     10 | 0.10 | -1.123877 | -0.284655  | 0.741366  | 0.645000 | 0.015000    | 47.43%   | 1.865244 | 0.6442          | FRAGILE  |  |

## Buy-and-hold passive control (adversarial-review benchmark)

Equal-weight, hold from bar 0 over the SAME N paths and auto-L bootstrap.
Reference: adversarial review p50 Sharpe ≈ +1.78, P(loss) ≈ 4%, p95 MaxDD ≈ 51% at auto-L, N=500.

| row       | p5_sharpe | p50_sharpe | p95_sharpe | prob_loss | P(Sharpe>1) | p95_maxdd | spread   | verdict  |
|-----------|-----------|------------|------------|-----------|-------------|-----------|----------|----------|
| BUYHOLD   | -0.475960 | 0.945434  | 2.613640  | 0.150000 | 0.470000    | 65.86%   | 3.089601 | (passive — no verdict) |

## Family verdict

FAMILY-UNIFORM-FRAGILE

Every active θ-cell is FRAGILE under the frozen decision-rule bands.
No multiple-testing correction is needed for a uniform-negative result:
C3 is not selecting a winner — it is reporting that no cell cleared the bar.
Conclusion: v1 time-series momentum (per-asset long/flat on own trailing return) is
structurally fragile across the tested parameter space on this 10-symbol 1h universe.
Whipsaw/fee-bleed or late exits may have dominated the trend-capture benefit.
This closes the active-trading thesis on this universe: no method (x-sec or time-series)
beat passive buy-and-hold net of fees. Routes to broader-universe / horizon axis.

Notes:
- Decision-rule bands: frozen robustness-decision-rule-2026-05-30.md § 0.
- Composite = weakest-link of 5 PRIMARY signals (p5_sharpe, p50_sharpe, prob_loss, P(Sharpe>1), p95_maxdd).
- Spread and p50-vs-real-path are INTERPRETIVE (not verdict-forcing).
- Generator: `block-bootstrap-real` only is anchor-grade.
- Determinism scope: Apple-Silicon canonical box (ADR-0051 D5).
