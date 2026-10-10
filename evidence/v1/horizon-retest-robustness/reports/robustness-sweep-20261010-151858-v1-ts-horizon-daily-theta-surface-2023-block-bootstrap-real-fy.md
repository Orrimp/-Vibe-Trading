---
slug: horizon-retest-robustness
scenario: v1-ts-horizon-daily-theta-surface-2023-block-bootstrap-real-fy
generated: 2026-10-10T15:18:58Z
wall_clock_s: 9.9
host: M022517718D
pid: 73047
git_commit: 7a09d40cfbf934ddba475cca9444219d6dab5fb0
data_revision_sha: 3a8b96c43f2d8980fd8039303197ff3ac5d01e8f9cebaecdf74c853622dbbfc7
---
# Time-Series Momentum (daily horizon) θ-Surface — Parameter-Robustness Sweep — v1-ts-horizon-daily-theta-surface-2023-block-bootstrap-real-fy

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

Ruined paths: **0 of 6000** — no path's equity reached zero or below at any bar. Stated rather than left to inference: equity <= 0 is clamped to 1e-6 before the metrics, so `p95_maxdd` near 100% would otherwise be indistinguishable from ruin (bug-log #127).

time_in_market = fraction of bars where ≥1 long position was held (mean across N paths, D-TSM.6.4).

| g  | lookback | threshold | k_long | drift | p5_sharpe | p50_sharpe | p95_sharpe | prob_loss | P(Sharpe>1) | p95_maxdd | spread   | time_in_market | verdict  | notes |
|----|----------|-----------|--------|-------|-----------|------------|------------|-----------|-------------|-----------|----------|----------------|----------|-------|
|  0 |        5 | 0.00      |     10 | 0.10 | 0.105287 | 0.941410  | 1.814037  | 0.035000 | 0.448000    | 16.99%   | 1.708750 | 0.8253          | MARGINAL | → C5 DEFLATION REQUIRED |
|  1 |        5 | 0.02      |     10 | 0.10 | -0.000536 | 0.860736  | 1.704700  | 0.050000 | 0.393000    | 16.78%   | 1.705235 | 0.8005          | FRAGILE  |  |
|  2 |       20 | 0.00      |     10 | 0.10 | -0.195925 | 0.810844  | 1.817892  | 0.090000 | 0.391000    | 20.36%   | 2.013816 | 0.8048          | FRAGILE  |  |
|  3 |       20 | 0.02      |     10 | 0.10 | -0.149714 | 0.896752  | 1.897118  | 0.074000 | 0.441000    | 19.67%   | 2.046831 | 0.7731          | FRAGILE  |  |
|  4 |       60 | 0.00      |     10 | 0.10 | -0.273877 | 0.811683  | 1.887724  | 0.114000 | 0.373000    | 19.54%   | 2.161601 | 0.7291          | FRAGILE  |  |
|  5 |       60 | 0.02      |     10 | 0.10 | -0.284354 | 0.784161  | 1.874602  | 0.121000 | 0.360000    | 19.56%   | 2.158956 | 0.7200          | FRAGILE  |  |

## Buy-and-hold passive control (adversarial-review benchmark)

Equal-weight, hold from bar 0 over the SAME N paths and auto-L bootstrap.
Reference: adversarial review p50 Sharpe ≈ +1.78, P(loss) ≈ 4%, p95 MaxDD ≈ 51% at auto-L, N=500.

| row       | p5_sharpe | p50_sharpe | p95_sharpe | prob_loss | P(Sharpe>1) | p95_maxdd | spread   | verdict  |
|-----------|-----------|------------|------------|-----------|-------------|-----------|----------|----------|
| BUYHOLD   | -0.016732 | 1.928787  | 4.007733  | 0.053000 | 0.780000    | 48.61%   | 4.024465 | (passive — no verdict) |

## Family verdict

FAMILY-HAS-NON-FRAGILE-CELLS

At least one cell is MARGINAL or ROBUST. Per the § 0 pre-registration commitment:
C3 makes NO 'this θ is robust' claim. Each non-FRAGILE cell is flagged '→ C5 DEFLATION REQUIRED'
and is handed to the C5 PBO/Deflated-Sharpe pass before any promotion.

Notes:
- Decision-rule bands: frozen robustness-decision-rule-2026-05-30.md § 0.
- Composite = weakest-link of 5 PRIMARY signals (p5_sharpe, p50_sharpe, prob_loss, P(Sharpe>1), p95_maxdd).
- Spread and p50-vs-real-path are INTERPRETIVE (not verdict-forcing).
- Generator: `block-bootstrap-real` only is anchor-grade.
- Determinism scope: Apple-Silicon canonical box (ADR-0051 D5).
