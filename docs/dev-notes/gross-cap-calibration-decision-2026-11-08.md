# The 0.50 gross cap: what enforcing it costs, and the one call that is left

> **LIVE — and the numbers below are superseded twice. Read this box, then skip to
> "What the runs actually measured".**
>
> The arithmetic in this memo (`10 × 0.10 = 1.00` of gross) describes a fully invested book
> these lanes never hold, and the 47 %-of-buys figure was inferred from a committed table
> rather than measured. I then measured one lane, found zero refusals, and wrote this memo
> off as resolved — which was the same mistake again, from one synthetic window.
>
> Measured across windows, the cap **does** bind, and not only on the lane that scales:
>
> | scenario | refusals | effect |
> |----------|----------|--------|
> | `top10-2023-fy-tcn-overlay-weights` (synthetic) | 0 | body byte-identical |
> | `top10-2023-fy-tcn-overlay-weights-realdata` | 9 | body moves |
> | `top10-2023-fy-patchtst-overlay-realdata` | 294 | body moves; **total return +41.81 % → +33.31 %** |
>
> So the decision below is live, and it now has the fact it was missing: enforcing the
> declared cap costs patchtst **8.5 percentage points** of published return. See bug-log
> `#176` CORRECTION 2 for the mechanism (leg count, not leg size — five legs at 0.10 sit at
> ~0.498 and the sixth breaches).

**Date:** 2026-11-08 · **Bug-log:** [`#176`](bug-log.md) (and [`#95`](bug-log.md), [`#71`](bug-log.md))
**Status:** LIVE. Code shipped; the re-emission and the calibration call are open.

## The question as it stood before the run

`portfolio_exposure_cap: Some(dec!(0.50))` is declared by six scenario lanes. Is that
**0.50 a risk limit that was never enforced**, or **a value that was never calibrated
against how these lanes size their legs**?

Both answers end in a re-emission. They differ in *what gets re-emitted*.

## The arithmetic that forces the question

- The universe is `top10_symbols_with_prices()` — **ten** symbols.
- Every lane sizes each Buy at a flat `let fraction = dec!(0.10);` of equity.
- A fully-invested book is therefore **`10 × 0.10 = 1.00`** gross.
- The declared cap is **`0.50`** — it permits at most **five of the ten** legs.

The strategy's own sizing does not agree with the lane's:
`cross_sectional::selector::top_k_long` assigns each leg `exposure_cap / k`, which for
a 0.50 cap over 10 symbols is **0.05** — half the lane's 0.10. The two were never
reconciled for a structural reason: **`trading_core::Signal` has no weight field**, so
the strategy's computed weight cannot cross the signal boundary and each lane picks a
size itself.

## What the runs actually measured

Each lane run through the binary with `--reports-dir` pointed outside `evidence/` (the
`#113` isolation flag), so nothing in the corpus was touched:

| scenario | peak gross | refusals | body |
|----------|-----------|----------|------|
| `top10-2023-fy-tcn-overlay-weights` (synthetic) | 0.363 | **0** | **byte-identical** — `a25a3693…`, `diff` empty |
| `top10-2023-fy-tcn-overlay-weights-realdata` | >0.50 | **9** | moves — Buys 3103 → 3094 |
| `top10-2023-fy-patchtst-overlay-realdata` | **0.598** | **294** | moves — Buys 1686 → 1392, **total return +41.81 % → +33.31 %** |

`3103 − 9 = 3094` and `1392 + 294 = 1686`: every refusal is exactly one Buy that no longer
happens.

**The mechanism is leg count, not leg size.** At a flat `fraction` of 0.10 a five-leg book
sits at ~0.498 of equity and the sixth leg breaches 0.50. The refusal warnings name it
directly — `XRPUSDT resulting_gross=0.5976 cap=0.50`. The synthetic window never carries a
sixth concurrent leg; the realdata windows do. patchtst crosses more often than TCN because
its forecaster dampens fewer signals, so it holds more legs at once.

This also corrects the first thing I wrote here. garch breaches because it SCALES its
notional (0.363 × 2.0 ≈ 0.74) — true, but not the only route. patchtst breaches with no
scaler at all. The 1469-refusal figure in garch's committed body is therefore the *scaling*
lane's cost, and says nothing about the unscaled ones; their cost is 9 and 294.

## The two readings

### A — 0.50 is the risk limit, and it was simply never enforced

Then these five lanes have run at up to **2× their declared limit** for their entire
recorded history, every report printed a cap that `Order::new` never consulted, and the
lower-turnover numbers are the honest ones. garch is already correct; the others catch
up to it.

*Re-emission scope:* the nine zero-count bodies, plus whatever verdict bodies move with
them (`regime-verdict-bs1-realdata`, `sharpe-comparison-*`).

### B — 0.50 was never calibrated against a 10 × 0.10 book

Then enforcing it restricts every strategy to half its universe — a **strategy change
wearing a bug fix's clothes**. The declaration is what should change: either the cap
rises to match the sizing (1.00), or the lanes' flat `fraction` drops to the
`exposure_cap / k` the strategy already computes (0.05), which would need `Signal` to
carry the weight.

*Re-emission scope:* under B, **garch's** currently-shipped 1469-refusal body is the
mis-calibrated one, so the group that moves is the opposite group.

## What shipped regardless of the answer

The gate itself is correct under either reading, so it is not held:

- `resulting_gross_fraction` / `would_breach_gross_cap` lifted from the garch scenario
  into `crates/risk/src/portfolio.rs` (one implementation for five lanes; tests now in
  a crate CI compiles with no feature opt-in).
- Four further lanes wired to the per-order entry gate.
- `crates/backtest/tests/gross_cap_declared_is_gross_cap_enforced.rs` — scans the
  scenarios directory so a *future* lane that declares a cap and enforces neither way
  fails by name, and the gate must precede the `Order::new` it gates. Both assertions
  probed with throwaway scan fixtures.
- The `equity > 0` conjunct, which no prior fixture separated from its absence, pinned
  by the one input that does (a negative cap) and probed.

## The gate state, stated plainly

`scripts/verify_anchors.sh` → **119 / 119 PASS**, before and after; nothing under
`evidence/` was touched. The System-2 half lives in `crates/backtest/tests/determinism.rs`
— six in-test anchor-reproduction pins that *re-run* these lanes
(`m3_*_tcn_overlay_weights_*`, `realdata_*_tcn_overlay_weights_reproduces_anchor`,
`realdata_*_regime_dispatcher_reproduces_anchor`).

I expected those pins to go red. One of them was run directly and did **not**:
`top10-2023-fy-tcn-overlay-weights` reproduced `a25a3693…` byte-for-byte with the gate
wired. The remaining five are feature-gated behind `candle,realdata`; the full gate is
`cargo test -p backtest --test determinism --features candle,realdata` and its result is
recorded in bug-log `#176`.

Should any of them move, the re-lock is additive and happens in the same pass as the body
re-emission, per ADR-0045 § D6.1 — but on the evidence so far there is nothing to move.
