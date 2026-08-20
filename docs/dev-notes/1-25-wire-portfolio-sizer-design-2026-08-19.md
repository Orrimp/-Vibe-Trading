# 1-25 — wiring `size_portfolio_target`: what the ruling actually requires

**Date:** 2026-08-19 · **Status:** design note, NOT implemented · **Ruling:** operator, 2026-08-19 —
*"wire it fully, accept both controls"* (supersedes the earlier "drop the drift axis")

## 1. The discovery that merged two defects into one

`#68` (drift axis inert) and `#69` (portfolio cap inert) were logged as separate findings. They are
the same defect. `risk::size_portfolio_target` implements **both**:

| control | site | what it does |
|---|---|---|
| portfolio cap | `portfolio.rs:189` | `if let Some(portfolio_cap) = limits.portfolio_exposure_cap` → validates Σ long notional ≤ cap × equity |
| drift hold band | `portfolio.rs:110` | `relative_drift > drift_threshold` → no-trade band; skip the rebalance if the position has not drifted far enough |

Neither is inert *by design*. Both are inert because **the one function that consumes them has no
production caller** — census: its definition, its own unit tests, and 3 sites in
`agent/tests/v1_rebalance_reject.rs`. `montecarlo.rs` 0, `param_robustness_sweep.rs` 0.

**The hold band is not missing. It is written, unit-tested, and never invoked.** That is why the
earlier "drop the axis" ruling was withdrawn — its premise ("implementing a hold band would be new
strategy behaviour") was false.

## 2. Why this is a rewrite, not a patch

The four fixes already landed (#67, #75, #71, #76) were each **one line contradicting logic the
codebase already had right**. This is not that.

`run_path` builds orders **per signal**:

```
for bar in &merged_bars {
    for sig in strategy.on_bar(bar) {
        … Order::new(sig.strategy_id, sig.symbol, side, qty, …)   // ×4 sites
        … engine.step(last_bar_by_symbol[&sig.symbol], vec![ord])
    }
}
```

`size_portfolio_target` consumes a **target vector**:

```
size_portfolio_target(
    targets: &BTreeMap<Symbol, TargetLeg>,   // symbol → { target_weight, mark_price }
    equity, position_book, drift_threshold, limits, strategy_id, ts,
) -> Result<Vec<Order>, PortfolioSizeError>
```

So wiring it requires restructuring the core loop:

1. **Accumulate** signals to a rebalance boundary instead of acting on each immediately.
2. **Build** `BTreeMap<Symbol, TargetLeg>` — target weights, not per-signal buy/sell intents.
3. **Call** the sizer, which decides Hold / Open / Close / Resize across the whole book atomically.
4. **Step** the returned `Vec<Order>` — which is already alphabetically sorted for determinism (R12.5).

That changes **when** orders are created and **which orders exist**. It is a change to the harness's
order-generation *model*, not a correction inside it.

## 3. Blast radius

- **Results move on every non-BUYHOLD lane.** The sizer's Hold decision alone (drift band) suppresses
  rebalances the current code performs unconditionally. Turnover falls; fee drag falls with it.
- **The cap can now REJECT.** `PortfolioSizeError::PortfolioExposureBreach` becomes reachable, where
  today the book simply runs to ~60–100 % gross. Every lane needs a decision on what a breach means:
  refuse the whole rebalance, or scale the vector down.
- **Interacts with #71.** `Order::new`'s per-symbol cap (now resulting-exposure aware) runs *inside*
  the sizer's loop, so both caps apply. Their interaction needs a test, not an assumption.
- **All 34 anchored surfaces move** — which is precisely what story `1-26` exists to absorb, and why
  its AC1 entry gate requires this to land first.

## 4. Why it needs an ADR

Per **AD-18**, every non-trivial decision gets a numbered ADR plus its Registry row in the same
commit. Changing the harness's order-generation model qualifies: it is the kind of decision a future
reader will need the *reasoning* for, not just the diff. The ADR should record at minimum:

- the target-vector model replacing per-signal construction, and why;
- the breach policy (reject vs scale) — a real choice with different research consequences;
- the interaction with `Order::new`'s per-symbol cap;
- that the drift band arrives as a **consequence** of wiring, not as a new feature.

## 5. Recommended next step

Do **not** start the loop restructure inside a long working session. The correct sequence is:

1. **ADR** for the order-generation model + breach policy (operator ratifies the breach policy — it
   changes what the research measures).
2. **Implement** the restructure behind the existing tests, expecting several to move.
3. **Binding tests per AC3** — one proving the portfolio cap actually refuses an over-cap vector, one
   proving the drift band actually suppresses a within-band rebalance. Both must be RED-provable:
   a gate for a limit that cannot fail is what created #69 in the first place.
4. **Then** `1-26` may run.

## 6. Status

Nothing in this note is implemented. The ruling is recorded on story `1-25`; `#68` and `#69` remain
open there with their ⚠️ annotations in place at their declarations, which keeps the code honest in
the meantime.
