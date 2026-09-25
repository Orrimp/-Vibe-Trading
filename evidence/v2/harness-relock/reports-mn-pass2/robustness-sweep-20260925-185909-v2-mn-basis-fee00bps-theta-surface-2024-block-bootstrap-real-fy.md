---
slug: perp-basis-mn-spread
scenario: v2-mn-basis-fee00bps-theta-surface-2024-block-bootstrap-real-fy
generated: 2026-09-25T18:59:09Z
wall_clock_s: 17.4
host: M022517718D
pid: 6812
git_commit: a172f178e6b9462808638574775f966ab7a87cf3
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
Trades = total trade count across all N paths. NOTE: this count INCLUDES synthetic
liquidation covers, so MN turnover is not directly comparable with the long-only families
(bug-log #110). Read it next to the liquidations column, not on its own.
Funding = total realized funding cashflow across all N paths, in quote currency. The MN short
leg shorts the HIGHEST-funding names, so it is an INCOME leg: a positive figure means the book
received funding. Together with the fee ladder this makes R-MN.3's net-of-cost read derivable
from this report rather than from outside it.

| g  | lookback | rebalance | k_long | k_short | drift | p5_sharpe | p50_sharpe | p95_sharpe | prob_loss | P(Sharpe>1) | p95_maxdd | spread   | liquidations | trades     | funding        | verdict  | notes |
|----|----------|-----------|--------|---------|-------|-----------|------------|------------|-----------|-------------|-----------|----------|--------------|------------|----------------|----------|-------|
|  0 |       60 |       480 |      3 |       3 | 0.10 | -0.301017 | 0.060300  | 0.436827  | 0.405000 | 0.000000    | 24.23%   | 0.737844 |            0 |     253769 |      209043.72 | FRAGILE  |  |
|  1 |      168 |       480 |      3 |       3 | 0.10 | -0.162669 | 0.203762  | 0.588060  | 0.165000 | 0.005000    | 20.40%   | 0.750729 |            0 |     155240 |      190887.52 | FRAGILE  |  |

## Dollar-neutral null (the § 0 null for a beta-neutral book)

ADR-0051 § D-MN: a beta-neutral book's null is CASH, not buy-and-hold. This row is the
bar the spread must clear on the frozen § 0 weakest-link bands. It is analytic, not
resampled: a dollar-neutral cash-equivalent returns 0 on every path, so every statistic
below is 0 by construction. It carries NO verdict.

| row       | p5_sharpe | p50_sharpe | p95_sharpe | prob_loss | P(Sharpe>1) | p95_maxdd | spread   | verdict  |
|-----------|-----------|------------|------------|-----------|-------------|-----------|----------|----------|
| NULL-0    | 0.000000 | 0.000000  | 0.000000  | 0.000000 | 0.000000    | 0.00%   | 0.000000 | (null — no verdict) |

## Buy-and-hold passive control (adversarial-review benchmark)

RETAINED AS A REFERENCE, NOT AS THIS ARM'S NULL. Per ADR-0051 § D-MN, buy-and-hold is the
WRONG null for a beta-stripped book — the § 0 null above is. It is kept because deleting a
measured control loses information, and it is labelled so it cannot be read as the bar.

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
construction removes directional beta. This report does NOT identify what remains
as the binding cost, and it no longer claims fee-bleed: the 2026-09-25 re-lock
measured this family at BOTH fee levels and the arm is uniformly FRAGILE at 0 bps
taker fee — with no fee bleed at all — while 0 -> 5 bps costs only ~0.05-0.16
Sharpe against a gap to the FRAGILE band of ~0.3. The fee is not the killer.
This surface IS the 0 bps read: fee-bleed is excluded by construction here.
VERDICT: FRAGILE. Pre-registered result — see R-MN.LOAD (§ D6.10).
Supersession (bug-log #110): the earlier fee-bleed reading is WITHDRAWN, not softened.

Notes:
- Decision-rule bands: frozen robustness-decision-rule-2026-05-30.md § 0.
- Composite = weakest-link of 5 PRIMARY signals (p5_sharpe, p50_sharpe, prob_loss, P(Sharpe>1), p95_maxdd).
- Spread and p50-vs-real-path are INTERPRETIVE (not verdict-forcing).
- Generator: `block-bootstrap-real` only is anchor-grade.
- Determinism scope: Apple-Silicon canonical box (ADR-0051 D5).
