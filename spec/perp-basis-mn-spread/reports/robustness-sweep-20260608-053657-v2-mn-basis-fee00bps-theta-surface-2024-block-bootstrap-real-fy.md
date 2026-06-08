---
slug: perp-basis-mn-spread
scenario: v2-mn-basis-fee00bps-theta-surface-2024-block-bootstrap-real-fy
generated: 2026-06-08T05:36:57Z
wall_clock_s: 204.4
host: M022517718D
pid: 52325
git_commit: 18334c9a31d86d956c103d7392ecc5f10a15ba0c
data_revision_sha: 3a8b96c43f2d8980fd8039303197ff3ac5d01e8f9cebaecdf74c853622dbbfc7
---
# MN Basis-Spread (long-short, taker_fee=0bps) θ-Surface — Parameter-Robustness Sweep — v2-mn-basis-fee00bps-theta-surface-2024-block-bootstrap-real-fy

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
| selected_block_length_L  | 200 (θ-independent — same L for all cells per OQ-3)      |
| source_revision_sha      | 3a8b96c43f2d8980fd8039303197ff3ac5d01e8f9cebaecdf74c853622dbbfc7                                    |
| taker_fee_bps            | 0                                                   |
| slippage_bps             | 2                                                    |
| held_constant            | score_source=basis_reversal selection_mode=long_short k_long=k_short=3 exposure_cap=0.50 vol_floor=inert max_leverage=1 maintenance_margin_frac=0.5 |
| data_revisions           | basis:aa72409aa0f856960385a823bc61be1b8274e84f658439b65e5d1b1b1a48f1cd funding:bf1ede44e57d797b57e5a4f2743f58027e4eba12d91e1ffaf883dcdd49365668 |

## MN-Spread θ-grid definition (2-cell, LOCKED § D-MN.8-LOCKED — changing this changes the SHA)

grid_definition:
  g=0 lookback_bars=60 rebalance_minutes=480 k_long=3 k_short=3 drift=0.10 max_leverage=1 maintenance_margin_frac=0.5
  g=1 lookback_bars=168 rebalance_minutes=480 k_long=3 k_short=3 drift=0.10 max_leverage=1 maintenance_margin_frac=0.5

## θ-surface (per-cell distribution + verdict)

Notation: p5/p50/p95 Sharpe at {:.6}; prob_loss / P(Sharpe>1) at {:.6}; p95_maxdd at {:.2}%.
Spread = p95_sharpe − p5_sharpe (interpretive, NOT verdict-forcing).
Verdict: FRAGILE/MARGINAL/ROBUST via 5-signal weakest-link (frozen decision-rule § 0 bands).

Liquidations = total maintenance-margin liquidation events across all N paths (MN only, D-MN.8).

| g  | lookback | rebalance | k_long | k_short | drift | p5_sharpe | p50_sharpe | p95_sharpe | prob_loss | P(Sharpe>1) | p95_maxdd | spread   | liquidations | verdict  | notes |
|----|----------|-----------|--------|---------|-------|-----------|------------|------------|-----------|-------------|-----------|----------|--------------|----------|-------|
|  0 |       60 |       480 |      3 |       3 | 0.10 | -0.016089 | 0.033331  | 0.077235  | 0.110000 | 0.000000    | 85.51%   | 0.093324 |            9 | FRAGILE  |  |
|  1 |      168 |       480 |      3 |       3 | 0.10 | -0.039811 | 0.041106  | 0.118217  | 0.150000 | 0.000000    | 86.59%   | 0.158028 |           13 | FRAGILE  |  |

## Buy-and-hold passive control (adversarial-review benchmark)

Equal-weight, hold from bar 0 over the SAME N paths and auto-L bootstrap.
Reference: adversarial review p50 Sharpe ≈ +1.78, P(loss) ≈ 4%, p95 MaxDD ≈ 51% at auto-L, N=500.

| row       | p5_sharpe | p50_sharpe | p95_sharpe | prob_loss | P(Sharpe>1) | p95_maxdd | spread   | verdict  |
|-----------|-----------|------------|------------|-----------|-------------|-----------|----------|----------|
| BUYHOLD   | -0.682135 | 1.104731  | 2.690469  | 0.165000 | 0.535000    | 64.83%   | 3.372604 | (passive — no verdict) |

## Family verdict

FAMILY-UNIFORM-FRAGILE

Every active θ-cell is FRAGILE under the frozen decision-rule bands.
No multiple-testing correction is needed for a uniform-negative result:
C3 is not selecting a winner — it is reporting that no cell cleared the bar.
Conclusion: v2 market-neutral basis spread at 0 bps taker fee is structurally fragile
across the tested parameter space on this 10-symbol universe. The dollar-neutral
construction removes directional beta but not fee-bleed from short-leg turnover.
VERDICT: FRAGILE. Pre-registered result — see R-MN.LOAD (§ D6.10).

Notes:
- Decision-rule bands: frozen robustness-decision-rule-2026-05-30.md § 0.
- Composite = weakest-link of 5 PRIMARY signals (p5_sharpe, p50_sharpe, prob_loss, P(Sharpe>1), p95_maxdd).
- Spread and p50-vs-real-path are INTERPRETIVE (not verdict-forcing).
- Generator: `block-bootstrap-real` only is anchor-grade.
- Determinism scope: Apple-Silicon canonical box (ADR-0051 D5).
