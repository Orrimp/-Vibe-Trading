---
slug: carry-strategy
mode: release
status: draft
audience: human-operator
updated: 2026-06-02
generated: 2026-06-02T08:35:00Z
---

# Cross-sectional Funding Carry (v0.1.0) — release

## TL;DR

The funding-carry strategy was built and tested (VERDICT → PASS, 250 tests green, 89/89 anchors); it came back **FAMILY-UNIFORM-FRAGILE on BOTH 2023 and 2024** — so carry is **retired**, and with it **all three** cross-sectional families we set out to test (momentum, mean-reversion, carry) are now confirmed dominated by passive equal-weight buy-and-hold on this 10-coin universe.

## What changed

- **A complete new strategy was built, staged and additively** — funding-data loader → the shared-index bootstrap co-resample (the methodological crux) → a new `ScoreSource::FundingCarry` signal carrying the load-bearing harvest sign → per-bar funding-cashflow accrual on the equity curve → sweep-bin wiring with a realized-funding column → 6 falsifier tests → the anchored 6×200 θ-surface on two years.
- **It changed nothing that already shipped.** Every new seam defaults off, so the 87 pre-existing regression anchors stayed byte-identical the whole way through. The build added exactly **2** new anchors (#88 carry-2023, #89 carry-2024) → **89/89 PASS**.
- **The strategy works and harvests real funding** — the realized-funding column is non-zero on every cell — but harvesting funding earned **no robust risk-adjusted edge**: every cell of every grid, on both years, scored FRAGILE under the frozen decision rule and fell well below the buy-and-hold bar.

## Why

Carry was the **pre-registered rotation target**, not a fresh idea. We had already run the two price-based cross-sectional families through the robustness harness and retired both: momentum and mean-reversion each came back FAMILY-UNIFORM-FRAGILE, killed by turnover / fee-bleed — on the same resampled 2023 histories, simply holding the 10 coins equal-weight earned **+1.74 Sharpe** while the active price strategies churned that drift into a break-even-at-best loss machine. Carry was named the runner-up because it is the most *structurally different* bet on the table: it earns the **funding payment** (a cash settlement every 8h), not a price move, and it rebalances on the slow 8h funding cadence — so on paper it has the best shot at dodging the fee-bleed that killed the price families. Its data was already banked. This v0.1.0 ships the cheapest honest test of that thesis: framing (a), a long-only directional carry-tilt (long the most-negative-funding names, on the existing solvency-guarded engine), measured apples-to-apples against the +1.74 buy-and-hold bar. Source: [`spec/carry-strategy/feature.md`](../feature.md).

## What you can do now

| Action | Command |
|--------|---------|
| Re-run the anchored carry surface (2023, N=200, ~31s) | `cargo run -p backtest --features "candle realdata" --bin param_robustness_sweep -- --score-source carry --grid carry-tier1 --generator block-bootstrap-real --year 2023 --paths 200 --ensemble-seed 0xC0FFEE --out-dir /tmp/carry-2023` |
| Re-run on the harder 2024 regime (N=200, ~28s) | `cargo run -p backtest --features "candle realdata" --bin param_robustness_sweep -- --score-source carry --grid carry-tier1 --generator block-bootstrap-real --year 2024 --paths 200 --ensemble-seed 0xC0FFEE --out-dir /tmp/carry-2024` |
| Verify the regression gate (89/89, ~seconds) | `bash scripts/verify_anchors.sh` |
| Re-run the 6 carry falsifiers | `cargo test -p backtest --features "candle realdata" --test carry_divergence_e2e` |

_Note: carry is **not** promoted to live trading. There is nothing to switch on; the deliverable is the decision-grade verdict below._

## Live demo

A fresh N=3 smoke run (the cheap representative run — the anchored gating numbers are N=200, in the committed surfaces below). This proves the carry path is wired end-to-end and the realized-funding column renders live:

```
$ cargo run -p backtest --features "candle realdata" --bin param_robustness_sweep -- \
    --score-source carry --grid carry-tier1 --generator block-bootstrap-real \
    --year 2023 --paths 3 --ensemble-seed 0xC0FFEE --out-dir /tmp/carry-demo

param_robustness_sweep DONE
  report:         /tmp/carry-demo/robustness-sweep-...-v1-carry-theta-surface-2023-block-bootstrap-real-fy.md
  body_sha:       bd8cdaa853fbf6366fe8d29000b46be9d8a62ca52e22550a937b34f70bdda5cc
  wall_clock_s:   18.3
  n_cells:        6
  n_paths:        3
  family_verdict: FAMILY-UNIFORM-FRAGILE
  buyhold p50 Sharpe: 1.2351  P(loss): 0.0000  p95 MaxDD: 38.33%

  per-cell summary:
    g= 0 lookback=   9 k_long=3 → FRAGILE | p50=0.0246 p5=0.0150 MaxDD_p95=75.0%
    g= 1 lookback=   3 k_long=3 → FRAGILE | p50=0.0181 p5=0.0133 MaxDD_p95=75.5%
    g= 2 lookback=  21 k_long=3 → FRAGILE | p50=0.0212 p5=0.0093 MaxDD_p95=72.9%
    g= 3 lookback=   9 k_long=5 → FRAGILE | p50=0.0273 p5=0.0183 MaxDD_p95=84.8%
    g= 4 lookback=   9 k_long=1 → FRAGILE | p50=0.0288 p5=0.0189 MaxDD_p95=68.6%
    g= 5 lookback=   3 k_long=5 → FRAGILE | p50=-0.0123 p5=-0.0296 MaxDD_p95=85.0%
```

Notice: even at N=3 the family verdict is already FAMILY-UNIFORM-FRAGILE, and the buy-and-hold control (p50 +1.24 on these 3 paths) sits an order of magnitude above the best carry cell (p50 +0.029). The N=3 smoke is throwaway — read the committed N=200 surfaces below for the gating numbers. Full smoke output saved at [`artifacts/carry-strategy-2026-06-02/carry-smoke-n3-2023-live.md`](artifacts/carry-strategy-2026-06-02/carry-smoke-n3-2023-live.md).

## Screenshots

_n/a — non-UI feature. The carry strategy is a backtest/research surface; the deliverable is the anchored markdown θ-surface reports, not a cockpit screen._

## The verdict evidence (real numbers from the committed surfaces)

### 2023-FY θ-surface — anchor #88 ([source](../reports/robustness-sweep-20260602-075354-v1-carry-theta-surface-2023-block-bootstrap-real-fy.md))

200 paths/cell, shared-index block-bootstrap of real 2023 Binance OHLCV + co-resampled funding, 6 bps fees. p5/p50/p95 = 5th/50th/95th percentile Sharpe across the 200 paths. `funding_harvested` = total realized funding cashflow summed across all 200 paths (Decimal).

| g | L (settle) | rebal | K | p5 Sharpe | p50 Sharpe | p95 Sharpe | P(loss) | P(Sharpe>1) | p95 MaxDD | funding_harvested | verdict |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 0 | 9 | 480m | 3 | −0.1003 | +0.0252 | +0.0512 | 31.5% | 0.0% | 90.77% | +3,098,097 | FRAGILE |
| 1 | 3 | 480m | 3 | −0.0933 | +0.0154 | +0.0349 | 38.0% | 0.0% | 92.19% | +1,572,693 | FRAGILE |
| 2 | 21 | 480m | 3 | −0.1323 | +0.0278 | +0.0528 | 22.5% | 0.0% | 90.80% | +1,997,086 | FRAGILE |
| 3 | 9 | 1440m | 5 | −0.1009 | +0.0222 | +0.0671 | 25.5% | 0.0% | 89.89% | −1,321,996 | FRAGILE |
| 4 | 9 | 480m | 1 | −0.1921 | **+0.0386** | +0.0774 | 14.0% | 0.0% | 87.43% | +4,081,759 | FRAGILE |
| 5 | 3 | 480m | 5 | −0.0717 | +0.0170 | +0.0473 | 23.5% | 0.0% | 91.15% | −1,626,590 | FRAGILE |

**Buy-and-hold control (same 200 paths):** p5 +0.124, **p50 +1.735**, p95 +3.870, P(loss) 4.5%, P(Sharpe>1) 77.5%, p95 MaxDD 51.15%.

Best carry cell is g=4 (K=1, narrowest selection) at **p50 +0.039** Sharpe — roughly **1.70 Sharpe units below** the +1.74 buy-and-hold bar. Every cell has p5 Sharpe < 0 (the FRAGILE trigger) and **P(Sharpe>1) = 0.0%** across all six.

### 2024-FY θ-surface — anchor #89 ([source](../reports/robustness-sweep-20260602-075424-v1-carry-theta-surface-2024-block-bootstrap-real-fy.md))

Same locked grid, harder tail-negative regime.

| g | L (settle) | rebal | K | p5 Sharpe | p50 Sharpe | p95 Sharpe | P(loss) | P(Sharpe>1) | p95 MaxDD | verdict |
|---|---|---|---|---|---|---|---|---|---|---|
| 0 | 9 | 480m | 3 | −0.0186 | +0.0046 | +0.0383 | 34.5% | 0.0% | 80.66% | FRAGILE |
| 1 | 3 | 480m | 3 | −0.0336 | −0.0014 | +0.0268 | 54.5% | 0.0% | 82.04% | FRAGILE |
| 2 | 21 | 480m | 3 | −0.0262 | −0.0001 | +0.0350 | 50.0% | 0.0% | 77.05% | FRAGILE |
| 3 | 9 | 1440m | 5 | −0.0661 | −0.0094 | +0.0434 | 61.0% | 0.0% | 83.86% | FRAGILE |
| 4 | 9 | 480m | 1 | +0.0163 | **+0.0427** | +0.0942 | 2.5% | 0.0% | 63.60% | FRAGILE |
| 5 | 3 | 480m | 5 | −0.0569 | −0.0169 | +0.0283 | 72.5% | 0.0% | 87.83% | FRAGILE |

**Buy-and-hold control (same 200 paths):** p5 −0.682, **p50 +1.105**, p95 +2.690, P(loss) 16.5%, P(Sharpe>1) 53.5%, p95 MaxDD 64.83%.

Best carry cell is again g=4 at **p50 +0.043** — about **1.06 Sharpe units below** the +1.10 buy-and-hold bar. g=4 is the only cell with p5 > 0, but its p95 MaxDD (63.6%) is well above the ~50% ROBUST threshold and its P(Sharpe>1) is 0.0% → still FRAGILE on the weakest-link rule.

### The funding IS real (carry is not a no-op)

The `funding_harvested` column is non-zero on every cell (e.g. +3.1M on 2023 g=0, +4.1M on 2023 g=4) — the cashflow accrual is genuinely moving the equity curve, guarded by a dedicated non-no-op falsifier. **The honest diagnosis:** framing (a) holds *directional long perp exposure* on the negative-funding names, so the P&L is dominated by price risk, not the funding premium. The funding is harvested; the directional price exposure simply overwhelms it. Mixed-sign harvested values (some cells negative) reflect that the long-only framing pays funding when a held name turns positive-funding over a path.

## The headline: all three cross-sectional families are now retired

The harness has now run the three most-cited crypto cross-sectional strategy classes and retired each on the robustness axis:

| Family | Year | Best-cell p50 Sharpe | Buy-and-hold p50 | Gap vs bar | Killer | Verdict |
|--------|------|---------------------|------------------|-----------|--------|---------|
| Momentum (top-K winners) | 2023 | +0.014 | +1.74 | −1.73 | turnover / fee-bleed | FAMILY-UNIFORM-FRAGILE |
| Mean-reversion (bottom-K losers) | 2023 | +0.007 | +1.74 | −1.73 | turnover / fee-bleed | FAMILY-UNIFORM-FRAGILE |
| **Carry** (long most-negative funding) | **2023** | **+0.039** | **+1.74** | **−1.70** | directional price risk swamps funding | **FAMILY-UNIFORM-FRAGILE** |
| **Carry** | **2024** | **+0.043** | **+1.10** | **−1.06** | same, harder regime | **FAMILY-UNIFORM-FRAGILE** |

**Conclusion: active cross-sectional trading — whether on price (momentum, mean-reversion) or on funding (carry) — is dominated by passive equal-weight buy-and-hold on this 10-USDT-pair Binance universe.** Carry was the structurally-best a-priori shot (non-trend, independent return source, naturally low-turnover) and it still came back fragile on both a bull (2023) and a harder tail-negative (2024) regime. This is a **methodology win**: the robustness harness has cheaply ruled out the three families at a fraction of a live-trading cost, completing the go/no-go without burning capital.

## Verification

V-items are the carry requirements (R-CARRY.*) from the feature spec; the spec's `## Verification` section links to the tester report. Evidence cites the tester report [`test-2026-06-02-carry-strategy.md`](../reports/test-2026-06-02-carry-strategy.md).

| V-id | Description | Status | Evidence |
|------|-------------|--------|----------|
| R-CARRY.1 | Carry score = trailing-mean funding rank over L settlements | VERIFIED | `strategy` lib 136/136 pass; carry_score unit + ranking tests green (tester §3) |
| R-CARRY.2 | Load-bearing harvest SIGN (long the paid side of negative funding) | VERIFIED | Falsifier RED-on-revert: flipping `carry_score` sign FAILED `r_carry2_sign_assertion_*`; restored green (tester §5) |
| R-CARRY.3 | Default 8h funding-cadence rebalance + grid spans turnover axis | VERIFIED | Grid g=0..5 spans L∈{3,9,21}, rebal∈{480,1440}m, K∈{1,3,5} — both surfaces §"grid_definition" |
| R-CARRY.4 | Reuse `top_k_long` + long-only sizing; new `ScoreSource::FundingCarry` | VERIFIED | Carry runs the identical long-only engine as #86/#87; surfaces render under same harness |
| R-CARRY.5 | Funding parquet loader + REVISION pin `bf1ede44…` | VERIFIED | `funding_data` 10/10 unit tests pass; both surfaces carry funding revision `bf1ede44…` |
| R-CARRY.6 | 8h→1h as-of forward-fill, no look-ahead | VERIFIED | Falsifier RED-on-revert: `t <= bar_ts` → `t < bar_ts` FAILED `no_look_ahead_falsifier`; restored (tester §5) |
| R-CARRY.7 | Funding co-resampled THROUGH the bootstrap via the SAME shared index (the crux) | VERIFIED | `data synth::bootstrap` 15/15 pass incl. `funding_index_aligned_co_movement` (0 misaligned bars) (tester §3) |
| R-CARRY.8 | Funding-cashflow accrual moves equity (non-no-op) | VERIFIED | Falsifier RED-on-revert: zeroing `cash += cashflow` FAILED `r_carry10b_funding_cashflow_non_no_op`; restored (tester §5) |
| R-CARRY.9 | Day-1 BOTH-axes gate (C2 path + C3 parameter + buy-and-hold control) | VERIFIED | Both surfaces deliver per-cell distributions + family verdict + BH control row |
| R-CARRY.10 | Carry-vs-price divergence + funding non-no-op falsifiers | VERIFIED | `carry_divergence_e2e` 6/6 pass; both 10a + 10b RED-on-revert confirmed (tester §5) |
| R-CARRY.11 | Determinism + additive anchoring (87 hold, +2 new) | VERIFIED | `verify_anchors.sh` → 89/89 PASS; 87 pre-existing byte-identical; two-run byte-identity green |
| R-CARRY.12 | In-sample 2023-FY + 2024-FY (upgraded to day-1 gating per E1) | VERIFIED | Both anchored surfaces #88 (2023) + #89 (2024) committed |

## Numbers that matter

- **Tests:** 250 passed, 0 failed, 0 ignored (strategy 136, data/bootstrap 15, backtest lib 76, carry_divergence_e2e 6, param_sweep_e2e 8, montecarlo_e2e 9). Source: tester §3.
- **Anchors:** **89/89 PASS** — verified live by this presenter (`bash scripts/verify_anchors.sh` → `ANCHORS PASS (89 / 89)`). 87 pre-existing byte-identical + #88 carry-2023 (`f03cd714…`) + #89 carry-2024 (`fd96d5a8…`).
- **Falsifiers:** 4 mandatory falsifiers each independently RED-on-revert (sign, no-look-ahead, carry-vs-price divergence, cashflow non-no-op) + two-run byte-identity green.
- **Best carry cell vs bar:** 2023 → p50 +0.039 vs +1.74 (−1.70); 2024 → p50 +0.043 vs +1.10 (−1.06). P(Sharpe>1) = **0.0%** on all 12 cells across both years.
- **Wall-clock:** anchored surfaces 30.7s (2023) / 28.4s (2024) for 6×200 = 1,200 paths each — comparable to MR's 6×200.
- **spec-lint:** FAIL (94 violations in 2 categories: 87 dead-link + 7 trace-broken-path) — verified live by this presenter. This is **one fewer** than the audit-2026-06-01 baseline (95 in 3 categories) and introduces **no new categories or higher counts**; all violations are pre-existing spec debt, not a carry regression.

## Open decisions

1. **Approve the carry finding + retirement.** Confirm carry v0.1.0 is retired (dominated by buy-and-hold) and the three-family program is closed. (The build is sound and anchored regardless of this decision; the decision is whether you accept the science verdict and close the family.)

The strategic "where next" question (below) is **framing for your awareness, not a decision you must make in this deck** — it follows from approving the finding, and the orchestrator can route the chosen direction to the analyst as a fresh feature.

### Where next (neutral framing — surfaced by the tester/analyst, NOT decided here)

Now that all three cross-sectional families are retired, the rotation options are:

- **(a) Value factor** — a genuinely different signal class, but **data-gated**: it needs a new fundamental/on-chain dataset that is not yet banked.
- **(b) Regime / blended approach** — switch or blend strategies by detected market regime rather than running one family always-on.
- **(c) Market-neutral carry v0.2.0 (framing (b))** — long/short dollar-neutral funding harvest that isolates funding from price. This would need the short-side engine. **Caveat:** the v0.1.0 result shows the funding signal lacks directional edge on this universe, so building the short-side engine first is durable infrastructure on an unvalidated premise — likely **defer**.
- **(d) Accept the finding and pivot the product thesis** — treat "passive equal-weight beats active cross-sectional on this universe" as the load-bearing result and re-aim the product accordingly.

## Approval

- [ ] Approved — ship
- [ ] Approve with notes (notes below)
- [ ] Reject — _add reason below_

### Notes / feedback

_empty until operator fills_

## Changelog

- 2026-06-02 (presenter): initial release deck — carry v0.1.0 FAMILY-UNIFORM-FRAGILE on 2023 + 2024, three-family program closed; 250 tests, 89/89 anchors verified live, spec-lint 94/2 (no regression).
