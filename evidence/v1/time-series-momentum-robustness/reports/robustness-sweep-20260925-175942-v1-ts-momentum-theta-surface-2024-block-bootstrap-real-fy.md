---
slug: time-series-momentum-robustness
scenario: v1-ts-momentum-theta-surface-2024-block-bootstrap-real-fy
generated: 2026-09-25T17:59:42Z
wall_clock_s: 42.9
host: M022517718D
pid: 39111
git_commit: 2d63ddef7964e6a4d5bce6082478ab06f6d04fd8
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
| selected_block_length_L  | 200 (θ-independent — same L for all cells per OQ-3)      |
| source_revision_sha      | 3a8b96c43f2d8980fd8039303197ff3ac5d01e8f9cebaecdf74c853622dbbfc7                                    |
| held_constant            | selection_mode=time_series_long_flat score_source=vol_adjusted_return direction=momentum rebalance_minutes=60 exposure_cap=0.50 k_long=10(inert) vol_floor=inert k_short=0 size=equal_weight |

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

time_in_market = fraction of bars where ≥1 long position was held (mean across N paths, D-TSM.6.4).

| g  | lookback | threshold | k_long | drift | p5_sharpe | p50_sharpe | p95_sharpe | prob_loss | P(Sharpe>1) | p95_maxdd | spread   | time_in_market | verdict  | notes |
|----|----------|-----------|--------|-------|-----------|------------|------------|-----------|-------------|-----------|----------|----------------|----------|-------|
|  0 |      168 | 0.00      |     10 | 0.10 | -0.775267 | 0.462997  | 1.356340  | 0.285000 | 0.200000    | 38.27%   | 2.131607 | 0.8101          | FRAGILE  |  |
|  1 |       24 | 0.00      |     10 | 0.10 | -1.220299 | -0.202215  | 0.757073  | 0.635000 | 0.020000    | 47.57%   | 1.977371 | 0.8229          | FRAGILE  |  |
|  2 |      720 | 0.00      |     10 | 0.10 | -0.769526 | 0.477946  | 1.485206  | 0.250000 | 0.245000    | 35.15%   | 2.254732 | 0.7892          | FRAGILE  |  |
|  3 |      168 | 0.02      |     10 | 0.10 | -0.649740 | 0.464417  | 1.578845  | 0.270000 | 0.205000    | 35.87%   | 2.228585 | 0.7536          | FRAGILE  |  |
|  4 |      720 | 0.02      |     10 | 0.10 | -0.693568 | 0.473501  | 1.459388  | 0.255000 | 0.240000    | 33.27%   | 2.152956 | 0.7651          | FRAGILE  |  |
|  5 |       24 | 0.02      |     10 | 0.10 | -1.117243 | -0.260993  | 0.722161  | 0.680000 | 0.020000    | 40.66%   | 1.839404 | 0.6556          | FRAGILE  |  |

## Buy-and-hold passive control (adversarial-review benchmark)

Equal-weight, hold from bar 0 over the SAME N paths and auto-L bootstrap.
Reference: adversarial review p50 Sharpe ≈ +1.78, P(loss) ≈ 4%, p95 MaxDD ≈ 51% at auto-L, N=500.

| row       | p5_sharpe | p50_sharpe | p95_sharpe | prob_loss | P(Sharpe>1) | p95_maxdd | spread   | verdict  |
|-----------|-----------|------------|------------|-----------|-------------|-----------|----------|----------|
| BUYHOLD   | -0.682135 | 1.104731  | 2.690469  | 0.165000 | 0.535000    | 64.83%   | 3.372604 | (passive — no verdict) |

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
