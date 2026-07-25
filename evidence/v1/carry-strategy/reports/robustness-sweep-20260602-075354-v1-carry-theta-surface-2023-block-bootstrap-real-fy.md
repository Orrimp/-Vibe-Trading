---
slug: carry-strategy
scenario: v1-carry-theta-surface-2023-block-bootstrap-real-fy
generated: 2026-06-02T07:53:54Z
wall_clock_s: 30.7
host: M022517718D
pid: 27663
git_commit: c95db8671c7d658835d6ec24232b003ae5df33d1
data_revision_sha: 3a8b96c43f2d8980fd8039303197ff3ac5d01e8f9cebaecdf74c853622dbbfc7
---
# Carry (Funding) θ-Surface — Parameter-Robustness Sweep — v1-carry-theta-surface-2023-block-bootstrap-real-fy

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
|  0 |        9 |      3 | 0.10 | -0.100302 | 0.025150  | 0.051242  | 0.315000 | 0.000000    | 90.77%   | 0.151544 | 3098096.9544009210860872235309 | FRAGILE  |  |
|  1 |        3 |      3 | 0.10 | -0.093271 | 0.015433  | 0.034937  | 0.380000 | 0.000000    | 92.19%   | 0.128208 | 1572693.4630071852065529183502 | FRAGILE  |  |
|  2 |       21 |      3 | 0.10 | -0.132311 | 0.027828  | 0.052803  | 0.225000 | 0.000000    | 90.80%   | 0.185114 | 1997085.7819945746327835765826 | FRAGILE  |  |
|  3 |        9 |      5 | 0.10 | -0.100873 | 0.022161  | 0.067072  | 0.255000 | 0.000000    | 89.89%   | 0.167945 | -1321996.2039871633016259587341 | FRAGILE  |  |
|  4 |        9 |      1 | 0.10 | -0.192058 | 0.038619  | 0.077351  | 0.140000 | 0.000000    | 87.43%   | 0.269410 | 4081758.8449692249732578010967 | FRAGILE  |  |
|  5 |        3 |      5 | 0.10 | -0.071667 | 0.017037  | 0.047261  | 0.235000 | 0.000000    | 91.15%   | 0.118928 | -1626590.1208265514818084481587 | FRAGILE  |  |

## Buy-and-hold passive control (adversarial-review benchmark)

Equal-weight, hold from bar 0 over the SAME N paths and auto-L bootstrap.
Reference: adversarial review p50 Sharpe ≈ +1.78, P(loss) ≈ 4%, p95 MaxDD ≈ 51% at auto-L, N=500.

| row       | p5_sharpe | p50_sharpe | p95_sharpe | prob_loss | P(Sharpe>1) | p95_maxdd | spread   | verdict  |
|-----------|-----------|------------|------------|-----------|-------------|-----------|----------|----------|
| BUYHOLD   | 0.124469 | 1.735275  | 3.870337  | 0.045000 | 0.775000    | 51.15%   | 3.745868 | (passive — no verdict) |

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
