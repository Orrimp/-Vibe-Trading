---
slug: horizon-retest-robustness
scenario: v1-carry-horizon-4h-theta-surface-2024-block-bootstrap-real-fy
generated: 2026-06-05T07:12:09Z
wall_clock_s: 7.1
host: M022517718D
pid: 8782
git_commit: d8f327ccda527cb2ae3cfc38b639457e2c3c7a8d
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
|  0 |        6 |      3 | 0.10 | -0.012134 | 0.014255  | 0.051343  | 0.160000 | 0.000000    | 79.67%   | 0.063477 | -773395.24326642502331771273748 | FRAGILE  |  |
|  1 |        2 |      3 | 0.10 | -0.012880 | 0.014453  | 0.042149  | 0.130000 | 0.000000    | 84.41%   | 0.055029 | -716552.30102686403086025270309 | FRAGILE  |  |
|  2 |       12 |      3 | 0.10 | -0.006710 | 0.020929  | 0.058689  | 0.120000 | 0.000000    | 77.37%   | 0.065399 | -820482.5727085868757359871380 | FRAGILE  |  |
|  3 |        6 |      5 | 0.10 | -0.036233 | 0.015052  | 0.063996  | 0.315000 | 0.000000    | 82.10%   | 0.100230 | -2609129.4727854052427041885845 | FRAGILE  |  |
|  4 |        6 |      1 | 0.10 | 0.006120 | 0.032067  | 0.073432  | 0.010000 | 0.000000    | 67.18%   | 0.067311 | 332103.48848960647873634861913 | FRAGILE  |  |
|  5 |        2 |      5 | 0.10 | -0.021484 | 0.020023  | 0.057849  | 0.225000 | 0.000000    | 87.89%   | 0.079333 | -2702513.2054947025245384744022 | FRAGILE  |  |

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
