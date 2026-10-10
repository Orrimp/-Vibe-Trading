---
slug: perp-basis-signal-robustness
scenario: v1-basis-reversal-fee00bps-theta-surface-2024-block-bootstrap-real-fy
generated: 2026-10-10T15:08:13Z
wall_clock_s: 40.0
host: M022517718D
pid: 57315
git_commit: e1097ebcfd9c5071454d5a1da5768aace74af5ad
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
| selected_block_length_L  | 282 (θ-independent — same L for all cells per OQ-3)      |
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

Ruined paths: **0 of 1200** — no path's equity reached zero or below at any bar. Stated rather than left to inference: equity <= 0 is clamped to 1e-6 before the metrics, so `p95_maxdd` near 100% would otherwise be indistinguishable from ruin (bug-log #127).

Trades = total trade count across all N paths (turnover legibility — fee story for reversal arm, D-BR.2-LOCKED).

| g  | lookback | rebalance | k_long | drift | p5_sharpe | p50_sharpe | p95_sharpe | prob_loss | P(Sharpe>1) | p95_maxdd | spread   | trades     | verdict  | notes |
|----|----------|-----------|--------|-------|-----------|------------|------------|-----------|-------------|-----------|----------|------------|----------|-------|
|  0 |       60 |       480 |      3 | 0.10 | -0.266710 | 0.542774  | 1.276752  | 0.130000 | 0.175000    | 28.73%   | 1.543463 |     186869 | FRAGILE  |  |
|  1 |       24 |       480 |      3 | 0.10 | -0.318469 | 0.504335  | 1.307111  | 0.125000 | 0.170000    | 28.45%   | 1.625579 |     316720 | FRAGILE  |  |
|  2 |      168 |       480 |      3 | 0.10 | -0.038311 | 0.709732  | 1.489756  | 0.060000 | 0.285000    | 25.77%   | 1.528067 |      89533 | FRAGILE  |  |
|  3 |       60 |      1440 |      5 | 0.10 | -0.245959 | 0.713146  | 1.719480  | 0.105000 | 0.330000    | 41.54%   | 1.965439 |     160037 | FRAGILE  |  |
|  4 |       60 |       480 |      1 | 0.10 | -0.257367 | 0.175904  | 0.592442  | 0.275000 | 0.000000    | 11.95%   | 0.849809 |      70684 | FRAGILE  |  |
|  5 |       24 |       480 |      5 | 0.10 | -0.352988 | 0.645705  | 1.719753  | 0.145000 | 0.290000    | 44.37%   | 2.072741 |     389897 | FRAGILE  |  |

## Buy-and-hold passive control (adversarial-review benchmark)

Equal-weight, hold from bar 0 over the SAME N paths and auto-L bootstrap.
Reference: adversarial review p50 Sharpe ≈ +1.78, P(loss) ≈ 4%, p95 MaxDD ≈ 51% at auto-L, N=500.

| row       | p5_sharpe | p50_sharpe | p95_sharpe | prob_loss | P(Sharpe>1) | p95_maxdd | spread   | verdict  |
|-----------|-----------|------------|------------|-----------|-------------|-----------|----------|----------|
| BUYHOLD   | -0.475960 | 0.945434  | 2.613640  | 0.150000 | 0.470000    | 65.86%   | 3.089601 | (passive — no verdict) |

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
