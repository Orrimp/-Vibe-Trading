---
slug: carry-strategy
scenario: v1-carry-theta-surface-2023-block-bootstrap-real-fy
generated: 2026-10-10T15:13:27Z
wall_clock_s: 37.4
host: M022517718D
pid: 65025
git_commit: c407ca477bde1fcf4364332f885b75c9b5b3ef1e
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
| selected_block_length_L  | 281 (θ-independent — same L for all cells per OQ-3)      |
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
|  0 |        9 |      3 | 0.10 | -0.044270 | 0.767540  | 1.560158  | 0.070000 | 0.355000    | 20.96%   | 1.604428 | 327561.55865278210604553890265 | FRAGILE  |  |
|  1 |        3 |      3 | 0.10 | -0.089037 | 0.673048  | 1.596432  | 0.085000 | 0.280000    | 22.28%   | 1.685469 | 204548.03513015238498426923334 | FRAGILE  |  |
|  2 |       21 |      3 | 0.10 | 0.112521 | 0.879031  | 1.704180  | 0.040000 | 0.415000    | 19.10%   | 1.591660 | 222016.49651985500703909409633 | MARGINAL | → C5 DEFLATION REQUIRED |
|  3 |        9 |      5 | 0.10 | -0.036681 | 0.973309  | 1.912280  | 0.055000 | 0.470000    | 29.28%   | 1.948960 | -147040.86559784913954356794147 | FRAGILE  |  |
|  4 |        9 |      1 | 0.10 | -0.194416 | 0.489240  | 1.100444  | 0.140000 | 0.090000    | 9.06%   | 1.294861 | 391847.07264564580764658100391 | FRAGILE  |  |
|  5 |        3 |      5 | 0.10 | -0.141725 | 0.815787  | 1.853927  | 0.090000 | 0.395000    | 32.27%   | 1.995652 | -152027.62814022982478436036777 | FRAGILE  |  |

## Buy-and-hold passive control (adversarial-review benchmark)

Equal-weight, hold from bar 0 over the SAME N paths and auto-L bootstrap.
Reference: adversarial review p50 Sharpe ≈ +1.78, P(loss) ≈ 4%, p95 MaxDD ≈ 51% at auto-L, N=500.

| row       | p5_sharpe | p50_sharpe | p95_sharpe | prob_loss | P(Sharpe>1) | p95_maxdd | spread   | verdict  |
|-----------|-----------|------------|------------|-----------|-------------|-----------|----------|----------|
| BUYHOLD   | 0.246190 | 1.812105  | 3.586571  | 0.045000 | 0.760000    | 51.73%   | 3.340381 | (passive — no verdict) |

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
