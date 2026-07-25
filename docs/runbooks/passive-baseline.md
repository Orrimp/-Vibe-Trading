# Passive-Baseline Runbook — buy-and-hold as the production baseline

**Version:** v0
**Owner:** operator / analyst
**Status:** canonical baseline (active-edge search CONCLUDED 2026-06-08)
**Related code:** the buy-and-hold (BH) control in `crates/backtest` (the
benchmark every robustness surface is scored against)
**Decision of record:** [`docs/archive/pre-bmad-spec/product.md` § Strategy library — Active-edge-search
status](../../docs/archive/pre-bmad-spec/product.md) (terminal verdict: ship passive)
**Evidence:** [`onchain-netflow-spike-2026-06-08.md`](../dev-notes/onchain-netflow-spike-2026-06-08.md),
[`onchain-vs-conclude-fork-2026-06-08.md`](../dev-notes/onchain-vs-conclude-fork-2026-06-08.md)

---

## Overview

The active-vs-passive search concluded on 2026-06-08: across the three reachable
information channels — **price/OHLCV, derivatives-positioning, and on-chain** — no
active strategy beat passive buy-and-hold net of cost under the frozen
block-bootstrap Monte-Carlo § 0 rule, on the 2023-24 large-cap perp sample.
**Passive buy-and-hold is the recommended and shipped baseline strategy** for this
project.

This is a **promotion of already-built, already-anchored code, not a new build.**
The BH control has existed throughout the robustness program as the benchmark
every strategy surface was scored against — it is the most-tested path in the
repo. "Ship passive" promotes it from *benchmark* to *the strategy the
paper-trading agent runs by default*.

> **Scope honesty (read before quoting this anywhere).** "Passive wins" means
> "active ≤ passive in the *reachable* universe (price + positioning + on-chain),
> net of cost, on the 2023-24 large-cap perp sample." It does **NOT** mean "active
> trading is impossible." Untested channels (options/implied-vol, macro, social)
> remain by lower prior or infeasibility, and the passive benchmark's high Sharpe
> (+1.74 in 2023) is partly a structural bull-leg artifact of the sample, not a
> guarantee across regimes. See [`product.md`](../../docs/archive/pre-bmad-spec/product.md) for the full bounded
> statement.

---

## What the baseline IS

| Property            | Value                                                                 |
|---------------------|-----------------------------------------------------------------------|
| Strategy            | Buy-and-hold (BH) on the configured universe                          |
| Universe            | The config-driven universe (`config/agent.toml` → `data.sources.*`); the program's sample was the 10 large-cap USDT perps/spot. |
| Position rule       | Hold the universe; no signal-driven entry/exit. (Equal-weight is the proposed default; cap-weight is an operator option — see Rebalance cadence below.) |
| Rebalance cadence   | Periodic rebalance to target weights — **monthly is the proposed default** (see decision below). |
| Mode                | Paper (simulated fills) — this project is paper-terminal per `product.md` § Project scope boundary. No real-money execution. |
| Benchmark behaviour | Pinned by the BH anchor scenarios in `evidence/anchors.toml` (the BH control the sweep scored against). |

---

## Rebalance cadence — operator decision

A pure hold needs a documented rebalance cadence so weights do not drift
unboundedly and so the operational behaviour is testable.

- **Proposed default: monthly rebalance to equal weights.** Rationale: monthly
  keeps turnover (and therefore fee drag) low — consistent with the program
  finding that even low-cost active arms did not clear the BH bar — while
  bounding weight drift. Equal-weight avoids importing a market-cap data
  dependency.
- **Operator options:** (a) cadence — monthly (default) / quarterly (lower
  turnover) / never (true buy-once-hold, max drift); (b) weighting — equal
  (default) / cap-weighted (tracks the market more closely, needs a cap feed).

> **This is the one open decision in shipping passive.** It does not block the
> designation (BH is the baseline regardless of cadence); it only sets the
> rebalance schedule. Confirm `(cadence, weighting)` and record the choice in
> this runbook's changelog.

---

## How to run it (paper mode)

The BH control is exercised today as the sweep benchmark. Running it *as the
baseline strategy* in paper mode uses the existing harness:

1. Set `mode = "paper"` in `config/agent.toml`.
2. Configure the universe under `data.sources.*` (the program's sample was the
   top-10 USDT large-caps).
3. Run the agent in paper mode; the baseline holds the universe and rebalances on
   the configured cadence.
4. Operator success reports (`docs/archive/pre-bmad-spec/v1/operator-success-reports/`) already headline
   "cumulative return vs BH baseline" — with passive *as* the baseline, that
   headline reads as tracking error ≈ 0 by construction, and the report's value
   becomes drawdown / Sharpe / system-health visibility.

> **Long-running task note.** If a paper run or a baseline backtest is kicked off
> for >2 min, emit a copy-pasteable watch block, e.g.:
> ```
> watch -n 30 'tail -n 20 <run-log>'
> ```

---

## What "ship passive" does NOT require

To keep the wind-down honest and minimal, shipping passive is **documentation +
a designation**, not engineering:

- **No new strategy crate.** BH already exists.
- **No new `ScoreSource`, no new sweep arm, no new backtest binary.**
- **No new regression anchor.** The BH control is already anchored.
- **No further domain hunt.** The pre-committed hard-stop binds: no options hunt,
  no macro hunt, no on-chain sub-signal mining under this program. The operator
  may open any of those later as a **fresh** program — it is not a continuation of
  this hunt.

Anything beyond this runbook + the `product.md` designation would be inventing
scope the terminal verdict does not call for.

---

## Real-data validation (2026-06-14)

The shipped Lab tooling backtested the simple strategies (sma/macd/rsi/bbands)
against buy-and-hold on the **real** Binance hourly corpus (10 symbols × 2023–24,
net of cost). **Passive dominates in the 18/20 symbol-years where B&H was
positive** (often by an order of magnitude) — confirming ship-passive as the base.
Nuance: in the 2 down-market cases (AVAX 2024 −8.2%, DOT 2024 −19.6%) the
**trend-followers (SMA/MACD) protected capital** — a defensible down-market hedge,
not a reason to go active. Mean-reverters (RSI/BBands) had no edge anywhere. Full
table + re-runnable harness:
[`docs/dev-notes/realdata-simple-strategy-survey-2026-06-13.md`](../dev-notes/realdata-simple-strategy-survey-2026-06-13.md).

> **REVISION (2026-06-15) — the down-market hedge nuance did NOT survive
> path-robustness testing.** A block-bootstrap Monte-Carlo (N=500 stationary-resampled
> paths per cell, scored against the frozen § 0 decision rule) tested whether the 2-case
> AVAX-2024 / DOT-2024 trend-following protection is a real strategy property or an
> artifact of the one 2024 bar ordering. **All 9 cells scored FRAGILE** (every cell's p5
> Sharpe < 0). SMA/MACD have a positive *median*-path Sharpe on the down-market cells
> (AVAX·2024 SMA p50 +0.570, DOT·2024 SMA p50 +0.653) but their p5 left tails dip
> negative (-0.810, -0.910) — roughly 1 in 4 resampled orderings ends below the starting
> equity (SMA prob_loss 0.248 on both). **The down-market hedge is path-fragile**:
> sensitive to the specific 2024 ordering, NOT a robust property. **Net effect on this
> runbook: the ship-passive base recommendation is UNQUALIFIED on this evidence** — the
> "but trend-following is a defensible down-market hedge" qualifier above does not hold
> up to path-resampling and should not be quoted as a standalone reason to go active. The
> claim is capped at the per-symbol-year level (AVAX-2024 / DOT-2024 individually; 2
> symbol-years, hourly, default params — not down-markets in general). Confirmed numbers
> + scope cap:
> [`docs/dev-notes/analysis-2026-06-15-simple-strategy-overfit-guard.md`](../dev-notes/analysis-2026-06-15-simple-strategy-overfit-guard.md).

> **BEAR-SURVEY (2026-06-15) — a wider, deeper 2021-22 bear stress-test FIRMS
> ship-passive.** The overfit-guard revision above rested on **2 idiosyncratic
> alt-coin dips inside an otherwise-bull 2023-24**. To test the ship-passive verdict
> against a real market-wide bear, the four simple strategies were re-run over
> `data/binance-2122/` (10 symbols × {2021, 2022} hourly — the whole universe down
> in 2022: BTC ≈ −64%, SOL ≈ −94%, AVAX ≈ −90%) in a two-stage harness: Stage 1
> point-surveyed all 80 cells and found **40 apparent winners** (cells beating B&H
> by ≥ 10 pp while B&H was negative — **all from 2022**, several with spectacular
> single-path margins, e.g. SOL·2022 RSI B&H −94.2% vs strat +2.8%, a **+97.0 pp**
> margin); Stage 2 then block-bootstrap path-tested the top 16 (N=500, same frozen
> § 0 rule). **All 16 scored FRAGILE** — every candidate's p5 Sharpe is negative,
> including the +97 pp headline winner (p5 −0.888). The up-market contrast cell
> (SOL·2021 SMA) scored MARGINAL (p5 **+0.439**, positive), so the test
> discriminates regime direction; 9 of 16 candidates were RSI/BBands and none came
> back ROBUST. **Net effect on this runbook: ship-passive is firmed on the strongest
> available bear evidence.** This strengthens the same-day overfit-guard revision
> from "the 2024 hedge wasn't real" to "across a whole deep-bear universe — 40
> apparent winners, the single most spectacular +97 pp margin in the corpus — no
> apparent winner is path-robust." In a catastrophic bear almost any strategy that
> sat out the crash on the one realized ordering looks heroic on point returns;
> path-resampling separates lucky timing from a robust edge, and none survived. The
> high-value ROBUST-survivor tail (which would have REOPENED the active-vs-passive
> question for a v0.2.0 trend-following product line) did NOT appear, so the question
> stays closed. Scope-capped: still hourly, default params, 4 simple strategies, 10
> large-caps, the specific 2021-22 window — this FIRMS ship-passive but does NOT
> prove "no strategy can ever work." Confirmed numbers + scope cap:
> [`docs/dev-notes/analysis-2026-06-15-simple-strategy-bear-survey.md`](../dev-notes/analysis-2026-06-15-simple-strategy-bear-survey.md).

## Changelog

- 2026-06-15 (analyst): added the BEAR-SURVEY callout to the § Real-data validation
  section. A wider/deeper 2021-22 bear stress-test (`data/binance-2122/`, two-stage:
  80-cell point survey → 40 apparent winners, all 2022 → N=500 block-bootstrap on the
  top 16, frozen § 0 rule) scored all 16 candidates FRAGILE — including the +97.0 pp
  headline winner SOL·2022 RSI (p5 −0.888); up-market contrast SOL·2021 SMA MARGINAL
  (p5 +0.439, discriminates). This FIRMS ship-passive on the strongest available bear
  evidence, strengthening the same-day overfit-guard revision from "the 2024 hedge
  wasn't real" to "across a whole deep-bear universe, no apparent winner is
  path-robust." The high-value ROBUST-survivor reopen tail did NOT materialise →
  question stays closed. Scope-capped. Existing section history preserved; note
  appended, not rewritten. Evidence:
  [`docs/dev-notes/analysis-2026-06-15-simple-strategy-bear-survey.md`](../dev-notes/analysis-2026-06-15-simple-strategy-bear-survey.md).
- 2026-06-15 (analyst): added the REVISION callout to the § Real-data validation
  (2026-06-14) section. The survey's down-market trend-following hedge nuance did NOT
  survive path-robustness testing (block-bootstrap N=500, frozen § 0 rule → all 9 cells
  FRAGILE; positive median but negative p5 tail). The ship-passive base recommendation is
  now UNQUALIFIED on this evidence. Existing section history preserved; revision appended,
  not rewritten. Evidence:
  [`docs/dev-notes/analysis-2026-06-15-simple-strategy-overfit-guard.md`](../dev-notes/analysis-2026-06-15-simple-strategy-overfit-guard.md).

- 2026-06-08 (developer): produced realized equity-curve + full metrics for the BH baseline.
  Added `crates/backtest/examples/passive_baseline_equity.rs` (read-only probe, `--features realdata`).
  Wrote daily-sampled equity CSVs to
  `docs/runbooks/artifacts/passive-baseline-2026-06-08/bh-equity-curve-{2023,2024}.csv`.
  Updated `passive-baseline-characterization.md` with "Realized equity curve + full metrics" section.

  **Realized realized single-path metrics (data: revision 3a8b96c4, 10-sym equal-weight, $10k/sym):**

  | Year | Sharpe  | Sortino | Calmar | MaxDD%  | TotalReturn% | FinalEquity |
  |------|---------|---------|--------|---------|--------------|-------------|
  | 2023 | +1.8417 | +2.5126 | +5.677 | 34.57%  | +196.22%     | $296,221    |
  | 2024 | +0.8925 | +1.2047 | +1.853 | 48.95%  | +91.04%      | $191,040    |

  **Bootstrap reconciliation:** 2023 realized Sharpe +1.84 vs bootstrap p50 +1.74 (gap=+6.1%,
  realized is ABOVE the median bootstrap path — consistent: the actual 2023 bull leg was a
  strong path). 2024 realized +0.89 vs p50 +1.10 (gap=-19.2%, realized is BELOW the median
  — consistent: the 2024 actual path had a larger mid-year drawdown than many resampled paths).
  Both within expected single-path vs median-of-200 variance. No surprise; sign match.

  **Anchor gate:** 119/119 PASS. No stray files in `docs/archive/pre-bmad-spec/*/reports/`. Run command:
  `cargo run -p backtest --features realdata --example passive_baseline_equity`

- 2026-06-08 (developer): produced the passive-baseline characterization artifact
  at `docs/runbooks/artifacts/passive-baseline-2026-06-08/passive-baseline-characterization.md`
  by reading the 119 anchored sweep reports — zero new code. Key metrics confirmed:

  **2023 (8760 h bars, N=200 block-bootstrap-real, seed 0xC0FFEE, 10-symbol USDT perps):**
  - Sharpe p50 = **+1.735** (p5=+0.124, p95=+3.870; reconciles with the +1.74 bar in product.md)
  - P(loss) = 4.5% | P(Sharpe > 1.0) = 77.5% | p95 MaxDD = 51.15%
  - Byte-identical across 14 independent anchored reports

  **2024 (8784 h bars, same config):**
  - Sharpe p50 = **+1.105** (p5=-0.682, p95=+2.691; reconciles with the +1.10 bar in product.md)
  - P(loss) = 16.5% | P(Sharpe > 1.0) = 53.5% | p95 MaxDD = 64.83%
  - Byte-identical across 8 independent anchored reports

  **Actual BH construction (honest note):** The harness BH control is a **pure
  buy-once-hold** (no rebalancing, ever). Equal-weight initial allocation at bar-0
  close. No fee charged on the initial buy. Mark-to-market tracked per bar. This is
  NOT the proposed monthly/equal-weight cadence — that cadence is a forward operational
  proposal that has not been backtested under this harness.

  **Rebalance cadence decision:** The operator confirmed "go ahead" on the monthly /
  equal-weight default (2026-06-08 ratification). This is the designated operational
  cadence for the paper-trading agent. The +1.74/+1.10 Sharpe numbers characterize
  the pure-hold control; a monthly-rebalanced variant would differ slightly (small
  on a 10-symbol universe at monthly cadence) but has not been quantified.

  **Anchor gate:** 119/119 PASS verified. No stray reports written to anchored dirs.
  `git diff crates/` empty.

- 2026-06-08 (analyst): created on the terminal verdict of the active-vs-passive
  search (CONCLUDED across price + positioning + on-chain; ship passive). Defines
  the BH control as the canonical production baseline, the proposed monthly /
  equal-weight rebalance cadence (operator-confirmable), the paper-mode run
  recipe, and the explicit no-new-build boundary. Rebalance `(cadence, weighting)`
  pending operator confirmation.
