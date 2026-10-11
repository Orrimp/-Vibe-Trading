# bug-log `#95` — per-lane pricing of the "enforce or delete" ruling

**Date:** 2026-10-11 · **Method:** 8 parallel lane traces, each conclusion then challenged on two
axes (anchored-ness; exposure arithmetic) by independent agents instructed to refute. 25 agents,
0 errors. Four of eight conclusions were refuted by a challenger.

**Status: this is a decision memo, not a ruling.** Nothing here has been applied.

---

## 1. Per-lane

| Lane | Cap | Anchors | Enforcing moves a number? | Disputed |
|---|---|---|---|---|
| `garch_vol_target_overlay` | 0.50 | 7 rows / 5 bodies | **YES** | no — both challengers confirmed |
| `threshold_sweep` | 0.50 | 4 | **UNDETERMINED**, leans yes | yes — both challengers refuted the trace's "NO" |
| `regime_dispatcher` | 0.50 | 6 | **UNDETERMINED** | yes — the two challengers contradict *each other* |
| `patchtst_overlay_weights` | 0.50 | 3 | **UNDETERMINED** | no — trace and both challengers agree it is unknown |
| `tcn_overlay_weights` | 0.50 | 10 (+2 derived) | NO on the 2 synthetic rows, **UNDETERMINED** on the 2 realdata rows | partly |
| `tcn_overlay` | 0.50 | 10 (+2 derived) | NO | one challenger left it open |
| `momentum` | 0.50 | 5 | NO — a challenger counted parquet footers: only ADAUSDT ever fills, gross 0.10, zero refusals even at 0.50 | yes |
| `pairs` | 0.75 | 4 | NO | no |

**49 of the 119 anchors** sit on these 8 lanes, plus **2 blind-spot rows** —
`sharpe-comparison-realdata`, downstream of `tcn_overlay`, whose scenario name contains no lane
token, so a name-based grep never finds it.

Where the challengers won: **`momentum`** (measured, not reasoned) and **`threshold_sweep`** (the
trace's 3-leg/0.30 bound is false — a 44 % Sell→Hold dampen rate strands legs, the universe allows
10, and the cap binds at 6 concurrent legs). On **`regime_dispatcher`** neither challenger measured
anything, so it stays open rather than being resolved by majority.

## 2. The finding that inverts the question

**The false claim is not the config field. It is a hardcoded string in four report writers**, and
it is already printed across the corpus:

`report/pairs.rs:121`, `report/regime_dispatcher.rs:155`, `report/momentum.rs:113`,
`report/tcn_overlay.rs:196`.

Striking `portfolio_exposure_cap: Some(…)` is free on all 8 lanes — each passes `RiskLimits` only
into `Order::new`, which never reads the field (declared `crates/core/src/order.rs:84`; sole reader
`crates/risk/src/portfolio.rs:243`, whose only production caller is `montecarlo.rs:534`). It also
changes **zero report bytes**. So delete-the-field alone leaves every body asserting a cap while
removing the only trace of the intent behind it — the corpus gets *less* honest, not more.

**Correcting the printed line is the re-emission, in either branch.** That is the whole pricing
insight: the cheap-looking option costs a re-lock too.

## 3. Price of ENFORCE — three different prices

| what "enforce" means | anchors moved | cost |
|---|--:|---|
| **behaviour-only** (refuse/trim when Σ\|target\| > cap, no new body line) | 7 certain (garch), up to 15 more if the undetermined lanes bind | well under the `#141` 75-min window |
| **`#141`-style** (print the refusal count into the body) | **~51** — every body moves *even at a count of zero* | ≈1.5× `#141` ⇒ budget ~2 h plus the `realdata`/`candle` gates |
| **route through `size_portfolio_target`** | ~51 | **don't** — it replaces each lane's hardcoded `0.10`/leg with `exposure_cap / k_long` = 0.1667, a **67 % sizing increase**: a strategy change wearing a safety fix's clothes |

## 4. Recommendation (per-lane, deliberately not uniform)

1. **`garch_vol_target_overlay` — ENFORCE.** The only lane where 0.50 sits inside the live
   operating range: 7 of 10 symbols pin at `scale_clamp_max` 2.0, so any 3-leg basket is 0.50–0.60
   gross, and the body prints *"Dampen rate 100.00%"* beside *"exposure_cap=50%"*. Re-emit 5 bodies
   / 7 rows under D6.b, and re-check the vol verdict — `#137`'s flip is already pending there.
2. **`momentum`, `pairs`, `tcn_overlay` — strike the field now** (one commit, no anchor movement)
   **and fix the printed line in the same re-emission pass as garch.** Batching avoids two
   separate re-locks across one corpus.
3. **`threshold_sweep`, `patchtst_overlay_weights`, `regime_dispatcher`, `tcn_overlay_weights` —
   MEASURE FIRST.** Four instrumented runs printing peak gross and a refusal count to **stderr**,
   never into a body: no anchor movement, ~one compute window. Ruling on these now would be ruling
   on a guess — four of eight traces were refuted and one lane's challengers disagreed with each
   other.
4. **Nothing routes through `size_portfolio_target`.**

## 5. Still unknown, and what settles it

- **Realized peak gross on the four undetermined lanes.** No lane emits an exposure series
  (`position_curve: Vec::new()`), so no artifact records it. Settle with a temporary tracker:
  `max(Σ|notional| / equity)` plus a refusal count to stderr, one run per lane.
- **`regime_dispatcher` specifically:** do buys execute before offsetting sells inside one
  rebalance batch (a transient 0.60 gross), or does the end-of-bar net 0.30 hold? Cheapest of the
  four — execution order plus one counter.

## 6. Found in passing, routed elsewhere

- **`#111`'s open § Scope is now answered** — the 8 remaining `#67`-shaped `engine.step` sites
  across 6 lanes. Recorded in `#111` itself, with the call sites verified by hand.
- **`pairs` trades zero times at HEAD** while its two anchored bodies (locked 2026-04-30, *before*
  the `#67` guard) report **16 trades** each. Those 16 are `#67`-mispriced fills; the same orders
  now hit the guard and vanish. This is a concrete instance of `#93` — `verify_anchors.sh` hashes
  committed bodies and never re-runs, so it cannot see code-vs-evidence drift.
- **`Position::empty()` snapshots defeat `#71`'s resulting-exposure check** and can silently refuse
  a close. Visible at e.g. `scenarios/momentum.rs:365` — the risk check is handed an empty position,
  so "resulting exposure" is computed as though nothing were held.

---

*Provenance: the per-lane anchor counts and the exposure arithmetic are a sub-agent trace. Verified
independently before recording: the four hardcoded report-writer strings and that the corpus prints
them; the `#111` call-site enumeration and its `last_bar_by_symbol` discriminator; the `pairs`
anchored trade count of 16 and its 2026-04-30 lock date. The per-lane anchor counts were NOT
re-derived row-by-row.*
