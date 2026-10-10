---
slug: carry-strategy
scenario: v1-carry-theta-surface-2024-block-bootstrap-real-fy
generated: 2026-10-10T15:14:05Z
wall_clock_s: 37.7
host: M022517718D
pid: 65752
git_commit: c407ca477bde1fcf4364332f885b75c9b5b3ef1e
data_revision_sha: 3a8b96c43f2d8980fd8039303197ff3ac5d01e8f9cebaecdf74c853622dbbfc7
---
# Carry (Funding) θ-Surface — Parameter-Robustness Sweep — v1-carry-theta-surface-2024-block-bootstrap-real-fy

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
| selected_block_length_L  | 282 (θ-independent — same L for all cells per OQ-3)      |
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

Ruined paths: **0 of 1200** — no path's equity reached zero or below at any bar. Stated rather than left to inference: equity <= 0 is clamped to 1e-6 before the metrics, so `p95_maxdd` near 100% would otherwise be indistinguishable from ruin (bug-log #127).

funding_harvested = total realized funding cashflow across all N paths (Decimal, D-CARRY.2-LOCKED).

| g  | l_settle | k_long | drift | p5_sharpe | p50_sharpe | p95_sharpe | prob_loss | P(Sharpe>1) | p95_maxdd | spread   | funding_harvested | verdict  | notes |
|----|----------|--------|-------|-----------|------------|------------|-----------|-------------|-----------|----------|--------------------|----------|-------|
|  0 |        9 |      3 | 0.10 | -0.364818 | 0.406405  | 1.235295  | 0.190000 | 0.110000    | 30.68%   | 1.600113 | -228236.99629320570356807343239 | FRAGILE  |  |
|  1 |        3 |      3 | 0.10 | -0.441470 | 0.334360  | 1.186495  | 0.230000 | 0.095000    | 31.87%   | 1.627965 | -332193.42829167893621738798314 | FRAGILE  |  |
|  2 |       21 |      3 | 0.10 | -0.427097 | 0.366727  | 1.138006  | 0.210000 | 0.080000    | 29.24%   | 1.565103 | -313929.85211064848119245775922 | FRAGILE  |  |
|  3 |        9 |      5 | 0.10 | -0.534250 | 0.486743  | 1.495318  | 0.205000 | 0.215000    | 44.47%   | 2.029568 | -991116.3213241226133607434922 | FRAGILE  |  |
|  4 |        9 |      1 | 0.10 | -0.183903 | 0.402533  | 0.951327  | 0.110000 | 0.030000    | 11.82%   | 1.135230 | 158685.32687891822988398555209 | FRAGILE  |  |
|  5 |        3 |      5 | 0.10 | -0.524233 | 0.411739  | 1.357500  | 0.225000 | 0.170000    | 45.35%   | 1.881733 | -982203.9375915946802516815901 | FRAGILE  |  |

## Buy-and-hold passive control (adversarial-review benchmark)

Equal-weight, hold from bar 0 over the SAME N paths and auto-L bootstrap.
Reference: adversarial review p50 Sharpe ≈ +1.78, P(loss) ≈ 4%, p95 MaxDD ≈ 51% at auto-L, N=500.

| row       | p5_sharpe | p50_sharpe | p95_sharpe | prob_loss | P(Sharpe>1) | p95_maxdd | spread   | verdict  |
|-----------|-----------|------------|------------|-----------|-------------|-----------|----------|----------|
| BUYHOLD   | -0.475960 | 0.945434  | 2.613640  | 0.150000 | 0.470000    | 65.86%   | 3.089601 | (passive — no verdict) |

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
