---
slug: horizon-retest-robustness
scenario: v1-ts-horizon-4h-theta-surface-2023-block-bootstrap-real-fy
generated: 2026-09-25T17:57:49Z
wall_clock_s: 10.5
host: M022517718D
pid: 37829
git_commit: 2d63ddef7964e6a4d5bce6082478ab06f6d04fd8
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
| selected_block_length_L  | 80 (θ-independent — same L for all cells per OQ-3)      |
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

time_in_market = fraction of bars where ≥1 long position was held (mean across N paths, D-TSM.6.4).

| g  | lookback | threshold | k_long | drift | p5_sharpe | p50_sharpe | p95_sharpe | prob_loss | P(Sharpe>1) | p95_maxdd | spread   | time_in_market | verdict  | notes |
|----|----------|-----------|--------|-------|-----------|------------|------------|-----------|-------------|-----------|----------|----------------|----------|-------|
|  0 |       42 | 0.00      |     10 | 0.10 | -0.061313 | 0.849620  | 1.715038  | 0.060000 | 0.380000    | 21.15%   | 1.776351 | 0.8306          | FRAGILE  |  |
|  1 |       42 | 0.02      |     10 | 0.10 | -0.116154 | 0.844400  | 1.769514  | 0.070000 | 0.410000    | 20.20%   | 1.885669 | 0.7492          | FRAGILE  |  |
|  2 |      180 | 0.00      |     10 | 0.10 | -0.336172 | 0.864450  | 1.769677  | 0.100000 | 0.405000    | 22.61%   | 2.105849 | 0.8005          | FRAGILE  |  |
|  3 |      180 | 0.02      |     10 | 0.10 | -0.264908 | 0.845905  | 1.864639  | 0.110000 | 0.410000    | 22.23%   | 2.129547 | 0.7805          | FRAGILE  |  |
|  4 |      540 | 0.00      |     10 | 0.10 | -0.358381 | 0.689496  | 1.756632  | 0.150000 | 0.330000    | 20.87%   | 2.115013 | 0.6615          | FRAGILE  |  |
|  5 |      540 | 0.02      |     10 | 0.10 | -0.348016 | 0.715877  | 1.779712  | 0.165000 | 0.330000    | 20.15%   | 2.127728 | 0.6557          | FRAGILE  |  |

## Buy-and-hold passive control (adversarial-review benchmark)

Equal-weight, hold from bar 0 over the SAME N paths and auto-L bootstrap.
Reference: adversarial review p50 Sharpe ≈ +1.78, P(loss) ≈ 4%, p95 MaxDD ≈ 51% at auto-L, N=500.

| row       | p5_sharpe | p50_sharpe | p95_sharpe | prob_loss | P(Sharpe>1) | p95_maxdd | spread   | verdict  |
|-----------|-----------|------------|------------|-----------|-------------|-----------|----------|----------|
| BUYHOLD   | -0.286815 | 1.910291  | 3.935388  | 0.075000 | 0.795000    | 49.80%   | 4.222203 | (passive — no verdict) |

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
