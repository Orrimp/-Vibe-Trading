---
slug: time-series-momentum-robustness
scenario: v1-ts-momentum-theta-surface-2023-block-bootstrap-real-fy
generated: 2026-11-07T14:50:17Z
wall_clock_s: 46.5
host: M022517718D
pid: 86025
git_commit: bdc2c12a1cee61a9b5d3701331c49253e0ca4a51
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
| selected_block_length_L  | 281 (θ-independent — same L for all cells per OQ-3)      |
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
|  0 |      168 | 0.00      |     10 | 0.10 | -0.280876 | 0.747411  | 2.023703  | 0.120000 | 0.385000    | 40.97%   | 2.304579 | 0.8379          | FRAGILE  |  |
|  1 |       24 | 0.00      |     10 | 0.10 | -1.816649 | -0.693443  | 0.669805  | 0.805000 | 0.020000    | 65.75%   | 2.486454 | 0.8455          | FRAGILE  |  |
|  2 |      720 | 0.00      |     10 | 0.10 | -0.225273 | 0.914335  | 2.186862  | 0.120000 | 0.465000    | 35.66%   | 2.412135 | 0.8193          | FRAGILE  |  |
|  3 |      168 | 0.02      |     10 | 0.10 | -0.198914 | 0.820557  | 1.951221  | 0.100000 | 0.405000    | 33.09%   | 2.150135 | 0.7628          | FRAGILE  |  |
|  4 |      720 | 0.02      |     10 | 0.10 | -0.335151 | 0.950938  | 2.125817  | 0.115000 | 0.450000    | 36.28%   | 2.460968 | 0.7983          | FRAGILE  |  |
|  5 |       24 | 0.02      |     10 | 0.10 | -0.980520 | -0.165717  | 0.717758  | 0.625000 | 0.010000    | 41.62%   | 1.698278 | 0.6001          | FRAGILE  |  |

## Buy-and-hold passive control (adversarial-review benchmark)

Equal-weight, hold from bar 0 over the SAME N paths and auto-L bootstrap.
Reference: adversarial review p50 Sharpe ≈ +1.78, P(loss) ≈ 4%, p95 MaxDD ≈ 51% at auto-L, N=500.

| row       | p5_sharpe | p50_sharpe | p95_sharpe | prob_loss | P(Sharpe>1) | p95_maxdd | spread   | verdict  |
|-----------|-----------|------------|------------|-----------|-------------|-----------|----------|----------|
| BUYHOLD   | 0.246190 | 1.812105  | 3.586571  | 0.045000 | 0.760000    | 51.73%   | 3.340381 | (passive — no verdict) |

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
