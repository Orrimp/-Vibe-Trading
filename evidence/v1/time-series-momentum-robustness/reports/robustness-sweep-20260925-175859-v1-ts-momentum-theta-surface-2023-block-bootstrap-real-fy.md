---
slug: time-series-momentum-robustness
scenario: v1-ts-momentum-theta-surface-2023-block-bootstrap-real-fy
generated: 2026-09-25T17:58:59Z
wall_clock_s: 40.9
host: M022517718D
pid: 38567
git_commit: 2d63ddef7964e6a4d5bce6082478ab06f6d04fd8
data_revision_sha: 3a8b96c43f2d8980fd8039303197ff3ac5d01e8f9cebaecdf74c853622dbbfc7
---
# Time-Series Momentum θ-Surface — Parameter-Robustness Sweep — v1-ts-momentum-theta-surface-2023-block-bootstrap-real-fy

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
| selected_block_length_L  | 204 (θ-independent — same L for all cells per OQ-3)      |
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
|  0 |      168 | 0.00      |     10 | 0.10 | -0.402554 | 0.552276  | 1.603428  | 0.200000 | 0.255000    | 27.46%   | 2.005982 | 0.8321          | FRAGILE  |  |
|  1 |       24 | 0.00      |     10 | 0.10 | -1.600154 | -0.695441  | 0.204734  | 0.895000 | 0.000000    | 47.62%   | 1.804887 | 0.8267          | FRAGILE  |  |
|  2 |      720 | 0.00      |     10 | 0.10 | -0.201521 | 0.813943  | 1.816873  | 0.100000 | 0.365000    | 22.57%   | 2.018394 | 0.8130          | FRAGILE  |  |
|  3 |      168 | 0.02      |     10 | 0.10 | -0.228129 | 0.591950  | 1.560677  | 0.130000 | 0.285000    | 23.22%   | 1.788805 | 0.7595          | FRAGILE  |  |
|  4 |      720 | 0.02      |     10 | 0.10 | -0.187433 | 0.780904  | 1.854015  | 0.085000 | 0.370000    | 22.13%   | 2.041447 | 0.7946          | FRAGILE  |  |
|  5 |       24 | 0.02      |     10 | 0.10 | -0.948301 | -0.221631  | 0.453679  | 0.655000 | 0.005000    | 32.03%   | 1.401979 | 0.6057          | FRAGILE  |  |

## Buy-and-hold passive control (adversarial-review benchmark)

Equal-weight, hold from bar 0 over the SAME N paths and auto-L bootstrap.
Reference: adversarial review p50 Sharpe ≈ +1.78, P(loss) ≈ 4%, p95 MaxDD ≈ 51% at auto-L, N=500.

| row       | p5_sharpe | p50_sharpe | p95_sharpe | prob_loss | P(Sharpe>1) | p95_maxdd | spread   | verdict  |
|-----------|-----------|------------|------------|-----------|-------------|-----------|----------|----------|
| BUYHOLD   | 0.124469 | 1.735275  | 3.870337  | 0.045000 | 0.775000    | 51.15%   | 3.745868 | (passive — no verdict) |

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
