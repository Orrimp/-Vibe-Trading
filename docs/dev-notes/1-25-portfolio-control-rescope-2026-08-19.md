# 1-25 — re-scoping the portfolio exposure control for a long/short book

**Date:** 2026-08-19 · **Status:** analysis for an operator decision · **Triggered by:** the ruling to
wire `size_portfolio_target` (ADR-0089), stopped before implementation when the enforcer turned out
not to fit the harness it was meant to protect.

## 1. Why the wiring stopped

`risk::size_portfolio_target` is **structurally long-only**:

| evidence | site |
|---|---|
| `TargetLeg.target_weight` documented as `[0, exposure_cap / k_long]`, `0 == close` | `portfolio.rs:24` |
| the only `Side::Sell` it emits fires under `target_weight == 0 && current_qty > 0` — *sell to flat* | `:121-130` |
| it accumulates `total_long_notional` and caps on that alone | `:83, :116, :158, :191` |

There is no representation of a short position. `run_path`, meanwhile, has **four** transitions:

| branch | transition |
|---|---|
| `Buy` while `current_qty < 0`, `k_short > 0` | cover a short |
| `Buy` while `current_qty <= 0` | open long |
| `Sell` while `current_qty > 0` | close long |
| `Sell` while `current_qty <= 0`, `k_short > 0` | **open short** |

Routing the harness through this enforcer would silently break every short-capable lane — the MN
family (#108-#119), basis-reversal (#100-#107), the `*_ls` arms, `always_short`. Those are precisely
the surfaces the re-lock exists to correct.

**This reframes #69.** The enforcer may have gone uncalled not because someone forgot, but because it
could not serve a long/short harness in the first place. "Wire the existing function" is therefore not
a small decision deferred — it is a decision that cannot be executed as stated.

## 2. The deeper problem: "exposure_cap = 0.50" has never been defined

Every MN report body asserts `exposure_cap = 0.50`. The book is 6 legs at `fraction = 0.10`
(`montecarlo.rs:628` long, `:627-638` short — both hard-code `dec!(0.10)`). So:

| measure | value | vs 0.50 |
|---|---|---|
| **gross** (Σ \|notional\|) | 3 long × 0.10 + 3 short × 0.10 = **0.60** | **breach** |
| **net** (Σ signed notional) | 0.30 − 0.30 ≈ **0.00** | far inside |
| **long-only** (what the enforcer measures) | **0.30** | inside |

All three are defensible readings of "exposure", and they disagree about whether the anchored MN
surfaces violated their own declared limit. The bug-log records the breach as "~60 % gross vs the
hashed 0.50 claim" — which is true **only under the gross reading**, and the enforcer that was supposed
to prevent it measures the long-only quantity, where the same book sits comfortably inside.

For the long-only lanes the ambiguity is harmless: 3 legs × 0.10 = 0.30 under every reading. It bites
exactly where the short lanes live.

**So the corpus contains a limit whose units were never specified, on surfaces whose compliance
depends on which units you pick.** That is the real finding, and it is larger than "the function has
no caller".

## 3. What a correct control for this book would need

1. **A stated measure.** Gross, net, or long-only — chosen deliberately and written into the ADR and
   the report bodies. For a market-neutral arm, *net* is nearly meaningless (it is ~0 by construction)
   and *gross* is the quantity that carries the risk. For long-only lanes all three coincide.
2. **Signed target weights**, so a short leg is expressible at all.
3. **A leverage/margin view.** The short path already reserves `margin = notional / MAX_LEVERAGE`
   (`montecarlo.rs:640`). A gross cap and a margin constraint are different limits; deciding one does
   not decide the other.
4. **A breach policy that is visible** — ADR-0089 D2 already settles this: skip, count, surface.

## 4. Options

**(a) Extend the sizer to signed weights, cap on GROSS.** Closes #69 on every lane and matches what
the MN book actually does. Cost: modifies `crates/risk`, whose existing tests all assume long-only;
needs an ADR clause and its own binding tests. Highest value, highest blast radius.

**(b) Long-only lanes now, short lanes explicitly uncapped.** Route long-only through the existing
enforcer; leave short lanes on the current per-signal path with a loud, counted marker that they are
not portfolio-capped. Honest and incremental, but #69 closes only halfway and the MN bodies must stop
claiming a cap they do not have.

**(c) Drop the portfolio cap, keep only the per-symbol cap.** #71 already made the per-symbol cap
correct (resulting-exposure, side-aware, un-evadable). Delete `portfolio_exposure_cap` and strike the
claim from every body. Cheapest and fully honest, and it concedes that a portfolio-level control was
specified but never designed for this book.

**(d) Specify the measure first, then choose.** Rule on gross-vs-net-vs-long-only as a standalone
decision, then pick (a) or (c) with the units settled. Slowest, but it is the only order in which the
other options mean anything — and the units question is the one that has actually been unanswered
since the MN surfaces were locked.

## 5. Recommendation

**(d), then (a) or (c).** Every other path requires silently picking a definition of "exposure" that
the corpus has never stated, and the anchored MN bodies' compliance flips depending on that pick. The
units are not an implementation detail here — they are the finding.

Whatever is chosen, ADR-0089 needs amending: D1's "target-vector rebalance" assumed an enforcer that
can express this book, and it cannot.

## 6. Status

Nothing implemented. ADR-0089 stands as written but is now **partially blocked** — D2/D5/D6 survive
unchanged; D1/D3/D4 presuppose an enforcer that fits, which is the open question. `#68` and `#69`
remain open on story 1-25 with their ⚠️ annotations in place at their declarations.
