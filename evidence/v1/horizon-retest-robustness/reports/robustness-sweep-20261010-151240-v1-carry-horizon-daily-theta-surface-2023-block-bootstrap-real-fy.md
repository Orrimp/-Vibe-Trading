---
slug: horizon-retest-robustness
scenario: v1-carry-horizon-daily-theta-surface-2023-block-bootstrap-real-fy
generated: 2026-10-10T15:12:40Z
wall_clock_s: 9.0
host: M022517718D
pid: 64601
git_commit: c407ca477bde1fcf4364332f885b75c9b5b3ef1e
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
| selected_block_length_L  | 22 (θ-independent — same L for all cells per OQ-3)      |
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

Ruined paths: **0 of 6000** — no path's equity reached zero or below at any bar. Stated rather than left to inference: equity <= 0 is clamped to 1e-6 before the metrics, so `p95_maxdd` near 100% would otherwise be indistinguishable from ruin (bug-log #127).

funding_harvested = total realized funding cashflow across all N paths (Decimal, D-CARRY.2-LOCKED).

| g  | l_settle | k_long | drift | p5_sharpe | p50_sharpe | p95_sharpe | prob_loss | P(Sharpe>1) | p95_maxdd | spread   | funding_harvested | verdict  | notes |
|----|----------|--------|-------|-----------|------------|------------|-----------|-------------|-----------|----------|--------------------|----------|-------|
|  0 |        3 |      3 | 0.10 | -0.031052 | 0.925921  | 1.755296  | 0.057000 | 0.443000    | 17.53%   | 1.786348 | 676598.56530630491010985608466 | FRAGILE  |  |
|  1 |        1 |      3 | 0.10 | 0.175342 | 0.981914  | 1.775118  | 0.022000 | 0.485000    | 16.34%   | 1.599776 | 563266.07594110058445147704099 | MARGINAL | → C5 DEFLATION REQUIRED |
|  2 |        7 |      3 | 0.10 | 0.004166 | 0.920894  | 1.719031  | 0.049000 | 0.446000    | 16.73%   | 1.714864 | 607572.73950106907859343608555 | MARGINAL | → C5 DEFLATION REQUIRED |
|  3 |        3 |      5 | 0.10 | -0.153486 | 0.910907  | 1.883725  | 0.084000 | 0.441000    | 28.48%   | 2.037211 | -1037861.0419657051274073060606 | FRAGILE  |  |
|  4 |        3 |      1 | 0.10 | -0.252574 | 0.265685  | 0.759964  | 0.198000 | 0.012000    | 8.01%   | 1.012538 | 1177757.3975394346471935123072 | FRAGILE  |  |
|  5 |        7 |      5 | 0.10 | 0.028830 | 1.034564  | 1.967567  | 0.046000 | 0.526000    | 26.71%   | 1.938737 | -1042824.6997297508897522726314 | MARGINAL | → C5 DEFLATION REQUIRED |

## Buy-and-hold passive control (adversarial-review benchmark)

Equal-weight, hold from bar 0 over the SAME N paths and auto-L bootstrap.
Reference: adversarial review p50 Sharpe ≈ +1.78, P(loss) ≈ 4%, p95 MaxDD ≈ 51% at auto-L, N=500.

| row       | p5_sharpe | p50_sharpe | p95_sharpe | prob_loss | P(Sharpe>1) | p95_maxdd | spread   | verdict  |
|-----------|-----------|------------|------------|-----------|-------------|-----------|----------|----------|
| BUYHOLD   | -0.016732 | 1.928787  | 4.007733  | 0.053000 | 0.780000    | 48.61%   | 4.024465 | (passive — no verdict) |

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
