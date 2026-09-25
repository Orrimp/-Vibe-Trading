---
slug: perp-basis-signal-robustness
scenario: v1-basis-reversal-fee00bps-theta-surface-2023-block-bootstrap-real-fy
generated: 2026-09-25T16:49:29Z
wall_clock_s: 38.0
host: M022517718D
pid: 87885
git_commit: 2d63ddef7964e6a4d5bce6082478ab06f6d04fd8
data_revision_sha: 3a8b96c43f2d8980fd8039303197ff3ac5d01e8f9cebaecdf74c853622dbbfc7
---
# Basis-Reversal (taker_fee=0bps) θ-Surface — Parameter-Robustness Sweep — v1-basis-reversal-fee00bps-theta-surface-2023-block-bootstrap-real-fy

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
| taker_fee_bps            | 0                                                   |
| slippage_bps             | 2                                                    |
| held_constant            | score_source=basis_reversal direction=momentum exposure_cap=0.50 vol_floor=inert k_short=0 size=equal_weight |
| basis_revision_sha       | aa72409aa0f856960385a823bc61be1b8274e84f658439b65e5d1b1b1a48f1cd |

## Basis-Reversal θ-grid definition (6-cell, LOCKED § D-BR.2-LOCKED — changing this changes the SHA)

grid_definition:
  g=0 lookback_bars=60 rebalance_minutes=480 k_long=3 drift=0.10
  g=1 lookback_bars=24 rebalance_minutes=480 k_long=3 drift=0.10
  g=2 lookback_bars=168 rebalance_minutes=480 k_long=3 drift=0.10
  g=3 lookback_bars=60 rebalance_minutes=1440 k_long=5 drift=0.10
  g=4 lookback_bars=60 rebalance_minutes=480 k_long=1 drift=0.10
  g=5 lookback_bars=24 rebalance_minutes=480 k_long=5 drift=0.10

## θ-surface (per-cell distribution + verdict)

Notation: p5/p50/p95 Sharpe at {:.6}; prob_loss / P(Sharpe>1) at {:.6}; p95_maxdd at {:.2}%.
Spread = p95_sharpe − p5_sharpe (interpretive, NOT verdict-forcing).
Verdict: FRAGILE/MARGINAL/ROBUST via 5-signal weakest-link (frozen decision-rule § 0 bands).

Trades = total trade count across all N paths (turnover legibility — fee story for reversal arm, D-BR.2-LOCKED).

| g  | lookback | rebalance | k_long | drift | p5_sharpe | p50_sharpe | p95_sharpe | prob_loss | P(Sharpe>1) | p95_maxdd | spread   | trades     | verdict  | notes |
|----|----------|-----------|--------|-------|-----------|------------|------------|-----------|-------------|-----------|----------|------------|----------|-------|
|  0 |       60 |       480 |      3 | 0.10 | 0.178441 | 0.907964  | 1.790987  | 0.030000 | 0.405000    | 15.81%   | 1.612546 |     165001 | MARGINAL | → C5 DEFLATION REQUIRED |
|  1 |       24 |       480 |      3 | 0.10 | 0.178681 | 0.880491  | 1.778512  | 0.030000 | 0.405000    | 17.13%   | 1.599831 |     292625 | MARGINAL | → C5 DEFLATION REQUIRED |
|  2 |      168 |       480 |      3 | 0.10 | 0.144744 | 0.929189  | 1.785446  | 0.035000 | 0.440000    | 15.96%   | 1.640701 |      79004 | MARGINAL | → C5 DEFLATION REQUIRED |
|  3 |       60 |      1440 |      5 | 0.10 | 0.156259 | 1.103340  | 2.218223  | 0.025000 | 0.575000    | 25.41%   | 2.061964 |     141029 | MARGINAL | → C5 DEFLATION REQUIRED |
|  4 |       60 |       480 |      1 | 0.10 | -0.088853 | 0.519782  | 1.144217  | 0.095000 | 0.110000    | 8.06%   | 1.233070 |      64381 | FRAGILE  |  |
|  5 |       24 |       480 |      5 | 0.10 | 0.091352 | 1.015307  | 2.055806  | 0.025000 | 0.505000    | 26.29%   | 1.964454 |     344081 | MARGINAL | → C5 DEFLATION REQUIRED |

## Buy-and-hold passive control (adversarial-review benchmark)

Equal-weight, hold from bar 0 over the SAME N paths and auto-L bootstrap.
Reference: adversarial review p50 Sharpe ≈ +1.78, P(loss) ≈ 4%, p95 MaxDD ≈ 51% at auto-L, N=500.

| row       | p5_sharpe | p50_sharpe | p95_sharpe | prob_loss | P(Sharpe>1) | p95_maxdd | spread   | verdict  |
|-----------|-----------|------------|------------|-----------|-------------|-----------|----------|----------|
| BUYHOLD   | 0.124469 | 1.735275  | 3.870337  | 0.045000 | 0.775000    | 51.15%   | 3.745868 | (passive — no verdict) |

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
