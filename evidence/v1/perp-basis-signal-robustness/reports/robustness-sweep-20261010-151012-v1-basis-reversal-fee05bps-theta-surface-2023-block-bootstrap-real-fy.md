---
slug: perp-basis-signal-robustness
scenario: v1-basis-reversal-fee05bps-theta-surface-2023-block-bootstrap-real-fy
generated: 2026-10-10T15:10:12Z
wall_clock_s: 38.2
host: M022517718D
pid: 61191
git_commit: c407ca477bde1fcf4364332f885b75c9b5b3ef1e
data_revision_sha: 3a8b96c43f2d8980fd8039303197ff3ac5d01e8f9cebaecdf74c853622dbbfc7
---
# Basis-Reversal (taker_fee=5bps) θ-Surface — Parameter-Robustness Sweep — v1-basis-reversal-fee05bps-theta-surface-2023-block-bootstrap-real-fy

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
| taker_fee_bps            | 5                                                   |
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

Ruined paths: **0 of 1200** — no path's equity reached zero or below at any bar. Stated rather than left to inference: equity <= 0 is clamped to 1e-6 before the metrics, so `p95_maxdd` near 100% would otherwise be indistinguishable from ruin (bug-log #127).

Trades = total trade count across all N paths (turnover legibility — fee story for reversal arm, D-BR.2-LOCKED).

| g  | lookback | rebalance | k_long | drift | p5_sharpe | p50_sharpe | p95_sharpe | prob_loss | P(Sharpe>1) | p95_maxdd | spread   | trades     | verdict  | notes |
|----|----------|-----------|--------|-------|-----------|------------|------------|-----------|-------------|-----------|----------|------------|----------|-------|
|  0 |       60 |       480 |      3 | 0.10 | 0.057358 | 0.855675  | 1.677582  | 0.045000 | 0.375000    | 18.33%   | 1.620224 |     162512 | MARGINAL | → C5 DEFLATION REQUIRED |
|  1 |       24 |       480 |      3 | 0.10 | -0.089931 | 0.722292  | 1.578876  | 0.075000 | 0.305000    | 20.40%   | 1.668808 |     289698 | FRAGILE  |  |
|  2 |      168 |       480 |      3 | 0.10 | 0.101056 | 0.933763  | 1.739877  | 0.035000 | 0.455000    | 16.90%   | 1.638821 |      77267 | MARGINAL | → C5 DEFLATION REQUIRED |
|  3 |       60 |      1440 |      5 | 0.10 | 0.088618 | 1.087310  | 2.007633  | 0.040000 | 0.575000    | 27.96%   | 1.919016 |     139590 | MARGINAL | → C5 DEFLATION REQUIRED |
|  4 |       60 |       480 |      1 | 0.10 | -0.221765 | 0.490686  | 1.136206  | 0.160000 | 0.100000    | 9.16%   | 1.357970 |      62195 | FRAGILE  |  |
|  5 |       24 |       480 |      5 | 0.10 | -0.032053 | 0.901302  | 1.963988  | 0.065000 | 0.435000    | 31.17%   | 1.996041 |     342696 | FRAGILE  |  |

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
