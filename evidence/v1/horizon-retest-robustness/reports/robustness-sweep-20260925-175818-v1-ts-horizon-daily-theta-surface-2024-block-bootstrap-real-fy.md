---
slug: horizon-retest-robustness
scenario: v1-ts-horizon-daily-theta-surface-2024-block-bootstrap-real-fy
generated: 2026-09-25T17:58:18Z
wall_clock_s: 9.5
host: M022517718D
pid: 38391
git_commit: 2d63ddef7964e6a4d5bce6082478ab06f6d04fd8
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
|  0 |        5 | 0.00      |     10 | 0.10 | -0.464753 | 0.480156  | 1.271377  | 0.204000 | 0.157000    | 28.36%   | 1.736130 | 0.8216          | FRAGILE  |  |
|  1 |        5 | 0.02      |     10 | 0.10 | -0.473280 | 0.466190  | 1.274858  | 0.191000 | 0.144000    | 27.06%   | 1.748138 | 0.7681          | FRAGILE  |  |
|  2 |       20 | 0.00      |     10 | 0.10 | -0.450167 | 0.530674  | 1.414760  | 0.185000 | 0.196000    | 29.22%   | 1.864927 | 0.8111          | FRAGILE  |  |
|  3 |       20 | 0.02      |     10 | 0.10 | -0.412659 | 0.534296  | 1.413445  | 0.183000 | 0.201000    | 28.15%   | 1.826104 | 0.7888          | FRAGILE  |  |
|  4 |       60 | 0.00      |     10 | 0.10 | -0.400853 | 0.509026  | 1.426571  | 0.172000 | 0.184000    | 26.01%   | 1.827423 | 0.7325          | FRAGILE  |  |
|  5 |       60 | 0.02      |     10 | 0.10 | -0.423296 | 0.504701  | 1.413110  | 0.178000 | 0.183000    | 25.50%   | 1.836406 | 0.7262          | FRAGILE  |  |

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
