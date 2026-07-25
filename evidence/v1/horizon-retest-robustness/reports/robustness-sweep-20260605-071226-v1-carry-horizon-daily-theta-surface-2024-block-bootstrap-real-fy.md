---
slug: horizon-retest-robustness
scenario: v1-carry-horizon-daily-theta-surface-2024-block-bootstrap-real-fy
generated: 2026-06-05T07:12:26Z
wall_clock_s: 6.9
host: M022517718D
pid: 9069
git_commit: d8f327ccda527cb2ae3cfc38b639457e2c3c7a8d
data_revision_sha: 3a8b96c43f2d8980fd8039303197ff3ac5d01e8f9cebaecdf74c853622dbbfc7
---
# Carry (Funding, daily horizon) θ-Surface — Parameter-Robustness Sweep — v1-carry-horizon-daily-theta-surface-2024-block-bootstrap-real-fy

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
| selected_block_length_L  | 3 (θ-independent — same L for all cells per OQ-3)      |
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
|  0 |        3 |      3 | 0.10 | -0.015528 | 0.030382  | 0.076270  | 0.103000 | 0.000000    | 76.34%   | 0.091799 | -868653.7418854767382619997540 | FRAGILE  |  |
|  1 |        1 |      3 | 0.10 | -0.023025 | 0.023765  | 0.060353  | 0.130000 | 0.000000    | 80.56%   | 0.083378 | -861812.2037940666398894853411 | FRAGILE  |  |
|  2 |        7 |      3 | 0.10 | -0.010638 | 0.050328  | 0.109370  | 0.077000 | 0.000000    | 71.97%   | 0.120009 | -882329.9839712079328734964283 | FRAGILE  |  |
|  3 |        3 |      5 | 0.10 | -0.022210 | 0.038591  | 0.109534  | 0.135000 | 0.000000    | 77.40%   | 0.131745 | -2303538.9033790993590996345442 | FRAGILE  |  |
|  4 |        3 |      1 | 0.10 | 0.004721 | 0.040031  | 0.072307  | 0.032000 | 0.000000    | 59.08%   | 0.067586 | 105989.33884279022256288424834 | FRAGILE  |  |
|  5 |        7 |      5 | 0.10 | -0.017350 | 0.058200  | 0.152274  | 0.109000 | 0.000000    | 75.56%   | 0.169624 | -2407358.4483651866136686329127 | FRAGILE  |  |

## Buy-and-hold passive control (adversarial-review benchmark)

Equal-weight, hold from bar 0 over the SAME N paths and auto-L bootstrap.
Reference: adversarial review p50 Sharpe ≈ +1.78, P(loss) ≈ 4%, p95 MaxDD ≈ 51% at auto-L, N=500.

| row       | p5_sharpe | p50_sharpe | p95_sharpe | prob_loss | P(Sharpe>1) | p95_maxdd | spread   | verdict  |
|-----------|-----------|------------|------------|-----------|-------------|-----------|----------|----------|
| BUYHOLD   | -0.549673 | 1.148085  | 2.811356  | 0.149000 | 0.543000    | 63.95%   | 3.361029 | (passive — no verdict) |

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
