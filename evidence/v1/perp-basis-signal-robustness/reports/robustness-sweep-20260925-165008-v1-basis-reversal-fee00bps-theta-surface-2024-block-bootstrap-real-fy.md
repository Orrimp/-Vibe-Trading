---
slug: perp-basis-signal-robustness
scenario: v1-basis-reversal-fee00bps-theta-surface-2024-block-bootstrap-real-fy
generated: 2026-09-25T16:50:08Z
wall_clock_s: 38.7
host: M022517718D
pid: 88610
git_commit: 2d63ddef7964e6a4d5bce6082478ab06f6d04fd8
data_revision_sha: 3a8b96c43f2d8980fd8039303197ff3ac5d01e8f9cebaecdf74c853622dbbfc7
---
# Basis-Reversal (taker_fee=0bps) θ-Surface — Parameter-Robustness Sweep — v1-basis-reversal-fee00bps-theta-surface-2024-block-bootstrap-real-fy

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
|  0 |       60 |       480 |      3 | 0.10 | -0.297687 | 0.570164  | 1.323062  | 0.155000 | 0.210000    | 29.17%   | 1.620749 |     189387 | FRAGILE  |  |
|  1 |       24 |       480 |      3 | 0.10 | -0.369675 | 0.586223  | 1.299157  | 0.170000 | 0.220000    | 30.41%   | 1.668832 |     320021 | FRAGILE  |  |
|  2 |      168 |       480 |      3 | 0.10 | -0.194733 | 0.747556  | 1.511276  | 0.100000 | 0.305000    | 27.13%   | 1.706009 |      92471 | FRAGILE  |  |
|  3 |       60 |      1440 |      5 | 0.10 | -0.311707 | 0.862531  | 1.741136  | 0.140000 | 0.400000    | 42.32%   | 2.052843 |     161718 | FRAGILE  |  |
|  4 |       60 |       480 |      1 | 0.10 | -0.279612 | 0.183379  | 0.648949  | 0.255000 | 0.000000    | 12.58%   | 0.928562 |      72733 | FRAGILE  |  |
|  5 |       24 |       480 |      5 | 0.10 | -0.467390 | 0.740770  | 1.688873  | 0.170000 | 0.360000    | 46.21%   | 2.156263 |     392150 | FRAGILE  |  |

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
Conclusion: v1 cross-sectional basis-reversal at 0 bps taker fee is structurally fragile
across the tested parameter space on this 10-symbol universe. The fee-bleed from
reversal-arm turnover consumes the gross −0.10 IC edge at this fee level.
VERDICT: FRAGILE-on-fees at this fee level. Pre-registered result — see R-BR.LOAD.

Notes:
- Decision-rule bands: frozen robustness-decision-rule-2026-05-30.md § 0.
- Composite = weakest-link of 5 PRIMARY signals (p5_sharpe, p50_sharpe, prob_loss, P(Sharpe>1), p95_maxdd).
- Spread and p50-vs-real-path are INTERPRETIVE (not verdict-forcing).
- Generator: `block-bootstrap-real` only is anchor-grade.
- Determinism scope: Apple-Silicon canonical box (ADR-0051 D5).
