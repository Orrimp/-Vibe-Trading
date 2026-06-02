---
slug: carry-strategy
scenario: v1-carry-theta-surface-2023-block-bootstrap-real-fy
generated: 2026-06-02T08:34:34Z
wall_clock_s: 18.3
host: M022517718D
pid: 71849
git_commit: 72d711c67d4f17e556a93b97291e98e626bd17a4
data_revision_sha: 3a8b96c43f2d8980fd8039303197ff3ac5d01e8f9cebaecdf74c853622dbbfc7
---
# Carry (Funding) θ-Surface — Parameter-Robustness Sweep — v1-carry-theta-surface-2023-block-bootstrap-real-fy

## Ensemble parameters (shared across all θ-cells)

| Field                    | Value                                                   |
|--------------------------|----------------------------------------------------------|
| master_seed              | 0xC0FFEE                                          |
| fill_seed                | 0xC0FFEE                                          |
| n_paths                  | 3                                                 |
| sub_seed_rule            | "master + j*0x9E3779B9 (SAME paths across cells, ADR-0051 D6.1)" |
| reduction_rule           | "index-order mean/std; total_cmp sort; type-7 linear pct" |
| generator                | block-bootstrap-real                                        |
| bootstrap_mode           | shared-index                                         |
| block_length_policy      | auto                                    |
| selected_block_length_L  | 204 (θ-independent — same L for all cells per OQ-3)      |
| source_revision_sha      | 3a8b96c43f2d8980fd8039303197ff3ac5d01e8f9cebaecdf74c853622dbbfc7                                    |
| held_constant            | score_source=funding_carry direction=momentum exposure_cap=0.50 vol_floor=inert k_short=0 size=equal_weight |
| funding_revision_sha     | bf1ede44e57d797b57e5a4f2743f58027e4eba12d91e1ffaf883dcdd49365668 |

## Carry θ-grid definition (6-cell, LOCKED § D-CARRY.2-LOCKED — changing this changes the SHA)

grid_definition:
  g=0 l_settlements=9 rebalance_minutes=480 k_long=3 drift=0.10
  g=1 l_settlements=3 rebalance_minutes=480 k_long=3 drift=0.10
  g=2 l_settlements=21 rebalance_minutes=480 k_long=3 drift=0.10
  g=3 l_settlements=9 rebalance_minutes=1440 k_long=5 drift=0.10
  g=4 l_settlements=9 rebalance_minutes=480 k_long=1 drift=0.10
  g=5 l_settlements=3 rebalance_minutes=480 k_long=5 drift=0.10

## θ-surface (per-cell distribution + verdict)

Notation: p5/p50/p95 Sharpe at {:.6}; prob_loss / P(Sharpe>1) at {:.6}; p95_maxdd at {:.2}%.
Spread = p95_sharpe − p5_sharpe (interpretive, NOT verdict-forcing).
Verdict: FRAGILE/MARGINAL/ROBUST via 5-signal weakest-link (frozen decision-rule § 0 bands).

funding_harvested = total realized funding cashflow across all N paths (Decimal, D-CARRY.2-LOCKED).

| g  | l_settle | k_long | drift | p5_sharpe | p50_sharpe | p95_sharpe | prob_loss | P(Sharpe>1) | p95_maxdd | spread   | funding_harvested | verdict  | notes |
|----|----------|--------|-------|-----------|------------|------------|-----------|-------------|-----------|----------|--------------------|----------|-------|
|  0 |        9 |      3 | 0.10 | 0.015003 | 0.024577  | 0.041951  | 0.000000 | 0.000000    | 74.97%   | 0.026948 | 47387.368143407699914882218491 | FRAGILE  |  |
|  1 |        3 |      3 | 0.10 | 0.013340 | 0.018098  | 0.029795  | 0.000000 | 0.000000    | 75.46%   | 0.016455 | 25273.648404576131379867957570 | FRAGILE  |  |
|  2 |       21 |      3 | 0.10 | 0.009325 | 0.021215  | 0.045135  | 0.000000 | 0.000000    | 72.94%   | 0.035809 | 25031.750123733171969028407240 | FRAGILE  |  |
|  3 |        9 |      5 | 0.10 | 0.018333 | 0.027341  | 0.029579  | 0.000000 | 0.000000    | 84.76%   | 0.011246 | -31827.958923939865249828037248 | FRAGILE  |  |
|  4 |        9 |      1 | 0.10 | 0.018949 | 0.028793  | 0.042322  | 0.000000 | 0.000000    | 68.58%   | 0.023374 | 54945.111897482135318535065066 | FRAGILE  |  |
|  5 |        3 |      5 | 0.10 | -0.029625 | -0.012280  | -0.006720  | 1.000000 | 0.000000    | 85.03%   | 0.022906 | -26303.253073318239925025263905 | FRAGILE  |  |

## Buy-and-hold passive control (adversarial-review benchmark)

Equal-weight, hold from bar 0 over the SAME N paths and auto-L bootstrap.
Reference: adversarial review p50 Sharpe ≈ +1.78, P(loss) ≈ 4%, p95 MaxDD ≈ 51% at auto-L, N=500.

| row       | p5_sharpe | p50_sharpe | p95_sharpe | prob_loss | P(Sharpe>1) | p95_maxdd | spread   | verdict  |
|-----------|-----------|------------|------------|-----------|-------------|-----------|----------|----------|
| BUYHOLD   | 0.534599 | 1.235081  | 1.857133  | 0.000000 | 0.666667    | 38.33%   | 1.322534 | (passive — no verdict) |

## Family verdict

FAMILY-UNIFORM-FRAGILE

Every active θ-cell is FRAGILE under the frozen decision-rule bands.
No multiple-testing correction is needed for a uniform-negative result:
C3 is not selecting a winner — it is reporting that no cell cleared the bar.
Conclusion: v1 cross-sectional carry (funding) is structurally fragile across the
tested parameter space on this universe (2023-FY resampled). Funding mean-reversion
or directional price exposure may have overwhelmed the funding harvest.

Notes:
- Decision-rule bands: frozen robustness-decision-rule-2026-05-30.md § 0.
- Composite = weakest-link of 5 PRIMARY signals (p5_sharpe, p50_sharpe, prob_loss, P(Sharpe>1), p95_maxdd).
- Spread and p50-vs-real-path are INTERPRETIVE (not verdict-forcing).
- Generator: `block-bootstrap-real` only is anchor-grade.
- Determinism scope: Apple-Silicon canonical box (ADR-0051 D5).
