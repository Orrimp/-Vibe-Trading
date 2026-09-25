---
slug: horizon-retest-robustness
scenario: v1-carry-horizon-4h-theta-surface-2023-block-bootstrap-real-fy
generated: 2026-09-25T16:54:01Z
wall_clock_s: 10.2
host: M022517718D
pid: 92205
git_commit: 2d63ddef7964e6a4d5bce6082478ab06f6d04fd8
data_revision_sha: 3a8b96c43f2d8980fd8039303197ff3ac5d01e8f9cebaecdf74c853622dbbfc7
---
# Carry (Funding, 4h horizon) θ-Surface — Parameter-Robustness Sweep — v1-carry-horizon-4h-theta-surface-2023-block-bootstrap-real-fy

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
| held_constant            | score_source=funding_carry direction=momentum exposure_cap=0.50 vol_floor=inert k_short=0 size=equal_weight |
| funding_revision_sha     | bf1ede44e57d797b57e5a4f2743f58027e4eba12d91e1ffaf883dcdd49365668 |

## Carry 4h θ-grid definition (6-cell, LOCKED § D-HR.4-LOCKED — changing this changes the SHA)

grid_definition:
  g=0 l_settlements=6 rebalance_minutes=0 k_long=3 drift=0.10
  g=1 l_settlements=2 rebalance_minutes=0 k_long=3 drift=0.10
  g=2 l_settlements=12 rebalance_minutes=0 k_long=3 drift=0.10
  g=3 l_settlements=6 rebalance_minutes=120 k_long=5 drift=0.10
  g=4 l_settlements=6 rebalance_minutes=0 k_long=1 drift=0.10
  g=5 l_settlements=2 rebalance_minutes=0 k_long=5 drift=0.10

## θ-surface (per-cell distribution + verdict)

Notation: p5/p50/p95 Sharpe at {:.6}; prob_loss / P(Sharpe>1) at {:.6}; p95_maxdd at {:.2}%.
Spread = p95_sharpe − p5_sharpe (interpretive, NOT verdict-forcing).
Verdict: FRAGILE/MARGINAL/ROBUST via 5-signal weakest-link (frozen decision-rule § 0 bands).

funding_harvested = total realized funding cashflow across all N paths (Decimal, D-CARRY.2-LOCKED).

| g  | l_settle | k_long | drift | p5_sharpe | p50_sharpe | p95_sharpe | prob_loss | P(Sharpe>1) | p95_maxdd | spread   | funding_harvested | verdict  | notes |
|----|----------|--------|-------|-----------|------------|------------|-----------|-------------|-----------|----------|--------------------|----------|-------|
|  0 |        6 |      3 | 0.10 | -0.102027 | 0.909185  | 1.775763  | 0.055000 | 0.440000    | 19.61%   | 1.877790 | 255857.33926525179379315557136 | FRAGILE  |  |
|  1 |        2 |      3 | 0.10 | -0.260316 | 0.674796  | 1.446345  | 0.115000 | 0.295000    | 22.18%   | 1.706661 | 282551.48549070396132466481625 | FRAGILE  |  |
|  2 |       12 |      3 | 0.10 | -0.011730 | 0.916125  | 1.738075  | 0.055000 | 0.450000    | 16.83%   | 1.749804 | 219170.99386129337245930059507 | FRAGILE  |  |
|  3 |        6 |      5 | 0.10 | -0.127734 | 1.021960  | 2.037832  | 0.080000 | 0.505000    | 29.37%   | 2.165566 | -147378.85110031481677012697383 | FRAGILE  |  |
|  4 |        6 |      1 | 0.10 | -0.138264 | 0.577412  | 1.392816  | 0.145000 | 0.175000    | 9.59%   | 1.531079 | 343840.46536159241986148053130 | FRAGILE  |  |
|  5 |        2 |      5 | 0.10 | -0.180439 | 1.006482  | 2.033517  | 0.085000 | 0.500000    | 29.10%   | 2.213956 | -43556.585972948207954185850219 | FRAGILE  |  |

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
Conclusion: v1 cross-sectional carry (funding) at the 4h horizon is structurally fragile across the
tested parameter space on this universe. Even at the native settlement cadence,
funding mean-reversion or directional price exposure overwhelmed the funding harvest.

Notes:
- Decision-rule bands: frozen robustness-decision-rule-2026-05-30.md § 0.
- Composite = weakest-link of 5 PRIMARY signals (p5_sharpe, p50_sharpe, prob_loss, P(Sharpe>1), p95_maxdd).
- Spread and p50-vs-real-path are INTERPRETIVE (not verdict-forcing).
- Generator: `block-bootstrap-real` only is anchor-grade.
- Determinism scope: Apple-Silicon canonical box (ADR-0051 D5).
