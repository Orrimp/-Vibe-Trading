---
slug: horizon-retest-robustness
scenario: v1-ts-horizon-4h-theta-surface-2024-block-bootstrap-real-fy
generated: 2026-10-10T15:18:48Z
wall_clock_s: 11.8
host: M022517718D
pid: 71925
git_commit: c407ca477bde1fcf4364332f885b75c9b5b3ef1e
data_revision_sha: 3a8b96c43f2d8980fd8039303197ff3ac5d01e8f9cebaecdf74c853622dbbfc7
---
# Time-Series Momentum (4h horizon) θ-Surface — Parameter-Robustness Sweep — v1-ts-horizon-4h-theta-surface-2024-block-bootstrap-real-fy

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
| selected_block_length_L  | 134 (θ-independent — same L for all cells per OQ-3)      |
| source_revision_sha      | 3a8b96c43f2d8980fd8039303197ff3ac5d01e8f9cebaecdf74c853622dbbfc7                                    |
| horizon                  | 4h                                               |
| held_constant            | selection_mode=time_series_long_flat score_source=vol_adjusted_return direction=momentum rebalance_minutes=60 exposure_cap=0.50 k_long=10(inert) vol_floor=inert k_short=0 size=equal_weight |

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

Ruined paths: **0 of 1200** — no path's equity reached zero or below at any bar. Stated rather than left to inference: equity <= 0 is clamped to 1e-6 before the metrics, so `p95_maxdd` near 100% would otherwise be indistinguishable from ruin (bug-log #127).

time_in_market = fraction of bars where ≥1 long position was held (mean across N paths, D-TSM.6.4).

| g  | lookback | threshold | k_long | drift | p5_sharpe | p50_sharpe | p95_sharpe | prob_loss | P(Sharpe>1) | p95_maxdd | spread   | time_in_market | verdict  | notes |
|----|----------|-----------|--------|-------|-----------|------------|------------|-----------|-------------|-----------|----------|----------------|----------|-------|
|  0 |       42 | 0.00      |     10 | 0.10 | -0.461869 | 0.592120  | 1.787092  | 0.135000 | 0.295000    | 31.45%   | 2.248961 | 0.8062          | FRAGILE  |  |
|  1 |       42 | 0.02      |     10 | 0.10 | -0.534002 | 0.534181  | 2.001537  | 0.195000 | 0.320000    | 30.61%   | 2.535539 | 0.7406          | FRAGILE  |  |
|  2 |      180 | 0.00      |     10 | 0.10 | -0.571597 | 0.497539  | 1.501822  | 0.260000 | 0.235000    | 30.83%   | 2.073419 | 0.7713          | FRAGILE  |  |
|  3 |      180 | 0.02      |     10 | 0.10 | -0.589409 | 0.510223  | 1.583491  | 0.255000 | 0.255000    | 29.77%   | 2.172900 | 0.7394          | FRAGILE  |  |
|  4 |      540 | 0.00      |     10 | 0.10 | -0.533863 | 0.502129  | 1.593231  | 0.185000 | 0.255000    | 24.95%   | 2.127093 | 0.6579          | FRAGILE  |  |
|  5 |      540 | 0.02      |     10 | 0.10 | -0.426849 | 0.486185  | 1.701652  | 0.185000 | 0.225000    | 25.54%   | 2.128501 | 0.6502          | FRAGILE  |  |

## Buy-and-hold passive control (adversarial-review benchmark)

Equal-weight, hold from bar 0 over the SAME N paths and auto-L bootstrap.
Reference: adversarial review p50 Sharpe ≈ +1.78, P(loss) ≈ 4%, p95 MaxDD ≈ 51% at auto-L, N=500.

| row       | p5_sharpe | p50_sharpe | p95_sharpe | prob_loss | P(Sharpe>1) | p95_maxdd | spread   | verdict  |
|-----------|-----------|------------|------------|-----------|-------------|-----------|----------|----------|
| BUYHOLD   | -0.608104 | 1.062518  | 2.942900  | 0.115000 | 0.535000    | 63.25%   | 3.551005 | (passive — no verdict) |

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
