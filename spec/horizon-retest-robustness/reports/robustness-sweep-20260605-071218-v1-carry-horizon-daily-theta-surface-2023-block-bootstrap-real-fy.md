---
slug: horizon-retest-robustness
scenario: v1-carry-horizon-daily-theta-surface-2023-block-bootstrap-real-fy
generated: 2026-06-05T07:12:17Z
wall_clock_s: 6.4
host: M022517718D
pid: 8925
git_commit: d8f327ccda527cb2ae3cfc38b639457e2c3c7a8d
data_revision_sha: 3a8b96c43f2d8980fd8039303197ff3ac5d01e8f9cebaecdf74c853622dbbfc7
---
# Carry (Funding, daily horizon) θ-Surface — Parameter-Robustness Sweep — v1-carry-horizon-daily-theta-surface-2023-block-bootstrap-real-fy

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
| selected_block_length_L  | 9 (θ-independent — same L for all cells per OQ-3)      |
| source_revision_sha      | 3a8b96c43f2d8980fd8039303197ff3ac5d01e8f9cebaecdf74c853622dbbfc7                                    |
| horizon                  | daily                                               |
| held_constant            | score_source=funding_carry direction=momentum exposure_cap=0.50 vol_floor=inert k_short=0 size=equal_weight |
| funding_revision_sha     | bf1ede44e57d797b57e5a4f2743f58027e4eba12d91e1ffaf883dcdd49365668 |

## Carry daily θ-grid definition (6-cell, LOCKED § D-HR.4-LOCKED — changing this changes the SHA)

grid_definition:
  g=0 l_settlements=3 rebalance_minutes=0 k_long=3 drift=0.10
  g=1 l_settlements=1 rebalance_minutes=0 k_long=3 drift=0.10
  g=2 l_settlements=7 rebalance_minutes=0 k_long=3 drift=0.10
  g=3 l_settlements=3 rebalance_minutes=0 k_long=5 drift=0.10
  g=4 l_settlements=3 rebalance_minutes=0 k_long=1 drift=0.10
  g=5 l_settlements=7 rebalance_minutes=0 k_long=5 drift=0.10

## θ-surface (per-cell distribution + verdict)

Notation: p5/p50/p95 Sharpe at {:.6}; prob_loss / P(Sharpe>1) at {:.6}; p95_maxdd at {:.2}%.
Spread = p95_sharpe − p5_sharpe (interpretive, NOT verdict-forcing).
Verdict: FRAGILE/MARGINAL/ROBUST via 5-signal weakest-link (frozen decision-rule § 0 bands).

funding_harvested = total realized funding cashflow across all N paths (Decimal, D-CARRY.2-LOCKED).

| g  | l_settle | k_long | drift | p5_sharpe | p50_sharpe | p95_sharpe | prob_loss | P(Sharpe>1) | p95_maxdd | spread   | funding_harvested | verdict  | notes |
|----|----------|--------|-------|-----------|------------|------------|-----------|-------------|-----------|----------|--------------------|----------|-------|
|  0 |        3 |      3 | 0.10 | -0.098192 | 0.039149  | 0.079421  | 0.173000 | 0.000000    | 85.55%   | 0.177612 | 266760.94217938173991826470449 | FRAGILE  |  |
|  1 |        1 |      3 | 0.10 | -0.098661 | 0.017633  | 0.045005  | 0.264000 | 0.000000    | 88.46%   | 0.143665 | 167622.83389555130552653954579 | FRAGILE  |  |
|  2 |        7 |      3 | 0.10 | -0.049119 | 0.050721  | 0.098842  | 0.089000 | 0.000000    | 77.67%   | 0.147961 | 251708.29867256912094964724967 | FRAGILE  |  |
|  3 |        3 |      5 | 0.10 | -0.033418 | 0.049315  | 0.102164  | 0.113000 | 0.000000    | 83.95%   | 0.135582 | -456924.28938809218675444533800 | FRAGILE  |  |
|  4 |        3 |      1 | 0.10 | -0.008552 | 0.035173  | 0.073012  | 0.078000 | 0.000000    | 72.07%   | 0.081564 | 477067.51950359139312428973433 | FRAGILE  |  |
|  5 |        7 |      5 | 0.10 | -0.015937 | 0.064869  | 0.132162  | 0.080000 | 0.000000    | 78.50%   | 0.148098 | -451517.80881154253032309132740 | FRAGILE  |  |

## Buy-and-hold passive control (adversarial-review benchmark)

Equal-weight, hold from bar 0 over the SAME N paths and auto-L bootstrap.
Reference: adversarial review p50 Sharpe ≈ +1.78, P(loss) ≈ 4%, p95 MaxDD ≈ 51% at auto-L, N=500.

| row       | p5_sharpe | p50_sharpe | p95_sharpe | prob_loss | P(Sharpe>1) | p95_maxdd | spread   | verdict  |
|-----------|-----------|------------|------------|-----------|-------------|-----------|----------|----------|
| BUYHOLD   | 0.265451 | 1.950642  | 3.910508  | 0.032000 | 0.812000    | 47.46%   | 3.645057 | (passive — no verdict) |

## Family verdict

FAMILY-UNIFORM-FRAGILE

Every active θ-cell is FRAGILE under the frozen decision-rule bands.
No multiple-testing correction is needed for a uniform-negative result:
C3 is not selecting a winner — it is reporting that no cell cleared the bar.
Conclusion: v1 cross-sectional carry (funding) at the daily horizon is structurally fragile across the
tested parameter space on this universe. Even at the native settlement cadence,
funding mean-reversion or directional price exposure overwhelmed the funding harvest.

Notes:
- Decision-rule bands: frozen robustness-decision-rule-2026-05-30.md § 0.
- Composite = weakest-link of 5 PRIMARY signals (p5_sharpe, p50_sharpe, prob_loss, P(Sharpe>1), p95_maxdd).
- Spread and p50-vs-real-path are INTERPRETIVE (not verdict-forcing).
- Generator: `block-bootstrap-real` only is anchor-grade.
- Determinism scope: Apple-Silicon canonical box (ADR-0051 D5).
