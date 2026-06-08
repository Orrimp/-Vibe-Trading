# Passive-Baseline Runbook — buy-and-hold as the production baseline

**Version:** v0
**Owner:** operator / analyst
**Status:** canonical baseline (active-edge search CONCLUDED 2026-06-08)
**Related code:** the buy-and-hold (BH) control in `crates/backtest` (the
benchmark every robustness surface is scored against)
**Decision of record:** [`spec/product.md` § Strategy library — Active-edge-search
status](../product.md) (terminal verdict: ship passive)
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
> guarantee across regimes. See [`product.md`](../product.md) for the full bounded
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
| Benchmark behaviour | Pinned by the BH anchor scenarios in `spec/anchors.toml` (the BH control the sweep scored against). |

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
4. Operator success reports (`spec/operator-success-reports/`) already headline
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

## Changelog

- 2026-06-08 (analyst): created on the terminal verdict of the active-vs-passive
  search (CONCLUDED across price + positioning + on-chain; ship passive). Defines
  the BH control as the canonical production baseline, the proposed monthly /
  equal-weight rebalance cadence (operator-confirmable), the paper-mode run
  recipe, and the explicit no-new-build boundary. Rebalance `(cadence, weighting)`
  pending operator confirmation.
