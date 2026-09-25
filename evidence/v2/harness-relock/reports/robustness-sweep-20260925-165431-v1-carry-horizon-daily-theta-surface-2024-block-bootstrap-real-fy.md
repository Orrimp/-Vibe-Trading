---
slug: horizon-retest-robustness
scenario: v1-carry-horizon-daily-theta-surface-2024-block-bootstrap-real-fy
generated: 2026-09-25T16:54:31Z
wall_clock_s: 9.3
host: M022517718D
pid: 92776
git_commit: 2d63ddef7964e6a4d5bce6082478ab06f6d04fd8
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
|  0 |        3 |      3 | 0.10 | -0.365435 | 0.418366  | 1.146531  | 0.214000 | 0.102000    | 27.59%   | 1.511966 | -1947257.5986257689951241975300 | FRAGILE  |  |
|  1 |        1 |      3 | 0.10 | -0.480388 | 0.380359  | 1.117606  | 0.237000 | 0.086000    | 30.66%   | 1.597994 | -1909709.2403301176134459434156 | FRAGILE  |  |
|  2 |        7 |      3 | 0.10 | -0.330043 | 0.458478  | 1.215880  | 0.175000 | 0.123000    | 27.53%   | 1.545923 | -1952547.2427931336992980341055 | FRAGILE  |  |
|  3 |        3 |      5 | 0.10 | -0.418900 | 0.599880  | 1.541118  | 0.187000 | 0.240000    | 42.34%   | 1.960019 | -5092891.1946387856603789047310 | FRAGILE  |  |
|  4 |        3 |      1 | 0.10 | -0.162650 | 0.388034  | 0.849320  | 0.131000 | 0.019000    | 9.65%   | 1.011971 | 255754.31886607042397451810853 | FRAGILE  |  |
|  5 |        7 |      5 | 0.10 | -0.445646 | 0.558203  | 1.496352  | 0.195000 | 0.228000    | 41.62%   | 1.941998 | -5010433.4163639624147942179184 | FRAGILE  |  |

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
