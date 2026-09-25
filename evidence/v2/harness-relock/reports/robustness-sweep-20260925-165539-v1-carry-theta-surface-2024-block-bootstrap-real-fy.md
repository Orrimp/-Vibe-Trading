---
slug: carry-strategy
scenario: v1-carry-theta-surface-2024-block-bootstrap-real-fy
generated: 2026-09-25T16:55:39Z
wall_clock_s: 34.2
host: M022517718D
pid: 93392
git_commit: 2d63ddef7964e6a4d5bce6082478ab06f6d04fd8
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
| selected_block_length_L  | 200 (θ-independent — same L for all cells per OQ-3)      |
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
|  0 |        9 |      3 | 0.10 | -0.509354 | 0.495086  | 1.190097  | 0.225000 | 0.155000    | 32.75%   | 1.699452 | -242040.33180755399921222509543 | FRAGILE  |  |
|  1 |        3 |      3 | 0.10 | -0.535550 | 0.419685  | 1.182802  | 0.230000 | 0.140000    | 31.61%   | 1.718352 | -347107.98139527333076963553815 | FRAGILE  |  |
|  2 |       21 |      3 | 0.10 | -0.474908 | 0.410823  | 1.159331  | 0.240000 | 0.105000    | 30.75%   | 1.634239 | -329114.44561233169461711314646 | FRAGILE  |  |
|  3 |        9 |      5 | 0.10 | -0.646305 | 0.624573  | 1.486356  | 0.220000 | 0.270000    | 45.88%   | 2.132661 | -1024822.2180093923507520412429 | FRAGILE  |  |
|  4 |        9 |      1 | 0.10 | -0.223802 | 0.418897  | 1.078408  | 0.145000 | 0.055000    | 11.59%   | 1.302211 | 149730.00570230573985572016182 | FRAGILE  |  |
|  5 |        3 |      5 | 0.10 | -0.637733 | 0.522807  | 1.464089  | 0.245000 | 0.215000    | 45.97%   | 2.101823 | -1014898.3731251949181062446783 | FRAGILE  |  |

## Buy-and-hold passive control (adversarial-review benchmark)

Equal-weight, hold from bar 0 over the SAME N paths and auto-L bootstrap.
Reference: adversarial review p50 Sharpe ≈ +1.78, P(loss) ≈ 4%, p95 MaxDD ≈ 51% at auto-L, N=500.

| row       | p5_sharpe | p50_sharpe | p95_sharpe | prob_loss | P(Sharpe>1) | p95_maxdd | spread   | verdict  |
|-----------|-----------|------------|------------|-----------|-------------|-----------|----------|----------|
| BUYHOLD   | -0.682135 | 1.104731  | 2.690469  | 0.165000 | 0.535000    | 64.83%   | 3.372604 | (passive — no verdict) |

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
