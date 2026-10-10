---
slug: momentum-parameter-robustness-sweep
scenario: v1-momentum-theta-surface-2023-block-bootstrap-real-fy
generated: 2026-10-10T16:00:35Z
wall_clock_s: 1483.6
host: M022517718D
pid: 93558
git_commit: 7a09d40cfbf934ddba475cca9444219d6dab5fb0
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
| selected_block_length_L  | 281 (θ-independent — same L for all cells per OQ-3)      |
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

Ruined paths: **0 of 1200** — no path's equity reached zero or below at any bar. Stated rather than left to inference: equity <= 0 is clamped to 1e-6 before the metrics, so `p95_maxdd` near 100% would otherwise be indistinguishable from ruin (bug-log #127).

| g  | lookback | k_long | drift | p5_sharpe | p50_sharpe | p95_sharpe | prob_loss | P(Sharpe>1) | p95_maxdd | spread   | verdict  | notes |
|----|----------|--------|-------|-----------|------------|------------|-----------|-------------|-----------|----------|----------|-------|
|  0 |       60 |      3 | 0.10 | -1.601516 | -0.792060  | 0.070416  | 0.935000 | 0.000000    | 51.02%   | 1.671932 | FRAGILE  |  |
|  1 |       24 |      3 | 0.10 | -2.299387 | -1.386536  | -0.400564  | 0.980000 | 0.000000    | 64.03%   | 1.898823 | FRAGILE  |  |
|  2 |      168 |      3 | 0.10 | -1.013731 | -0.192708  | 0.787459  | 0.600000 | 0.025000    | 41.31%   | 1.801190 | FRAGILE  |  |
|  3 |      720 |      3 | 0.50 | -0.398208 | 0.467491  | 1.243607  | 0.205000 | 0.125000    | 30.12%   | 1.641816 | FRAGILE  |  |
|  4 |       60 |      1 | 0.10 | -1.230089 | -0.547526  | 0.099679  | 0.930000 | 0.000000    | 28.98%   | 1.329767 | FRAGILE  |  |
|  5 |       60 |      5 | 0.10 | -1.599406 | -0.594172  | 0.447278  | 0.810000 | 0.015000    | 60.32%   | 2.046685 | FRAGILE  |  |

## Buy-and-hold passive control (adversarial-review benchmark)

Equal-weight, hold from bar 0 over the SAME N paths and auto-L bootstrap.
Reference: adversarial review p50 Sharpe ≈ +1.78, P(loss) ≈ 4%, p95 MaxDD ≈ 51% at auto-L, N=500.

| row       | p5_sharpe | p50_sharpe | p95_sharpe | prob_loss | P(Sharpe>1) | p95_maxdd | spread   | verdict  |
|-----------|-----------|------------|------------|-----------|-------------|-----------|----------|----------|
| BUYHOLD   | 0.246190 | 1.812105  | 3.586571  | 0.045000 | 0.760000    | 51.73%   | 3.340381 | (passive — no verdict) |

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
