---
slug: horizon-retest-robustness
scenario: v1-carry-horizon-4h-theta-surface-2024-block-bootstrap-real-fy
generated: 2026-09-25T16:54:13Z
wall_clock_s: 11.2
host: M022517718D
pid: 92423
git_commit: 2d63ddef7964e6a4d5bce6082478ab06f6d04fd8
data_revision_sha: 3a8b96c43f2d8980fd8039303197ff3ac5d01e8f9cebaecdf74c853622dbbfc7
---
# Carry (Funding, 4h horizon) θ-Surface — Parameter-Robustness Sweep — v1-carry-horizon-4h-theta-surface-2024-block-bootstrap-real-fy

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
| selected_block_length_L  | 49 (θ-independent — same L for all cells per OQ-3)      |
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
|  0 |        6 |      3 | 0.10 | -0.288231 | 0.434169  | 1.082376  | 0.170000 | 0.075000    | 26.15%   | 1.370607 | -287490.41851049802573849606410 | FRAGILE  |  |
|  1 |        2 |      3 | 0.10 | -0.364591 | 0.473763  | 1.293201  | 0.190000 | 0.145000    | 29.45%   | 1.657792 | -266459.02305890835040454985207 | FRAGILE  |  |
|  2 |       12 |      3 | 0.10 | -0.096753 | 0.573874  | 1.233540  | 0.090000 | 0.160000    | 22.87%   | 1.330292 | -319188.11862336375610329276471 | FRAGILE  |  |
|  3 |        6 |      5 | 0.10 | -0.471508 | 0.476779  | 1.387699  | 0.210000 | 0.230000    | 43.02%   | 1.859206 | -979375.1304222371115671970156 | FRAGILE  |  |
|  4 |        6 |      1 | 0.10 | -0.215828 | 0.293443  | 0.809892  | 0.180000 | 0.010000    | 9.70%   | 1.025720 | 127292.48943522134400439334840 | FRAGILE  |  |
|  5 |        2 |      5 | 0.10 | -0.556469 | 0.508774  | 1.443283  | 0.205000 | 0.200000    | 45.00%   | 1.999751 | -897296.3901619130034388418004 | FRAGILE  |  |

## Buy-and-hold passive control (adversarial-review benchmark)

Equal-weight, hold from bar 0 over the SAME N paths and auto-L bootstrap.
Reference: adversarial review p50 Sharpe ≈ +1.78, P(loss) ≈ 4%, p95 MaxDD ≈ 51% at auto-L, N=500.

| row       | p5_sharpe | p50_sharpe | p95_sharpe | prob_loss | P(Sharpe>1) | p95_maxdd | spread   | verdict  |
|-----------|-----------|------------|------------|-----------|-------------|-----------|----------|----------|
| BUYHOLD   | -0.519215 | 1.165555  | 2.845719  | 0.125000 | 0.580000    | 63.62%   | 3.364934 | (passive — no verdict) |

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
