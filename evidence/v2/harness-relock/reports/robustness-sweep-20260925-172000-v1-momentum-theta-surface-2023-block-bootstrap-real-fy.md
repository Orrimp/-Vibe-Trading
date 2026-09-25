---
slug: momentum-parameter-robustness-sweep
scenario: v1-momentum-theta-surface-2023-block-bootstrap-real-fy
generated: 2026-09-25T17:20:00Z
wall_clock_s: 1460.8
host: M022517718D
pid: 93854
git_commit: 2d63ddef7964e6a4d5bce6082478ab06f6d04fd8
data_revision_sha: 3a8b96c43f2d8980fd8039303197ff3ac5d01e8f9cebaecdf74c853622dbbfc7
---
# Momentum θ-Surface — Parameter-Robustness Sweep — v1-momentum-theta-surface-2023-block-bootstrap-real-fy

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
| selected_block_length_L  | 204 (θ-independent — same L for all cells per OQ-3)      |
| source_revision_sha      | 3a8b96c43f2d8980fd8039303197ff3ac5d01e8f9cebaecdf74c853622dbbfc7                                    |
| held_constant            | rebalance_minutes=60 exposure_cap=0.50 vol_floor=0.000001 k_short=0 size=equal_weight |

## Re-scoped θ-grid definition (6-cell, 2026-05-30 orchestrator re-scope — changing this changes the SHA)

grid_definition:
  g=0 lookback=60 k_long=3 drift=0.10
  g=1 lookback=24 k_long=3 drift=0.10
  g=2 lookback=168 k_long=3 drift=0.10
  g=3 lookback=720 k_long=3 drift=0.50
  g=4 lookback=60 k_long=1 drift=0.10
  g=5 lookback=60 k_long=5 drift=0.10

## θ-surface (per-cell distribution + verdict)

Notation: p5/p50/p95 Sharpe at {:.6}; prob_loss / P(Sharpe>1) at {:.6}; p95_maxdd at {:.2}%.
Spread = p95_sharpe − p5_sharpe (interpretive, NOT verdict-forcing).
Verdict: FRAGILE/MARGINAL/ROBUST via 5-signal weakest-link (frozen decision-rule § 0 bands).

| g  | lookback | k_long | drift | p5_sharpe | p50_sharpe | p95_sharpe | prob_loss | P(Sharpe>1) | p95_maxdd | spread   | verdict  | notes |
|----|----------|--------|-------|-----------|------------|------------|-----------|-------------|-----------|----------|----------|-------|
|  0 |       60 |      3 | 0.10 | -1.503930 | -0.726497  | 0.042383  | 0.920000 | 0.000000    | 50.01%   | 1.546313 | FRAGILE  |  |
|  1 |       24 |      3 | 0.10 | -2.196036 | -1.345299  | -0.426132  | 0.995000 | 0.000000    | 61.55%   | 1.769904 | FRAGILE  |  |
|  2 |      168 |      3 | 0.10 | -0.958117 | -0.147917  | 0.770518  | 0.595000 | 0.010000    | 38.50%   | 1.728635 | FRAGILE  |  |
|  3 |      720 |      3 | 0.50 | -0.314090 | 0.458374  | 1.311550  | 0.195000 | 0.105000    | 25.64%   | 1.625639 | FRAGILE  |  |
|  4 |       60 |      1 | 0.10 | -1.182754 | -0.524792  | 0.040876  | 0.930000 | 0.000000    | 28.55%   | 1.223629 | FRAGILE  |  |
|  5 |       60 |      5 | 0.10 | -1.530237 | -0.571987  | 0.488593  | 0.830000 | 0.005000    | 58.19%   | 2.018830 | FRAGILE  |  |

## Buy-and-hold passive control (adversarial-review benchmark)

Equal-weight, hold from bar 0 over the SAME N paths and auto-L bootstrap.
Reference: adversarial review p50 Sharpe ≈ +1.78, P(loss) ≈ 4%, p95 MaxDD ≈ 51% at auto-L, N=500.

| row       | p5_sharpe | p50_sharpe | p95_sharpe | prob_loss | P(Sharpe>1) | p95_maxdd | spread   | verdict  |
|-----------|-----------|------------|------------|-----------|-------------|-----------|----------|----------|
| BUYHOLD   | 0.124469 | 1.735275  | 3.870337  | 0.045000 | 0.775000    | 51.15%   | 3.745868 | (passive — no verdict) |

## Family verdict

FAMILY-UNIFORM-FRAGILE

Every active θ-cell is FRAGILE under the frozen decision-rule bands.
No multiple-testing correction is needed for a uniform-negative result:
C3 is not selecting a winner — it is reporting that no cell cleared the bar.
Conclusion: v1 cross-sectional momentum is structurally fragile across the
tested parameter space. The turnover/fee-bleed is not tunable away within
the Tier-1 grid (lookback × k_long × drift_rebalance_threshold).

Notes:
- Decision-rule bands: frozen robustness-decision-rule-2026-05-30.md § 0.
- Composite = weakest-link of 5 PRIMARY signals (p5_sharpe, p50_sharpe, prob_loss, P(Sharpe>1), p95_maxdd).
- Spread and p50-vs-real-path are INTERPRETIVE (not verdict-forcing).
- Generator: `block-bootstrap-real` only is anchor-grade.
- Determinism scope: Apple-Silicon canonical box (ADR-0051 D5).
