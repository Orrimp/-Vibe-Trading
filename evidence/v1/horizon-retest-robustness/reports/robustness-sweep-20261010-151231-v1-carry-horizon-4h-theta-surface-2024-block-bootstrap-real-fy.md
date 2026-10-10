---
slug: horizon-retest-robustness
scenario: v1-carry-horizon-4h-theta-surface-2024-block-bootstrap-real-fy
generated: 2026-10-10T15:12:31Z
wall_clock_s: 10.9
host: M022517718D
pid: 64403
git_commit: c407ca477bde1fcf4364332f885b75c9b5b3ef1e
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
| selected_block_length_L  | 134 (θ-independent — same L for all cells per OQ-3)      |
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

Ruined paths: **0 of 1200** — no path's equity reached zero or below at any bar. Stated rather than left to inference: equity <= 0 is clamped to 1e-6 before the metrics, so `p95_maxdd` near 100% would otherwise be indistinguishable from ruin (bug-log #127).

funding_harvested = total realized funding cashflow across all N paths (Decimal, D-CARRY.2-LOCKED).

| g  | l_settle | k_long | drift | p5_sharpe | p50_sharpe | p95_sharpe | prob_loss | P(Sharpe>1) | p95_maxdd | spread   | funding_harvested | verdict  | notes |
|----|----------|--------|-------|-----------|------------|------------|-----------|-------------|-----------|----------|--------------------|----------|-------|
|  0 |        6 |      3 | 0.10 | -0.355689 | 0.364686  | 1.112510  | 0.195000 | 0.130000    | 25.19%   | 1.468199 | -293139.28469471781062500063096 | FRAGILE  |  |
|  1 |        2 |      3 | 0.10 | -0.461254 | 0.406497  | 1.324889  | 0.190000 | 0.155000    | 28.98%   | 1.786142 | -272946.99459040181952534326381 | FRAGILE  |  |
|  2 |       12 |      3 | 0.10 | -0.138346 | 0.570029  | 1.289385  | 0.100000 | 0.200000    | 22.29%   | 1.427732 | -322554.60848753766985539065730 | FRAGILE  |  |
|  3 |        6 |      5 | 0.10 | -0.603907 | 0.407952  | 1.595726  | 0.230000 | 0.230000    | 44.15%   | 2.199633 | -996462.0035781527501321271111 | FRAGILE  |  |
|  4 |        6 |      1 | 0.10 | -0.220298 | 0.279928  | 0.800876  | 0.170000 | 0.000000    | 10.14%   | 1.021173 | 119636.01259174646708331775735 | FRAGILE  |  |
|  5 |        2 |      5 | 0.10 | -0.711470 | 0.446912  | 1.557240  | 0.260000 | 0.215000    | 46.11%   | 2.268710 | -924758.7018779071645456195843 | FRAGILE  |  |

## Buy-and-hold passive control (adversarial-review benchmark)

Equal-weight, hold from bar 0 over the SAME N paths and auto-L bootstrap.
Reference: adversarial review p50 Sharpe ≈ +1.78, P(loss) ≈ 4%, p95 MaxDD ≈ 51% at auto-L, N=500.

| row       | p5_sharpe | p50_sharpe | p95_sharpe | prob_loss | P(Sharpe>1) | p95_maxdd | spread   | verdict  |
|-----------|-----------|------------|------------|-----------|-------------|-----------|----------|----------|
| BUYHOLD   | -0.608104 | 1.062518  | 2.942900  | 0.115000 | 0.535000    | 63.25%   | 3.551005 | (passive — no verdict) |

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
