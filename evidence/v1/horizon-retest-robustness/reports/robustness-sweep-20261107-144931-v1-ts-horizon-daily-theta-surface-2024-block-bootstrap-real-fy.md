---
slug: horizon-retest-robustness
scenario: v1-ts-horizon-daily-theta-surface-2024-block-bootstrap-real-fy
generated: 2026-11-07T14:49:31Z
wall_clock_s: 10.7
host: M022517718D
pid: 85786
git_commit: bdc2c12a1cee61a9b5d3701331c49253e0ca4a51
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
| selected_block_length_L  | 22 (θ-independent — same L for all cells per OQ-3)      |
| source_revision_sha      | 3a8b96c43f2d8980fd8039303197ff3ac5d01e8f9cebaecdf74c853622dbbfc7                                    |
| horizon                  | daily                                               |
| held_constant            | selection_mode=time_series_long_flat score_source=vol_adjusted_return direction=momentum rebalance_minutes=60 exposure_cap=1.00 k_long=10(inert) vol_floor=inert k_short=0 size=equal_weight |

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
Portfolio exposure cap (ADR-0089 D2, all-or-nothing): 0 rebalance(s) refused across every cell and path of this surface. A refusal skips the WHOLE rebalance and leaves the book unchanged for that bar; 0 means the cap never bound, which is the expected reading wherever the grid's gross target stays below it.

Ruined paths: **0 of 6000** — no path's equity reached zero or below at any bar. Stated rather than left to inference: equity <= 0 is clamped to 1e-6 before the metrics, so `p95_maxdd` near 100% would otherwise be indistinguishable from ruin (bug-log #127).

time_in_market = fraction of bars where ≥1 long position was held (mean across N paths, D-TSM.6.4).

| g  | lookback | threshold | k_long | drift | p5_sharpe | p50_sharpe | p95_sharpe | prob_loss | P(Sharpe>1) | p95_maxdd | spread   | time_in_market | verdict  | notes |
|----|----------|-----------|--------|-------|-----------|------------|------------|-----------|-------------|-----------|----------|----------------|----------|-------|
|  0 |        5 | 0.00      |     10 | 0.10 | -0.606275 | 0.800286  | 2.068519  | 0.163000 | 0.396000    | 46.33%   | 2.674794 | 0.8266          | FRAGILE  |  |
|  1 |        5 | 0.02      |     10 | 0.10 | -0.316657 | 0.933238  | 1.996773  | 0.095000 | 0.469000    | 34.73%   | 2.313429 | 0.7405          | FRAGILE  |  |
|  2 |       20 | 0.00      |     10 | 0.10 | -0.722810 | 0.805109  | 2.149705  | 0.188000 | 0.417000    | 44.52%   | 2.872514 | 0.7900          | FRAGILE  |  |
|  3 |       20 | 0.02      |     10 | 0.10 | -0.719844 | 0.795501  | 2.146232  | 0.187000 | 0.418000    | 42.85%   | 2.866077 | 0.7481          | FRAGILE  |  |
|  4 |       60 | 0.00      |     10 | 0.10 | -0.761700 | 0.558038  | 1.965103  | 0.242000 | 0.304000    | 40.99%   | 2.726803 | 0.7470          | FRAGILE  |  |
|  5 |       60 | 0.02      |     10 | 0.10 | -0.755640 | 0.567540  | 1.970891  | 0.235000 | 0.307000    | 40.11%   | 2.726531 | 0.7318          | FRAGILE  |  |

## Buy-and-hold passive control (adversarial-review benchmark)

Equal-weight, hold from bar 0 over the SAME N paths and auto-L bootstrap.
Reference: adversarial review p50 Sharpe ≈ +1.78, P(loss) ≈ 4%, p95 MaxDD ≈ 51% at auto-L, N=500.

| row       | p5_sharpe | p50_sharpe | p95_sharpe | prob_loss | P(Sharpe>1) | p95_maxdd | spread   | verdict  |
|-----------|-----------|------------|------------|-----------|-------------|-----------|----------|----------|
| BUYHOLD   | -0.657435 | 1.109353  | 3.202830  | 0.159000 | 0.533000    | 61.80%   | 3.860264 | (passive — no verdict) |

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
