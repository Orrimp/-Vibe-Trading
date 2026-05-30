---
slug: momentum-parameter-robustness-sweep
scenario: v1-momentum-theta-surface-2023-block-bootstrap-real-fy
generated: 2026-05-30T18:00:06Z
wall_clock_s: 1217.1
host: M022517718D
pid: 43296
git_commit: 27c00d457daf36585960755a555ca68d0991f814
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
|  0 |       60 |      3 | 0.10 | -0.049054 | -0.008105  | 0.009759  | 0.760000 | 0.000000    | 91.48%   | 0.058812 | FRAGILE  |  |
|  1 |       24 |      3 | 0.10 | -0.048184 | -0.021493  | 0.001735  | 0.935000 | 0.000000    | 93.30%   | 0.049918 | FRAGILE  |  |
|  2 |      168 |      3 | 0.10 | -0.058325 | 0.001664  | 0.017433  | 0.450000 | 0.000000    | 88.20%   | 0.075758 | FRAGILE  |  |
|  3 |      720 |      3 | 0.50 | -0.032040 | 0.013730  | 0.048227  | 0.185000 | 0.000000    | 81.74%   | 0.080267 | FRAGILE  |  |
|  4 |       60 |      1 | 0.10 | -0.077302 | -0.007039  | 0.003774  | 0.830000 | 0.000000    | 89.28%   | 0.081075 | FRAGILE  |  |
|  5 |       60 |      5 | 0.10 | -0.046152 | -0.004590  | 0.016588  | 0.615000 | 0.000000    | 92.03%   | 0.062740 | FRAGILE  |  |

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
