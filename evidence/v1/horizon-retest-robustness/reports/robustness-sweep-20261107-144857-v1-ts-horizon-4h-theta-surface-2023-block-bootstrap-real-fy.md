---
slug: horizon-retest-robustness
scenario: v1-ts-horizon-4h-theta-surface-2023-block-bootstrap-real-fy
generated: 2026-11-07T14:48:57Z
wall_clock_s: 11.7
host: M022517718D
pid: 84984
git_commit: bdc2c12a1cee61a9b5d3701331c49253e0ca4a51
data_revision_sha: 3a8b96c43f2d8980fd8039303197ff3ac5d01e8f9cebaecdf74c853622dbbfc7
---
# Time-Series Momentum (4h horizon) θ-Surface — Parameter-Robustness Sweep — v1-ts-horizon-4h-theta-surface-2023-block-bootstrap-real-fy

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
| selected_block_length_L  | 124 (θ-independent — same L for all cells per OQ-3)      |
| source_revision_sha      | 3a8b96c43f2d8980fd8039303197ff3ac5d01e8f9cebaecdf74c853622dbbfc7                                    |
| horizon                  | 4h                                               |
| held_constant            | selection_mode=time_series_long_flat score_source=vol_adjusted_return direction=momentum rebalance_minutes=60 exposure_cap=1.00 k_long=10(inert) vol_floor=inert k_short=0 size=equal_weight |

## TS-momentum 4h θ-grid definition (6-cell, LOCKED § D-HR.4-LOCKED — changing this changes the SHA)

grid_definition:
  g=0 lookback=42 entry_threshold=0 k_long=10 drift=0.10
  g=1 lookback=42 entry_threshold=0.02 k_long=10 drift=0.10
  g=2 lookback=180 entry_threshold=0 k_long=10 drift=0.10
  g=3 lookback=180 entry_threshold=0.02 k_long=10 drift=0.10
  g=4 lookback=540 entry_threshold=0 k_long=10 drift=0.10
  g=5 lookback=540 entry_threshold=0.02 k_long=10 drift=0.10

## θ-surface (per-cell distribution + verdict)

Notation: p5/p50/p95 Sharpe at {:.6}; prob_loss / P(Sharpe>1) at {:.6}; p95_maxdd at {:.2}%.
Spread = p95_sharpe − p5_sharpe (interpretive, NOT verdict-forcing).
Verdict: FRAGILE/MARGINAL/ROBUST via 5-signal weakest-link (frozen decision-rule § 0 bands).
Portfolio exposure cap (ADR-0089 D2, all-or-nothing): 0 rebalance(s) refused across every cell and path of this surface. A refusal skips the WHOLE rebalance and leaves the book unchanged for that bar; 0 means the cap never bound, which is the expected reading wherever the grid's gross target stays below it.

Ruined paths: **0 of 1200** — no path's equity reached zero or below at any bar. Stated rather than left to inference: equity <= 0 is clamped to 1e-6 before the metrics, so `p95_maxdd` near 100% would otherwise be indistinguishable from ruin (bug-log #127).

time_in_market = fraction of bars where ≥1 long position was held (mean across N paths, D-TSM.6.4).

| g  | lookback | threshold | k_long | drift | p5_sharpe | p50_sharpe | p95_sharpe | prob_loss | P(Sharpe>1) | p95_maxdd | spread   | time_in_market | verdict  | notes |
|----|----------|-----------|--------|-------|-----------|------------|------------|-----------|-------------|-----------|----------|----------------|----------|-------|
|  0 |       42 | 0.00      |     10 | 0.10 | -0.282700 | 1.191042  | 2.296840  | 0.085000 | 0.595000    | 31.29%   | 2.579540 | 0.8400          | FRAGILE  |  |
|  1 |       42 | 0.02      |     10 | 0.10 | -0.163435 | 1.135372  | 2.188800  | 0.075000 | 0.590000    | 27.49%   | 2.352236 | 0.7564          | FRAGILE  |  |
|  2 |      180 | 0.00      |     10 | 0.10 | -0.343009 | 1.209382  | 2.433240  | 0.090000 | 0.600000    | 30.36%   | 2.776249 | 0.8088          | FRAGILE  |  |
|  3 |      180 | 0.02      |     10 | 0.10 | -0.340641 | 1.176564  | 2.330261  | 0.095000 | 0.600000    | 29.79%   | 2.670902 | 0.7865          | FRAGILE  |  |
|  4 |      540 | 0.00      |     10 | 0.10 | -0.481161 | 0.951321  | 2.307000  | 0.130000 | 0.485000    | 33.69%   | 2.788162 | 0.7115          | FRAGILE  |  |
|  5 |      540 | 0.02      |     10 | 0.10 | -0.473443 | 0.913053  | 2.286047  | 0.140000 | 0.485000    | 33.00%   | 2.759490 | 0.7044          | FRAGILE  |  |

## Buy-and-hold passive control (adversarial-review benchmark)

Equal-weight, hold from bar 0 over the SAME N paths and auto-L bootstrap.
Reference: adversarial review p50 Sharpe ≈ +1.78, P(loss) ≈ 4%, p95 MaxDD ≈ 51% at auto-L, N=500.

| row       | p5_sharpe | p50_sharpe | p95_sharpe | prob_loss | P(Sharpe>1) | p95_maxdd | spread   | verdict  |
|-----------|-----------|------------|------------|-----------|-------------|-----------|----------|----------|
| BUYHOLD   | -0.106196 | 1.930555  | 3.797254  | 0.080000 | 0.745000    | 51.80%   | 3.903450 | (passive — no verdict) |

## Family verdict

FAMILY-UNIFORM-FRAGILE

Every active θ-cell is FRAGILE under the frozen decision-rule bands.
No multiple-testing correction is needed for a uniform-negative result:
C3 is not selecting a winner — it is reporting that no cell cleared the bar.
Conclusion: v1 time-series momentum at the 4h horizon (per-asset long/flat on own trailing return) is
structurally fragile across the tested parameter space on this 10-symbol universe.
Even at the classically-preferred coarser decision cadence, the trend-capture benefit
does not overcome the buy-and-hold bar net of fees. Closes the OHLCV-only active-trading thesis.

Notes:
- Decision-rule bands: frozen robustness-decision-rule-2026-05-30.md § 0.
- Composite = weakest-link of 5 PRIMARY signals (p5_sharpe, p50_sharpe, prob_loss, P(Sharpe>1), p95_maxdd).
- Spread and p50-vs-real-path are INTERPRETIVE (not verdict-forcing).
- Generator: `block-bootstrap-real` only is anchor-grade.
- Determinism scope: Apple-Silicon canonical box (ADR-0051 D5).
