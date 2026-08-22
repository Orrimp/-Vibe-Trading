---
adr: 0089
title: Wire `size_portfolio_target` into the research harness — target-vector rebalance, and a breach policy that is visible
status: accepted
date: 2026-08-19
supersedes: none
superseded-by: none
---

# ADR-0089: Harness portfolio-sizer wiring (1-25 #68 + #69)

## Context

`risk::size_portfolio_target` implements two declared risk controls:

| control | site | behaviour |
|---|---|---|
| portfolio exposure cap | `portfolio.rs:189` | validates Σ long notional ≤ `portfolio_exposure_cap × equity` |
| drift hold band | `portfolio.rs:110` | `relative_drift > drift_threshold` → skip a rebalance that has not drifted far enough |

**It has zero production callers.** Census: its definition, its own unit tests, and three sites in
`agent/tests/v1_rebalance_reject.rs`. `montecarlo.rs` 0, `param_robustness_sweep.rs` 0.

So both controls are configured, documented, **range-validated**, printed into hashed report bodies —
and never consulted. Logged separately as bug-log **#69** (cap) and **#68** (drift axis) before it was
noticed they are the same defect: not two dead parameters, but one uncalled function.

The 2026-08-19 operator ruling is to wire it fully and accept both controls. That supersedes an
earlier same-day ruling to drop the drift axis, whose premise — "implementing a hold band would be new
strategy behaviour in a feature-complete project" — was false. The behaviour already exists.

## Decision

**D1 — Target-vector rebalance replaces per-signal order construction in the harness lanes.**
`run_path` currently calls `Order::new` per signal (4 sites) and steps each immediately.
`size_portfolio_target` consumes a `BTreeMap<Symbol, TargetLeg>` and returns the whole rebalance
atomically, alphabetically sorted for determinism (R12.5). The lanes accumulate signals to a
rebalance boundary, build the target vector, call the sizer, and step its output.

This changes **when** orders are created and **which orders exist**. That is the point: the sizer's
Hold/Open/Close/Resize decision is the thing that makes both controls real.

**D2 — Breach policy: SKIP the rebalance, COUNT it, SURFACE the count.**
`size_portfolio_target` already returns `Err(PortfolioExposureBreach)`; it does not scale. Rather than
modify the sizer, the caller honours that: on breach the rebalance is skipped, a counter increments,
and the count is surfaced in the run result.

Rejected alternative — **scale the vector down to fit the cap.** It is a defensible portfolio policy,
but it would mean editing the sizer to invent an allocation rule, and it silently converts a breach
into a smaller trade. A skipped rebalance is observable; a silently shrunk one is not.

Rejected alternative — **skip silently.** That reproduces the exact failure this project keeps
paying for: bug-log #66's class, where an unobservable skip reads as a clean run. The counter is
non-negotiable; a breach that leaves no trace is indistinguishable from no breach.

**D3 — Both caps apply, and their interaction is tested, not assumed.**
`Order::new`'s per-symbol cap (made resulting-exposure aware by bug-log #71) runs *inside* the sizer's
construction loop, so a rebalance passes only if it satisfies the per-symbol cap **and** the portfolio
cap. AC3 owes a test pinning that composition.

**D4 — The drift band arrives as a CONSEQUENCE of wiring, not as a feature.**
No new code implements it; it becomes reachable. The Tier-1 grid's third advertised axis
(`lookback × k_long × drift_rebalance_threshold`) becomes genuine, where today 54 of 58 cells sit at
0.10 and the axis changes nothing.

**D6 — Position sizing is PRESERVED: `target_weight = fixed_fraction` per selected leg.**
`Signal` carries only `Buy`/`Sell`/`Hold` — no weight — so wiring the sizer forces the harness to
derive one, and the two candidates disagree:

| rule | value at `k_long = 3`, `cap = 0.50` | status |
|---|---|---|
| harness today (`montecarlo.rs:628`, `equity × fraction`) | **0.10** | in use |
| `TargetLeg`'s documented domain `[0, exposure_cap / k_long]` | ~0.167 | what the sizer expects |

**Operator ruled 2026-08-19: preserve `fixed_fraction`.** The reason is attribution, not conservatism.
This wiring already enables two controls that move results (the cap and the drift band). Resizing
every position at the same time would conflate three changes, and no verdict move in the 1-26 re-lock
could be traced to a named cause. With sizing held constant, every delta is attributable to a control
we deliberately switched on.

Consequence, recorded honestly: at `fixed_fraction = 0.10` with `k_long = 3`, gross target is ~0.30 —
**below the 0.50 portfolio cap**, so on those cells the cap will not bind and the breach counter (D2)
will read zero. That is a true result, not a broken gate: the cap binds where the grid actually
exceeds it (the TS surfaces at ~90–100 % gross, the MN book at ~60 %). D5's RED-proof must therefore
construct an over-cap vector explicitly rather than hope a production lane supplies one.

**D5 — Binding tests must be RED-provable.** One proving the portfolio cap actually refuses an
over-cap vector; one proving the drift band actually suppresses a within-band rebalance. A gate for a
limit that cannot fail is precisely what created #69, so an assertion that passes without the control
being active is not acceptable evidence.

## D7 — `exposure_cap` MEANS GROSS: Σ |notional| (operator ruling, 2026-08-22)

The blocking question below is answered. **`exposure_cap = 0.50` is a limit on GROSS exposure —
Σ |notional| across all legs, long and short alike.**

Rationale, as ruled: gross is the quantity that actually carries risk in a long/short book. Both legs
can move against you, so a 3-long/3-short book at 0.60 gross is genuinely twice as exposed as a
0.30 long-only one. **Net** was rejected as near-vacuous here — it is ~0 *by construction* on a
market-neutral arm, so the cap could never bind on precisely the lanes it was written for.
**Long-only** was rejected because it ignores half the book: an MN arm could add unbounded short
exposure and never breach.

**Consequences, stated plainly because they are not comfortable:**

1. **The anchored MN surfaces DID breach their own declared limit.** At 6 legs × `fraction 0.10` the
   book runs **0.60 gross against a hashed `exposure_cap = 0.50`**. Bug-log #69's reading — "the
   declared limit is violated by construction on every MN path" — is now the **official** one, not a
   candidate interpretation.
2. ~~**The existing enforcer cannot implement this ruling.**~~ **RESOLVED 2026-08-22** —
   `size_portfolio_target` was extended to signed target weights with a gross-notional cap
   (`total_gross_notional`), signed `needs_order` including sign crossings, and signed emission
   (a negative `target_weight` opens a short; target 0 covers one). Its 13 long-only tests pass
   untouched: gross and long-only coincide when nothing is short.
3. **Anchor-impacting in a second way.** It is not only that the cap starts binding — surfaces which
   previously reported compliance were non-compliant under the ruled measure. 1-26's errata must say
   so per scenario.
4. **D1/D3/D4 are UNBLOCKED but re-scoped.** The target vector carries signed weights; the cap is
   evaluated on Σ |notional|; `crates/risk`'s long-only tests all need companions.

## ⚠️ RESOLVED by D7 — the analysis below is kept for the record

## ⚠️ PARTIALLY BLOCKED 2026-08-19 — D1 presupposes an enforcer that fits, and it does not

Implementation stopped before any code. `size_portfolio_target` is **structurally long-only**:
`TargetLeg.target_weight` is `[0, exposure_cap / k_long]` with `0 == close`, the only `Side::Sell` it
emits is sell-to-flat (`portfolio.rs:121-130`), and it caps on `total_long_notional` alone. `run_path`
has four transitions including **open-short** and **cover**, so routing the harness through it would
silently break every short-capable lane — MN (#108-#119), basis-reversal (#100-#107), `*_ls`,
`always_short`.

**This reframes #69**: the enforcer may have gone uncalled because it could not serve a long/short
harness, not because someone forgot to call it.

**And it surfaced the larger finding: `exposure_cap = 0.50` has never been defined.** The MN book is
6 legs at `fraction = 0.10`, so gross = 0.60 (breach), net ≈ 0.00 (far inside), long-only = 0.30
(inside). All three are defensible readings, they disagree about whether the anchored MN surfaces
violated their own declared limit, and the enforcer measures the one reading under which they did not.

**Still standing:** D2 (breach policy), D5 (RED-provable gates), D6 (sizing preserved).
**Now open:** D1, D3, D4 — all presuppose an enforcer that can express this book.

Options and a recommendation (specify the measure first, then extend or drop) are in
`docs/dev-notes/1-25-portfolio-control-rescope-2026-08-19.md`.

## Consequences

- **Anchor-impacting on every non-BUYHOLD lane.** All 34 surfaces in the 1-26 inventory move. That is
  what story `1-26` exists to absorb, and why its AC1 entry gate requires this to land first.

  ⚠️ **CORRECTED 2026-08-23 — this bullet originally read "the Hold decision suppresses rebalances the
  current code performs unconditionally, so turnover falls and fee drag falls with it". That is
  wrong.** The pre-wiring code could not resize a held position AT ALL: the long-open arm was guarded
  `Buy if current_qty <= 0`, and the strategy emits no Buy for a symbol it already holds. A held leg
  was opened once at `0.10 x equity` and never touched until it exited. So the drift band does not
  suppress behaviour the old code had — it BOUNDS behaviour this change introduces. Whether net
  turnover rises or falls is an empirical question for the 1-26 re-lock, and must be reported from
  measurement, not asserted here.
- **A previously unreachable error becomes reachable.** `PortfolioExposureBreach` can now fire on
  lanes that today run to ~60–100 % gross against a hashed `exposure_cap=0.50` claim.
- **The hashed claim becomes true.** Report bodies asserting `exposure_cap = 0.50` currently describe
  an intent; after this they describe the run.
- BUYHOLD rows stay clean — pure mark-to-market, never construct an `Order`, never reach the sizer.

## Implementation notes (2026-08-23) — two things the decision above got wrong

**D1's gate is the REBALANCE BOUNDARY, not a non-empty signal batch.** `build_rebalance_signals`
emits only TRANSITIONS — a leg that is held and still selected emits nothing — so once the book is
full a rebalance bar produces an EMPTY `Vec<Signal>`. Gating the sizer on `!signals.is_empty()`
compiles and looks right, and would have re-marked the book only when membership changed, leaving
the drift band very nearly as inert as #68 found it. `MomentumStrategy::last_rebalance_ts()` was
added so the harness can see the boundary itself; `run_path` latches it per timestamp, because on a
merged multi-symbol stream only the FIRST bar at a timestamp trips `is_rebalance_bar` while the
value stays equal to that timestamp for all of its siblings.

**Wiring surfaced bug-log #94, and it had to be fixed FIRST.** The sizer's resize branch emitted the
full target quantity rather than the delta — correct only against a "set position to X" API, while
every path here fills incrementally. First fixture through the resize path: **−74 % equity,
`min_cash_seen` 43.8 / 100 000**. It also DISABLED the control D4 is about: an overshooting order
leaves the leg outside the band on the next bar, so the band can hold nothing (10 fills delta-sized
vs 50 absolute-sized on the binding fixture). D5's band gate could not have been RED-proven without
this fix.

**Two hardcodes replaced by the config they claimed to honour.** `portfolio_exposure_cap` was
`Some(dec!(0.50))` in `run_path` while the report body printed the config's `exposure_cap`; the
drift threshold had no consumer at all. Both now come from `MomentumStrategy`. Every shipped grid
cell is at 0.50 / 0.10, so the cap change is behaviour-neutral on the corpus — but a config at any
other value was being silently ignored, which is the same declared-vs-executed shape as #69 itself.

**One behaviour change beyond the ruling, stated rather than buried.** The old open-short arm matched
`current_qty <= 0` and stacked another `0.10 x equity` short on top of an existing one without
bound. A target vector cannot express "add more" — `-FIXED_FRACTION` is a level — so a repeat Sell on
an already-short leg is now a Hold or a resize back to the level. Stacking was never a declared
control; it was an artefact of per-signal construction. Separately, the open-short arm's
`min(target_notional, cash)` cap was removed as DEAD by the same argument review 1-14 used on its
long-side twin: once notional is capped to cash the pre-flight becomes `cash >= cash + fee`, false
for any positive fee, so a capped short was always skipped anyway.

## References

- Design note: `docs/dev-notes/1-25-wire-portfolio-sizer-design-2026-08-19.md`
- Story: `1-25-harness-fill-correctness-relock` (AC3 enforce-or-delete).
- Blocks: `1-26-harness-relock-regeneration` AC1.
- Bug-log: **#69** (cap inert), **#68** (drift axis inert), **#71** (per-symbol cap, composes here).
