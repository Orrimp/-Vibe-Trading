---
slug: horizon-retest-robustness
scenario: v1-ts-horizon-daily-theta-surface-2024-block-bootstrap-real-fy
generated: 2026-06-05T07:11:51Z
wall_clock_s: 6.8
host: M022517718D
pid: 8479
git_commit: d8f327ccda527cb2ae3cfc38b639457e2c3c7a8d
data_revision_sha: 3a8b96c43f2d8980fd8039303197ff3ac5d01e8f9cebaecdf74c853622dbbfc7
---
# Time-Series Momentum (daily horizon) θ-Surface — Parameter-Robustness Sweep — v1-ts-horizon-daily-theta-surface-2024-block-bootstrap-real-fy

## Ensemble parameters (shared across all θ-cells)

| Field                    | Value                                                   |
|--------------------------|----------------------------------------------------------|
| master_seed              | 0xC0FFEE                                          |
| fill_seed                | 0xC0FFEE                                          |
| n_paths                  | 1000                                                 |
| sub_seed_rule            | "master + j*0x9E3779B9 (SAME paths across cells, ADR-0051 D6.1)" |
| reduction_rule           | "index-order mean/std; total_cmp sort; type-7 linear pct" |
| generator                | block-bootstrap-real                                        |
| bootstrap_mode           | shared-index                                         |
| block_length_policy      | auto                                    |
| selected_block_length_L  | 3 (θ-independent — same L for all cells per OQ-3)      |
| source_revision_sha      | 3a8b96c43f2d8980fd8039303197ff3ac5d01e8f9cebaecdf74c853622dbbfc7                                    |
| horizon                  | daily                                               |
| held_constant            | selection_mode=time_series_long_flat score_source=vol_adjusted_return direction=momentum rebalance_minutes=60 exposure_cap=0.50 k_long=10(inert) vol_floor=inert k_short=0 size=equal_weight |

## TS-momentum daily θ-grid definition (6-cell, LOCKED § D-HR.4-LOCKED — changing this changes the SHA)

grid_definition:
  g=0 lookback=5 entry_threshold=0 k_long=10 drift=0.10
  g=1 lookback=5 entry_threshold=0.02 k_long=10 drift=0.10
  g=2 lookback=20 entry_threshold=0 k_long=10 drift=0.10
  g=3 lookback=20 entry_threshold=0.02 k_long=10 drift=0.10
  g=4 lookback=60 entry_threshold=0 k_long=10 drift=0.10
  g=5 lookback=60 entry_threshold=0.02 k_long=10 drift=0.10

## θ-surface (per-cell distribution + verdict)

Notation: p5/p50/p95 Sharpe at {:.6}; prob_loss / P(Sharpe>1) at {:.6}; p95_maxdd at {:.2}%.
Spread = p95_sharpe − p5_sharpe (interpretive, NOT verdict-forcing).
Verdict: FRAGILE/MARGINAL/ROBUST via 5-signal weakest-link (frozen decision-rule § 0 bands).

time_in_market = fraction of bars where ≥1 long position was held (mean across N paths, D-TSM.6.4).

| g  | lookback | threshold | k_long | drift | p5_sharpe | p50_sharpe | p95_sharpe | prob_loss | P(Sharpe>1) | p95_maxdd | spread   | time_in_market | verdict  | notes |
|----|----------|-----------|--------|-------|-----------|------------|------------|-----------|-------------|-----------|----------|----------------|----------|-------|
|  0 |        5 | 0.00      |     10 | 0.10 | -0.035621 | 0.035450  | 0.114444  | 0.239000 | 0.000000    | 85.20%   | 0.150065 | 0.8472          | FRAGILE  |  |
|  1 |        5 | 0.02      |     10 | 0.10 | -0.037957 | 0.029586  | 0.102563  | 0.251000 | 0.000000    | 85.43%   | 0.140520 | 0.7810          | FRAGILE  |  |
|  2 |       20 | 0.00      |     10 | 0.10 | -0.067562 | 0.060319  | 0.231952  | 0.209000 | 0.000000    | 81.47%   | 0.299514 | 0.8355          | FRAGILE  |  |
|  3 |       20 | 0.02      |     10 | 0.10 | -0.069646 | 0.053472  | 0.211931  | 0.244000 | 0.000000    | 81.13%   | 0.281577 | 0.8107          | FRAGILE  |  |
|  4 |       60 | 0.00      |     10 | 0.10 | -0.098900 | 0.106119  | 0.432041  | 0.207000 | 0.002000    | 76.10%   | 0.530941 | 0.7726          | FRAGILE  |  |
|  5 |       60 | 0.02      |     10 | 0.10 | -0.096444 | 0.095647  | 0.441639  | 0.217000 | 0.001000    | 76.52%   | 0.538083 | 0.7635          | FRAGILE  |  |

## Buy-and-hold passive control (adversarial-review benchmark)

Equal-weight, hold from bar 0 over the SAME N paths and auto-L bootstrap.
Reference: adversarial review p50 Sharpe ≈ +1.78, P(loss) ≈ 4%, p95 MaxDD ≈ 51% at auto-L, N=500.

| row       | p5_sharpe | p50_sharpe | p95_sharpe | prob_loss | P(Sharpe>1) | p95_maxdd | spread   | verdict  |
|-----------|-----------|------------|------------|-----------|-------------|-----------|----------|----------|
| BUYHOLD   | -0.549673 | 1.148085  | 2.811356  | 0.149000 | 0.543000    | 63.95%   | 3.361029 | (passive — no verdict) |

## Family verdict

FAMILY-UNIFORM-FRAGILE

Every active θ-cell is FRAGILE under the frozen decision-rule bands.
No multiple-testing correction is needed for a uniform-negative result:
C3 is not selecting a winner — it is reporting that no cell cleared the bar.
Conclusion: v1 time-series momentum at the daily horizon (per-asset long/flat on own trailing return) is
structurally fragile across the tested parameter space on this 10-symbol universe.
Even at the classically-preferred coarser decision cadence, the trend-capture benefit
does not overcome the buy-and-hold bar net of fees. Closes the OHLCV-only active-trading thesis.

Notes:
- Decision-rule bands: frozen robustness-decision-rule-2026-05-30.md § 0.
- Composite = weakest-link of 5 PRIMARY signals (p5_sharpe, p50_sharpe, prob_loss, P(Sharpe>1), p95_maxdd).
- Spread and p50-vs-real-path are INTERPRETIVE (not verdict-forcing).
- Generator: `block-bootstrap-real` only is anchor-grade.
- Determinism scope: Apple-Silicon canonical box (ADR-0051 D5).
