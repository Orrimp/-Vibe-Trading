---
slug: horizon-retest-robustness
scenario: v1-carry-horizon-daily-theta-surface-2023-block-bootstrap-real-fy
generated: 2026-09-25T16:54:22Z
wall_clock_s: 9.0
host: M022517718D
pid: 92589
git_commit: 2d63ddef7964e6a4d5bce6082478ab06f6d04fd8
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
|  0 |        3 |      3 | 0.10 | 0.064576 | 0.881804  | 1.681941  | 0.035000 | 0.394000    | 17.66%   | 1.617366 | 587593.94814503491076612245649 | MARGINAL | → C5 DEFLATION REQUIRED |
|  1 |        1 |      3 | 0.10 | 0.182983 | 0.947563  | 1.689277  | 0.020000 | 0.446000    | 15.90%   | 1.506294 | 495321.78981912901182102834889 | MARGINAL | → C5 DEFLATION REQUIRED |
|  2 |        7 |      3 | 0.10 | 0.097382 | 0.860167  | 1.656805  | 0.034000 | 0.386000    | 17.31%   | 1.559423 | 511166.32102592363470030705455 | MARGINAL | → C5 DEFLATION REQUIRED |
|  3 |        3 |      5 | 0.10 | -0.022410 | 0.903253  | 1.836172  | 0.055000 | 0.434000    | 27.09%   | 1.858582 | -1046872.6733007379345706297829 | FRAGILE  |  |
|  4 |        3 |      1 | 0.10 | -0.216614 | 0.320551  | 0.853190  | 0.168000 | 0.022000    | 8.30%   | 1.069803 | 1097500.8295360498713184109595 | FRAGILE  |  |
|  5 |        7 |      5 | 0.10 | 0.083387 | 1.010073  | 1.916889  | 0.037000 | 0.511000    | 26.77%   | 1.833503 | -1059784.5856799248065179828104 | MARGINAL | → C5 DEFLATION REQUIRED |

## Buy-and-hold passive control (adversarial-review benchmark)

Equal-weight, hold from bar 0 over the SAME N paths and auto-L bootstrap.
Reference: adversarial review p50 Sharpe ≈ +1.78, P(loss) ≈ 4%, p95 MaxDD ≈ 51% at auto-L, N=500.

| row       | p5_sharpe | p50_sharpe | p95_sharpe | prob_loss | P(Sharpe>1) | p95_maxdd | spread   | verdict  |
|-----------|-----------|------------|------------|-----------|-------------|-----------|----------|----------|
| BUYHOLD   | 0.265451 | 1.950642  | 3.910508  | 0.032000 | 0.812000    | 47.46%   | 3.645057 | (passive — no verdict) |

## Family verdict

FAMILY-HAS-NON-FRAGILE-CELLS

At least one cell is MARGINAL or ROBUST. Per the § 0 pre-registration commitment:
C3 makes NO 'this θ is robust' claim. Each non-FRAGILE cell is flagged '→ C5 DEFLATION REQUIRED'
and is handed to the C5 PBO/Deflated-Sharpe pass before any promotion.

Notes:
- Decision-rule bands: frozen robustness-decision-rule-2026-05-30.md § 0.
- Composite = weakest-link of 5 PRIMARY signals (p5_sharpe, p50_sharpe, prob_loss, P(Sharpe>1), p95_maxdd).
- Spread and p50-vs-real-path are INTERPRETIVE (not verdict-forcing).
- Generator: `block-bootstrap-real` only is anchor-grade.
- Determinism scope: Apple-Silicon canonical box (ADR-0051 D5).
