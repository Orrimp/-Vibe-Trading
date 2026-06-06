---
slug: basis-reversal-vehicle-vs-signal-fork-2026-06-06
status: draft
owner: analyst
updated: 2026-06-06
tags: [perp-basis, basis-reversal, market-neutral, long-short, vehicle-vs-signal, funding-confound, cost-hurdle, on-chain, strategic-fork, post-ohlcv, durable-over-quick, go-no-go, dollar-neutral, short-leg, funding-accrual]
related:
  - spec/perp-basis-signal-robustness/feature.md
  - spec/perp-basis-signal-robustness/reports/test-2026-06-06-1200-perp-basis-signal-robustness.md
  - spec/dev-notes/new-data-domain-scoping-2026-06-05.md
  - spec/carry-strategy/feature.md
  - spec/carry-strategy/reports/test-2026-06-02-carry-strategy.md
  - spec/dev-notes/robustness-decision-rule-2026-05-30.md
  - spec/product.md
---

# Basis-reversal: vehicle vs signal — the v0.2.0 market-neutral fork

> **Mandate (analyst adjudication, FILES ONLY — orchestrator commits).** The
> `perp-basis-signal-robustness` v0.1.0 long-only arm closed PASS with science
> verdict **FAMILY-UNIFORM-FRAGILE at every fee level including 0 bps gross**
> ([test report](../perp-basis-signal-robustness/reports/test-2026-06-06-1200-perp-basis-signal-robustness.md)).
> Yet the underlying signal is the program's **first LIVE one** — a cross-sectional
> basis REVERSAL, rank-IC ≈ −0.08..−0.11, orthogonal to price/momentum, causal. The
> central question: **is the long-only-fragile verdict a death sentence for the basis
> signal, or a vehicle mismatch?** And then: **what is the highest-information next
> experiment — (A) build the market-neutral long/short v0.2.0 basis spread, or
> (B) route to on-chain?** Every number below traces to the eight anchored fee
> surfaces, the carry test report, or source inspected this session — no paraphrase
> trusted, no fabrication.

---

## 0. TL;DR — the verdict and the recommendation

**Vehicle, not signal.** The long-only FRAGILE verdict closes the **long-only
vehicle**, NOT the basis signal. The structural proof: the long-only arm carries
**full market beta** and captures only the **long-low-basis leg** of a reversal
*spread*, so its return ≈ market-beta + (½-ish of the spread tilt). It is benchmarked
against buy-and-hold's **+1.735** passive Sharpe — and it loses by 34–37× on p50 for
the same reason carry's long-only arm did: *the directional price exposure overwhelms
the tilt* (carry test report line 146, verbatim). The −0.10 IC was measured on the
**full long/short spread**; the long-only arm never tested it.

**Fees are demonstrably NOT the killer.** Across the {0,2,5,10}bps taker ladder the
best-cell p50 barely moves (g0: +0.0280 → +0.0259 from 0→5bps, a 0.0021 Sharpe move
for 5 bps of fee; g2/L=168: +0.04864 → +0.04690). The R-BR.LOAD fee-sweep — designed
to catch a turnover/fee-bleed death — instead **falsified the fee-bleed hypothesis**.
The arm is fragile at **0 bps gross**. The killer is the +1.74 beta bar, not friction.
This is the single most important diagnostic the v0.1.0 build produced, and it
**re-frames the whole fork**: the failure mode is NOT "the edge is too thin to clear
fees" (which would also kill a long/short arm), it is "the long-only vehicle drowns
the edge in beta" (which a market-neutral arm is built to fix).

**Recommendation: (A) BUILD the market-neutral long/short v0.2.0 spread — the durable
choice.** It is the only experiment that actually tests the signal the spike measured.
On-chain (B) is the right move ONLY IF the market-neutral spread also fails, and is
**deferred, not abandoned** — it remains the pre-registered next domain.

**Confidence: MEDIUM.** The cost hurdle for a crypto perp long/short is real and the
spread's net edge is unproven. But three facts make (A) the higher-information bet:
(i) the failure mode is beta, not fees, and a market-neutral arm strips beta by
construction; (ii) **the short-leg funding-cost model already exists** in the engine
(`montecarlo.rs:325-363`) — the v0.2.0 build is smaller than it looks; (iii) the
funding-confound is **directly testable** with data we already have banked, and
resolving it is itself decision-grade.

**If-budget-tightens annotation:** the strictly-cheaper fallback to a full v0.2.0
build is a **research spike first** (§ 6, Option A-lite): wire the short leg + the
dollar-neutral benchmark, run the {0,5}bps × {2023,2024} = 4 surfaces ONLY (skip the
8-surface full ladder), and gate the full build on whether the spread clears the ≈0
neutral benchmark at one cell. This is the "prove the spread has a net edge before
paying for the full anchored surface" lane, named explicitly so the operator has a
clean cheaper path.

---

## 1. Vehicle vs signal — the structural argument (precise)

### 1.1 What the long-only arm actually measured

The v0.1.0 arm (D-BR.0, framing (a)) longs the **lowest-basis** names via the
unchanged `top_k_long` selector, under the solvency-guarded long-only `run_path`
(`montecarlo.rs` opens only `Side::Buy`; `config.rs:308` rejects `k_short > 0` with
`unsupported_short_sizing`). Its return decomposes as:

```
r_longonly(t) ≈ β · r_market(t)  +  tilt_low_basis(t)
```

where `β ≈ 1` (it is fully invested long in K large-caps, equal-weight) and
`tilt_low_basis` is the **long leg only** of the cross-sectional reversal spread. The
buy-and-hold control is `r_market(t)` with `β = 1` and **zero tilt**. So the long-only
arm is, structurally, *buy-and-hold plus a small long-leg tilt* — and it is scored
against buy-and-hold. The tilt has to beat zero net of fees while dragging the entire
market beta along with it. The Sharpe arithmetic is brutal: a +0.05 p50 tilt-Sharpe
sitting on top of a +1.74 beta-Sharpe denominator does not move the needle, and the
tilt's own dispersion *widens* the drawdown tail (p95_maxdd 77–89% vs BH's 51%).

### 1.2 The numbers confirm the beta-swamp, not an edge-too-thin

| Quantity | Long-only basis-reversal (best cell, 2023) | BH control (2023) | Ratio |
|---|---|---|---|
| p50 Sharpe | **+0.0486** (g2, L=168, 0bps) | **+1.735** | BH 35.7× |
| p5 Sharpe | −0.0433 (g2) … −0.231 (g4) | +0.124 | BH positive, arm negative |
| P(Sharpe>1) | **0.000** every cell | 0.775 | — |
| p95 MaxDD | 77.3% (g2) … 89% (g1) | 51.15% | arm 1.5–1.7× worse |

The decisive read is the **fee-invariance**. If the edge were real-but-too-thin-for-
fees (the pre-registered R-BR.LOAD failure mode), p50 would *fall materially* as fees
rise. It does not:

| Cell | p50 @ 0bps | p50 @ 5bps | Δ (5bps of fee) |
|---|---|---|---|
| g0 (L=60, K=3, 8h) | +0.028011 | +0.025886 | −0.0021 |
| g2 (L=168, K=3, 8h) | +0.048645 | +0.046897 | −0.0017 |
| g3 (L=60, K=5, 24h) | +0.039698 | +0.037032 | −0.0027 |

A 5bps round-trip fee costs the arm ~0.002 Sharpe — **two orders of magnitude smaller
than the 1.70 gap to the BH bar.** The arm is not dying of fee-bleed; it is dying
because **+0.05 of tilt-Sharpe cannot out-run +1.74 of beta-Sharpe it is benchmarked
against.** That is a *vehicle* verdict, and it is exactly the verdict carry got
(§ 1.3). The −0.10 IC the spike measured lives in the **full long/short spread**, which
this vehicle never expressed.

### 1.3 The carry precedent makes this airtight

Carry (`carry-strategy`) ran the **identical long-only engine** on the **identical
universe** and came back FAMILY-UNIFORM-FRAGILE with best-cell p50 **+0.039** (2023) /
**+0.043** (2024). The carry test report's honest diagnosis (line 146, verbatim):

> "framing (a) long-only directional carry-tilt holds perp exposure on the
> negative-funding names, so P&L is **dominated by price risk rather than the funding
> premium. The long-only engine cannot isolate the funding signal from the price
> beta.** … The v0.2.0 durable follow-on (market-neutral long/short harvest, framing
> (b)) would need the short-side engine, and only warrants building if the [signal]
> has a directional edge."

Basis-reversal long-only's best cell (**+0.0486**) is **marginally better than carry's
(+0.039)** — consistent with the basis carrying a slightly stronger gross signal
(−0.10 IC vs funding's FRAGILE-everywhere) — but it dies for the **same structural
reason**: the long-only engine cannot strip the beta. Two strategies, same vehicle,
same beta-swamp failure, basis marginally ahead on the tilt. This is the cleanest
possible evidence that the verdict is about the **engine**, not the **signal**.

### 1.4 What a market-neutral arm changes — structurally

A dollar-neutral long-low/short-high basis spread is:

```
r_mn(t) ≈ tilt_low_basis(t)  −  tilt_high_basis(t)  −  funding_cost_short(t)  −  2×fee_drag
        = (the FULL reversal spread, β-stripped)     − (the new costs a short leg pays)
```

Two things change versus long-only, both decisive:

1. **Beta is stripped.** `r_market` drops out (long β ≈ short β ≈ 1, netted). The arm
   is no longer benchmarked against +1.74 — it is benchmarked against **≈ 0** (a
   dollar-neutral book's null is cash, not buy-and-hold). The 35× hurdle **disappears**.
   The frozen § 0 rule's BH control is replaced by a market-neutral null (see § 2.3 and
   the pre-registered rule in the feature brief).
2. **Both legs of the spread are captured.** The spike's −0.10 IC was measured on the
   full cross-section; the long-only arm captured ~half of it (the long-low-basis leg).
   The market-neutral arm captures the **whole spread** — plausibly ~2× the long-only
   tilt magnitude (see § 2.1 caveat on why "2×" is an upper bound).

**The verdict closes the vehicle, not the signal.** The honest statement: *we have not
yet tested the signal the spike measured.* v0.1.0 tested a beta-laden half of it.

---

## 2. Market-neutral v0.2.0 — viability under REAL costs (the crux)

This is where the recommendation has to earn its keep. A crypto perp long/short pays
costs the long-only arm never did. I estimate each against the plausible gross edge.

### 2.1 The gross edge — how big is the spread, really

The long-only arm's best-cell **tilt** is not +0.0486 — that number is mostly beta. The
*tilt itself* (long-only Sharpe minus a pure-beta long-only Sharpe) is the relevant
quantity, and it is small and hard to isolate from the surface alone. The cleaner
anchor is the **gross IC**: −0.10 rank-IC over L=60-168 on the full cross-section.

A rank-IC of 0.10 is a genuine but **thin** edge. The standard back-of-envelope
(Grinold's `IR ≈ IC · √breadth`) with breadth ≈ 10 names × ~rebalances/yr is
*optimistic* and not trustworthy for a 10-name universe (breadth is really ~the number
of independent cross-sectional bets, which on 10 correlated large-caps is far below
10). I will NOT manufacture a Sharpe from it (the fabricated-"Sharpe 1.40" precedent).
What I *can* say structurally:

- The market-neutral spread captures **both legs** vs the long-only's one, so the
  spread's gross tilt-Sharpe is plausibly **~1.5–2× the long-only tilt-Sharpe** — but
  the long-only tilt-Sharpe is itself small (the +0.0486 is ~95% beta). So "2× a small
  number" is still a small number. **The honest claim is: the spread MIGHT clear a
  ≈0 neutral benchmark where the long-only arm could not clear +1.74 — because the
  benchmark moved 1.74 Sharpe units in the arm's favour, not because the edge grew.**
- The decisive lever is **the benchmark change (1.74 units), not the leg-doubling**.
  Even if the spread's gross tilt is only +0.05 (no bigger than long-only's), beating
  ≈0 with +0.05 is a *positive* (if marginal) result, whereas beating +1.74 with +0.05
  was hopeless. **That is why (A) is worth running and (a) was always going to fail.**

### 2.2 The cost stack a short leg pays (and whether the edge clears it)

| Cost | Magnitude (per side, annualized-ish) | Clears the edge? |
|---|---|---|
| **(i) Perp funding on the SHORT leg** | **The load-bearing cost.** A short pays funding when funding is **positive**, and **high-basis names (the ones the arm shorts) carry high positive funding** (funding is the basis's mean-reversion mechanism). On large-caps in 2023-24, funding ran ~+10–30% annualized on crowded names → a short pays this as a **headwind**. This is both a real cost AND the funding-basis confound (§ 2.4). | **The binding question.** If the reversal edge (high-basis underperforms) is bigger than the funding the short pays, the spread survives; if the funding the short pays ≈ the underperformance it captures, the spread is **a wash by construction** — the basis-reversal would just be the mirror of the (already-fragile) funding-carry. **This is the experiment.** |
| **(ii) 2× fee/slippage drag** | Both legs trade. At 5bps taker + 2bps slippage = ~7bps/side round-trip, a dollar-neutral book pays ~2× the long-only's friction. BUT § 1.2 proved fees cost the long-only arm only ~0.002 Sharpe at 5bps → 2× is ~0.004 Sharpe. **Negligible relative to the edge.** | **YES** — fees were never the killer; doubling a negligible cost is still negligible. |
| **(iii) Borrow / short availability** | On Binance perps, "shorting" is opening a `Side::Sell` perp — **there is no borrow** (it is a derivative, not a stock loan). Availability is a non-issue for the 10 large-cap perps (all deeply liquid). The "borrow cost" IS the funding in (i). | **N/A** — folded into (i). No separate borrow cost on perps. |

**Net cost-hurdle assessment:** the hurdle is **almost entirely cost (i), the short-leg
funding**. Fees (ii) and borrow (iii) are non-issues for crypto perps. So the
market-neutral spread's viability reduces to **one clean question**: *does the
basis-reversal underperformance of high-basis names exceed the positive funding a short
on them pays?* This is answerable — and it is the experiment v0.2.0 runs.

### 2.3 The benchmark change is the whole game

The frozen § 0 rule benchmarks against buy-and-hold (+1.74 / +1.10). A dollar-neutral
book's correct null is **≈ 0 net-of-cost** (cash), NOT buy-and-hold — holding a
beta-neutral spread, the "do nothing" alternative is holding cash, which earns ~0
Sharpe. **This single change removes the 1.74 hurdle that killed every long-only arm.**

The pre-registered § 0 rule for the market-neutral arm (see the v0.2.0 feature brief
§ Pre-registration) replaces the BH control with a **dollar-neutral null** and keeps the
weakest-link bands (p5 Sharpe ≥ +0.5 ROBUST / < 0 FRAGILE; etc.). The arm must clear
**0 net-of-cost**, not +1.74. This is not goalpost-moving — it is the *correct* null for
a beta-neutral book, and it is the entire reason the vehicle change matters.

### 2.4 The funding-confound — explicit, and testable with data we already have

**The sharpest risk to (A): is basis-reversal just the mirror of funding-carry?** The
two are related by construction (funding is a clamped 8h settlement of the premium
index; spike BS.3b measured level corr **+0.47 (2023) / +0.66 (2024)**). The concern:

- Carry longs the **most-negative-funding** names (≈ lowest basis).
- Basis-reversal longs the **lowest-basis** names.
- **The long legs substantially overlap** (corr +0.47/+0.66). Carry's long-only arm
  failed; if basis-reversal long-only is mostly the same long leg, no surprise it also
  failed — *consistent with vehicle, not a new finding.*

But the **short legs are where they would diverge, and that is untested**:

- A market-neutral **basis** spread shorts the **highest-basis** names.
- A market-neutral **funding-carry** spread would short the **highest-funding** names.
- With corr +0.47/+0.66, these short books **overlap but are NOT identical** — the
  basis retains 45–78% variance funding discards (spike BS.3b). The basis-reversal
  short leg is shorting *crowded-premium* names; the funding short leg shorts
  *high-funding* names; the gap between them is the **distinct** part of the basis
  signal.

**The confound is directly testable — we already have the data.** The carry arm threads
`funding_by_symbol` through the bootstrap (`montecarlo.rs:113`, `funding_override`); the
basis arm threads `basis_by_symbol` (reusing the same channel, D-BR.3). So the v0.2.0
build can run **three market-neutral arms on the same paths**: (1) basis-spread,
(2) funding-spread, (3) **basis-orthogonalized-to-funding** (basis residual after
regressing out funding). If (1) ≈ (2), the basis-reversal IS the funding mirror and the
domain closes with finality (retire derivatives-positioning, route to on-chain). If (3)
retains a positive net edge, the basis carries **genuine orthogonal alpha** beyond
funding. **This disambiguation is itself decision-grade** and is the highest-value part
of the (A) experiment — it resolves a question the program has not been able to answer.

> **Honest prior on the confound:** MEDIUM-leaning-cautious. The +0.47/+0.66 long-leg
> overlap means a meaningful chunk of the basis signal IS funding. The bet is that the
> *short* leg's distinct variance (45–78%) + the beta-strip together produce a net edge
> the long-only vehicle hid. That is plausible but unproven — which is exactly why it
> must be **run**, not assumed.

---

## 3. The infra delta — what v0.2.0 actually requires (honest build size)

The v0.1.0 brief (§ Q-BR-2) and carry (§ D-CARRY.0) both deferred market-neutral to
v0.2.0 as "the short-side engine — a materially larger build." I inspected the engine
this session; the build is **smaller than that framing suggests**, because the
**short-leg funding-cost model already exists**.

### 3.1 What is already built (the load-bearing finding)

- **The short-leg funding accrual is written.** `montecarlo.rs:322-373` accrues
  `cash += notional × (−funding_rate)` at every 8h settlement boundary for held
  positions. Line 350 is currently `continue; // no short legs in framing (a)` — it
  **skips** short positions (`qty ≤ 0`). For a short (`qty < 0`), the existing formula
  `notional × (−rate)` with `notional < 0` *already produces the correct cost*: a short
  on a positive-funding name yields `(negative notional) × (−positive rate) = negative
  cashflow = a cost`. **The cost model is correct as written; only the `continue` skip
  gates it.** This is the single most important infra fact for the build estimate.
- `funding_by_symbol` / `funding_override` already thread funding through the bootstrap
  by the shared `idx_seq` (carry D-CARRY.7) — the short leg's funding lookup is free.
- The basis sidecar (`basis_data.rs`, `BasisDataSource`, `basis_as_of`,
  `build_basis_at_return`) is built and tested (M-DEV-0..3, 12 tests).
- The `ScoreSource::BasisReversal` arm + the LOAD-BEARING sign + the fee axis are built.

### 3.2 What v0.2.0 must add (the genuinely-new work)

| # | Change | Where | Size |
|---|---|---|---|
| 1 | **Un-gate short sizing** — allow `k_short > 0` (today `config.rs:308` hard-rejects it with `unsupported_short_sizing`). Add the bottom-K short selection in `selector.rs` (the `top_k_long` mirror — short the K highest-basis names). | `crates/strategy/cross_sectional/{config,selector}.rs` | ~moderate |
| 2 | **Short-position sizing + solvency in `run_path`** — open `Side::Sell` legs (today only `Side::Buy`), dollar-neutral notional split (½ long / ½ short), short-side solvency/margin accounting (the long-only Bug-B cap needs a short analogue). | `crates/backtest/scenarios/montecarlo.rs` | **the bulk** |
| 3 | **Enable the short-leg funding accrual** — replace line 350's `continue` with the short branch (the formula is already correct, § 3.1). Add `basis_by_symbol` OR reuse `funding_by_symbol` for the short-leg funding lookup (funding ≠ basis here — the SHORT pays FUNDING, the SIGNAL is BASIS — so v0.2.0 needs **both** sidecars live simultaneously, which retires the v0.1.0 "mutually exclusive, reuse the channel" simplification → the **sibling `basis_by_symbol` field** that D-BR.3 deferred is now owed). | `montecarlo.rs` + bootstrap | ~small-moderate |
| 4 | **Dollar-neutral § 0 benchmark** — replace the BH control with a ≈0 market-neutral null in the renderer; the frozen weakest-link bands stay. | `param_robustness_sweep.rs` renderer | ~small |
| 5 | **A `SelectionMode::LongShort`** (or equivalent) wiring the long-low/short-high split through to `run_path`. | strategy + sweep | ~small-moderate |
| 6 | **Day-1 falsifiers** — short-leg-divergence e2e, funding-cost-on-short non-no-op (mirror carry R-CARRY.10b), dollar-neutrality assertion (Σnotional ≈ 0), the funding-orthogonalization arm (§ 2.4). Plus the CLAUDE.md baseline-divergence gate. | new test file | ~moderate |
| 7 | **3 anchored arms** (basis-spread, funding-spread, basis⊥funding) × {0,5}bps × {2023,2024}, new namespace `perp-basis-mn-spread`. | sweep + anchors | ~moderate |

**Honest build-size estimate: ~5–8 dev-days.** This is **larger than v0.1.0's ~3–5d**
(it touches `run_path`, the one thing v0.1.0 kept byte-untouched, so the 99→107 anchor
guarantee no longer holds trivially — the short-side engine WILL produce a new
`MatchConfig` path and needs its own anchor-neutrality proof for the existing 107). But
it is **smaller than a from-scratch long/short engine** because the funding-cost model
(item 3, the part everyone assumes is hard) is already correct. The dominant new work is
item 2 (short solvency in `run_path`) and item 6 (the falsifier suite, including the
mandatory anchor-neutrality re-proof for the 107).

> **Anchor blast-radius warning for the architect:** item 2 modifies `run_path`, which
> v0.1.0 and carry both kept byte-untouched precisely to preserve the anchors. The
> market-neutral path MUST be additive/defaults-off (the long-only path byte-identical
> when `k_short = 0`) — the same discipline, but now applied to the `run_path` core, not
> just a `ScoreSource` arm. This is the M-T1 risk to design out, and the reason the
> build is ~5-8d not ~3-5d. ADR-0051 § D6.10 amendment owed (a 6th anchor-additive
> instance + the FIRST to touch `run_path`).

---

## 4. On-chain (option B) — the comparison

### 4.1 The #2-ranked domain's prior and shape

From the [scoping note](new-data-domain-scoping-2026-06-05.md) § 3 (domain B):

- **Hypothesis (orthogonality):** settlement-layer truth outside any price tape —
  exchange **net-flows** (coins to exchanges = sell pressure), **stablecoin supply**
  (mint/burn = dry powder), **active addresses** (adoption). The **strongest
  orthogonality story** on the board (genuinely not a function of OHLCV *or*
  derivatives positioning).
- **Data:** FREE-ish, full-history, **daily**. DeFiLlama (TVL, stablecoin supply, DEX
  vol — no key, full history); Glassnode/CryptoQuant free tiers are **daily + delayed**;
  Dune free tier (queryable, slow).
- **Harness fit:** MEDIUM — needs a **new fetcher** (per-source schema) + point-in-time
  hygiene (revisions/reorg), but the funding as-of-join template applies on a **daily**
  bar grid.
- **Cost:** **~5–8 dev-days** (fetcher + per-metric schema + PIT hygiene).
- **Prior:** MEDIUM — best orthogonality, but **daily-only** caps it: a 2-year daily
  window is **~730 points/series** — thin for a robust block-bootstrap tail (scoping
  note Assumption 2). The horizon-retest already built the daily resampler + corrected
  annualization, so that plumbing exists.

### 4.2 The expected-information comparison

| Dimension | (A) Market-neutral basis spread | (B) On-chain |
|---|---|---|
| **Tests a signal we KNOW is live?** | **YES** — −0.10 IC, sign-stable, causal, orthogonal (spike-proven). | **NO** — untested hypothesis; orthogonality is a *story*, IC unmeasured. |
| **Build cost** | ~5–8 dev-days (engine + falsifiers; funding-cost model already built). | ~5–8 dev-days (new fetcher + PIT hygiene + daily adapter). |
| **Data already banked?** | **YES** — `data/binance-basis` (`aa72409a…`) + `data/binance-funding` (`bf1ede44…`) both pinned. | **NO** — must fetch + PIT-clean from scratch. |
| **Resolution** | **Hourly** (8,760 pts/yr) — rich block-bootstrap tail. | **Daily** (~730 pts/yr) — thin tail, the binding limit. |
| **Resolves an open question?** | **YES** — the funding-confound (§ 2.4), unanswerable today. | Opens a new one. |
| **Decision-grade either way?** | **YES** — ROBUST = first survivable signal; FRAGILE = retire derivatives-positioning *with finality* (the long/short was the last untested vehicle) → route to on-chain with full justification. | YES, but a FRAGILE on-chain result leaves the basis spread forever untested (a dangling "what if"). |

**The asymmetry is decisive.** (A) costs the same as (B) but: tests a **proven-live**
signal vs an **untested** one; runs on **hourly** data already banked vs **daily** data
to fetch; and **resolves the funding-confound** — a question that, left open, will haunt
every future derivatives-positioning decision. Running (B) before (A) means spending
~5-8 days on an unmeasured hypothesis while leaving the program's **only measured live
signal** in an untested vehicle. That is the opposite of information-per-dollar.

**Crucially, (A) de-risks (B).** If the market-neutral spread is FRAGILE, the
derivatives-positioning family (price-rank + funding + basis, long AND short) is closed
*with finality* — there is no remaining vehicle to wonder about — and on-chain becomes
the clean, fully-justified next dollar. If the spread is ROBUST, the program has its
first product and on-chain can wait. **Either outcome of (A) sharpens the (B) decision;
running (B) first sharpens nothing.**

---

## 5. Recommendation — (A), with the durable-over-quick rationale

**Build the market-neutral long/short v0.2.0 basis spread (A).** The `(Recommended)`
tag goes on (A) — the durable choice — not on (B), per the operator's durable-over-quick
lens (AGENT.md 2026-05-28).

**Why (A) is durable, not merely the next thing:**

1. **It tests the actual signal.** v0.1.0 tested a beta-laden half of the spread and the
   verdict was about the vehicle (§ 1). Routing to on-chain now would close the basis
   domain on a verdict that **never tested the signal the spike measured** — an
   un-durable, "we'll wonder forever" close. (A) gives the basis signal its fair test.
2. **The failure mode points straight at it.** The fee-sweep falsified fee-bleed
   (§ 1.2); the killer is beta; a market-neutral arm strips beta by construction. The
   diagnostic the v0.1.0 build produced **literally argues for (A)**.
3. **The infra is mostly built** (§ 3.1) — the short-leg funding-cost model is already
   correct in `run_path`; v0.2.0 is engine-completion, not invention.
4. **It resolves the funding-confound** (§ 2.4) — a standing question that gates all
   future derivatives-positioning work. Durable = answer it once, with the data we have.
5. **Either outcome is decision-grade and de-risks on-chain** (§ 4.2). A clean ROBUST or
   a final FRAGILE; both sharpen the next fork. On-chain stays the pre-registered next
   domain, now entered with eyes open.

**If-budget-tightens (the named cheaper lane):** if ~5-8 dev-days is too much this cycle,
run the **Option A-lite spike** — wire the short leg + dollar-neutral null, run ONLY the
{0,5}bps × {2023,2024} = 4 surfaces (skip the 8-surface ladder; fees are not the killer
so the full ladder is low-value here), and gate the full anchored build on whether the
spread clears the ≈0 neutral benchmark at the best cell. This is the "prove the spread
has a net edge before paying for the full surface" fallback. It is **not** the
Recommended path (it spawns a follow-on if positive), but it is the clean cheaper lane.

### 5.1 Pre-registered experiment (the v0.2.0 falsifiable hypothesis)

Pre-registered now, before any number (program discipline + frozen § 0 inheritance).
The full pre-registration is appended to the new
[`spec/perp-basis-mn-spread/feature.md`](../perp-basis-mn-spread/feature.md); the
falsifiable core:

- **H0 (null):** the dollar-neutral basis spread has **zero** net-of-cost edge — its p50
  Sharpe ≤ 0 net of short-leg funding + 2× fees, and/or it is **statistically
  indistinguishable from the funding-carry spread** (basis ⊥ funding arm shows no
  residual edge). → derivatives-positioning closes with finality, route to on-chain.
- **H1 (alt):** the spread clears the **≈0 dollar-neutral null** (NOT the +1.74 BH bar —
  § 2.3) on the frozen weakest-link bands at the realistic 5bps fee, AND the
  basis⊥funding arm retains a positive net edge (the basis carries orthogonal alpha
  beyond funding). → first survivable active signal; new product direction.
- **Decision rule:** the frozen `robustness-decision-rule-2026-05-30.md` § 0 weakest-link
  composite, with the **BH control replaced by a dollar-neutral ≈0 null** (the only
  change; bands unchanged). Anti-cherry-pick FP-C3.5 (full surface, no argmax crown)
  preserved. Void-if-not `block-bootstrap-real`/`shared-index`.
- **Cost model:** short-leg funding via the existing `montecarlo.rs:325-363` accrual
  (`cash += notional × (−rate)`, short branch un-gated); 2× fee/slippage at the swept
  taker ladder {0,5}bps (the R-BR.LOAD minimum — full ladder optional since fees aren't
  the killer); no borrow cost (perps).
- **Kill-criteria:** (k1) the spread is FRAGILE at 0bps gross on the dollar-neutral null
  → the spread has no edge even before costs → close. (k2) basis-spread ≈ funding-spread
  (the basis⊥funding arm shows no residual) → it IS the funding mirror → close. (k3) the
  short-leg funding cost exceeds the high-basis underperformance it captures (the spread
  is a wash by construction) → close. Any of k1/k2/k3 → retire derivatives-positioning,
  route to on-chain.
- **Mandatory falsifiers (day 1, RED-on-revert):** dollar-neutrality (Σnotional ≈ 0);
  short-leg funding-cost non-no-op (mirror carry R-CARRY.10b — zero the short accrual,
  assert equity diverges); baseline-divergence e2e (CLAUDE.md non-negotiable); the
  basis⊥funding orthogonalization arm; no-look-ahead (inherited); two-run byte-identity;
  **the 107 existing anchors byte-identical when `k_short = 0`** (the new `run_path`
  anchor-neutrality re-proof — the load-bearing gate, § 3.2 warning).

---

## 6. Assumptions & limits (challengeable by operator / architect)

1. **The spread's gross edge is unproven and may be small.** The −0.10 IC is the gross
   ceiling, not net P&L (the same caveat that preceded the long-only FRAGILE). The bet is
   structural (beta-strip + leg-doubling + a ≈0 benchmark), not a promised Sharpe. I
   deliberately do NOT manufacture a target Sharpe (the fabricated-"1.40" precedent). The
   build is justified by **resolving the vehicle/signal + funding-confound questions**,
   not by optimism about a positive verdict.
2. **The funding-confound could close the domain (k2).** The +0.47/+0.66 funding overlap
   is real; if the basis⊥funding arm shows no residual, basis-reversal IS the funding
   mirror and the domain closes. This is a **likely-enough** outcome that it is a named
   kill-criterion, not a footnote.
3. **`run_path` is touched for the first time.** v0.1.0 and carry preserved the anchors
   by never touching `run_path`. The short-side engine cannot — so the build carries a
   real anchor-neutrality risk (the 107 must hold byte-identical when `k_short = 0`).
   This is the M-T1 design risk and the reason for the ~5-8d (not ~3-5d) estimate.
4. **The robustness axis judges resampled real 2023/2024 only** (inherited scope limit).
   A ROBUST verdict is "robust to resampled 2023/2024 net of cost," not "robust to all
   future basis regimes."
5. **On-chain is deferred, not abandoned.** It remains the pre-registered next domain.
   The recommendation is a **sequencing** call: test the proven-live signal in its
   correct vehicle FIRST (cheap, data-banked, resolves a standing question), THEN route
   to on-chain — with the derivatives-positioning family either retired-with-finality or
   superseded by a live product.
6. **Dollar-neutral null vs BH is the one frozen-rule change, and it is correct, not a
   goalpost move.** A beta-neutral book's null is cash (≈0 Sharpe), not buy-and-hold.
   Benchmarking a market-neutral arm against +1.74 would be the *wrong* null — it is the
   long-only mistake that this whole fork exists to correct.

---

## Changelog

- 2026-06-06 (analyst, basis-reversal vehicle-vs-signal fork): adjudicated the strategic
  fork after `perp-basis-signal-robustness` v0.1.0 closed PASS / FAMILY-UNIFORM-FRAGILE
  at all fee levels incl. 0bps gross. VERDICT: **vehicle, not signal** — the long-only
  arm carries full market beta + captures only the long-low-basis leg, benchmarked
  against BH +1.74; fees demonstrably NOT the killer (p50 moves ~0.002 Sharpe across the
  {0,2,5,10}bps ladder vs a 1.70 gap to the BH bar — the R-BR.LOAD fee-sweep falsified
  the fee-bleed hypothesis). The carry precedent (best long-only cell +0.039, "the
  long-only engine cannot isolate the [signal] from the price beta", test report line
  146) makes the vehicle-mismatch read airtight; basis-reversal long-only (+0.0486) is
  marginally better, same beta-swamp failure. Cost-hurdle assessment: the binding cost is
  short-leg perp funding (high-basis names = high positive funding → expensive to short
  AND the funding-confound); 2× fees are negligible (fees aren't the killer); no borrow on
  perps. The benchmark change (BH +1.74 → dollar-neutral ≈0) is the whole game — it
  removes the 35× hurdle. Funding-confound is DIRECTLY testable with banked data (a
  basis⊥funding orthogonalization arm) and is itself decision-grade. Infra delta: the
  short-leg funding-cost model ALREADY EXISTS (`montecarlo.rs:325-363`, `cash += notional
  × (−rate)`, line 350 `continue` skip the only gate); v0.2.0 ≈ ~5-8 dev-days (short
  solvency in run_path is the bulk + the first run_path touch → anchor-neutrality re-proof
  for the 107). On-chain (B) costs the same ~5-8d but tests an UNMEASURED hypothesis on
  DAILY data (~730 pts/yr) not yet banked, vs (A)'s PROVEN-LIVE signal on HOURLY data
  already pinned. RECOMMENDATION: **(A) market-neutral long/short v0.2.0** (the durable
  choice; on-chain deferred-not-abandoned, de-risked by (A)'s outcome either way).
  Pre-registered H0/H1 + dollar-neutral § 0 rule + cost model + k1/k2/k3 kill-criteria.
  If-budget-tightens: Option A-lite spike ({0,5}bps × {2023,2024} = 4 surfaces, gate full
  build on clearing the ≈0 null). Queued `perp-basis-mn-spread` in backlog; v0.2.0 feature
  brief authored. No code, no commit, no anchored-report edits.
