---
slug: perp-basis-mn-spread
version: 0.2.0
status: proposed
owner: analyst
priority: P1
predecessor: perp-basis-signal-robustness v0.1.0
updated: 2026-06-06
---

# Perp-basis MARKET-NEUTRAL spread — the v0.2.0 follow-on that finally tests the basis signal in its correct vehicle (long low-basis / short high-basis, dollar-neutral, beta-stripped) — v0.2.0

> **The vehicle fix.** `perp-basis-signal-robustness` v0.1.0 (the long-only
> basis-reversal arm) closed PASS with science verdict **FAMILY-UNIFORM-FRAGILE at
> every fee level including 0 bps gross**
> ([test report](../perp-basis-signal-robustness/reports/test-2026-06-06-1200-perp-basis-signal-robustness.md)).
> The adjudication
> ([vehicle-vs-signal fork](../dev-notes/basis-reversal-vehicle-vs-signal-fork-2026-06-06.md))
> found this is a **VEHICLE verdict, not a signal verdict**: the long-only arm carries
> full market beta and captures only the long-low-basis leg of a reversal *spread*, so
> it is swamped by the +1.74 passive bar — exactly as carry's long-only arm was ("the
> long-only engine cannot isolate the [signal] from the price beta", carry test report
> line 146). The fee-sweep **falsified fee-bleed** (p50 moves ~0.002 Sharpe across the
> {0,2,5,10}bps ladder vs a 1.70 gap to BH). The −0.10 IC the spike measured lives in
> the **full long/short spread**, which v0.1.0 never tested. This brief formalizes the
> market-neutral build that does.
>
> **This brief is analyst-altitude only.** It scopes the WHY, the requirements, the
> day-1 falsifiers, the backtest scenarios, the cost model, and the framed design
> questions. It commits NO code, writes NO Design section, and authors NO tasks.md —
> the architect's M-T1 owns those next. The carry feature
> ([`carry-strategy`](../carry-strategy/feature.md)) and v0.1.0
> ([`perp-basis-signal-robustness`](../perp-basis-signal-robustness/feature.md)) are
> the line-for-line precedents.

---

## 0. Pre-registration & anti-cherry-pick (inherited, with ONE correct null change)

The market-neutral arm is vetted under the **already-frozen** pre-registered decision
rule ([`robustness-decision-rule-2026-05-30.md`](../dev-notes/robustness-decision-rule-2026-05-30.md) § 0)
— the same weakest-link composite that scored all four retired families, carry, and
the v0.1.0 long-only basis arm. **The bands are frozen and unchanged.** Exactly ONE
thing changes, and it is a **correction**, not a goalpost move:

- **The buy-and-hold control (+1.74 / +1.10) is REPLACED by a dollar-neutral ≈0 null.**
  A beta-neutral book's correct "do-nothing" alternative is **holding cash** (≈0
  Sharpe), NOT buy-and-hold. Benchmarking a market-neutral arm against +1.74 would be
  the *wrong* null — it is precisely the long-only mistake this whole feature exists to
  correct (see the fork note § 2.3). The market-neutral arm must clear **0 net-of-cost**
  on the frozen bands (p5 Sharpe ≥ +0.5 ROBUST / < 0 FRAGILE; prob-loss ≤ 15% / > 35%;
  p95 MaxDD ≤ ~50% / > ~70%; p50 Sharpe ≥ 1.0; P(Sharpe>1) ≥ 60%; composite =
  weakest-link). The dollar-neutral null carries no verdict (like the BH control before
  it); it is the bar the spread must clear to matter.

The other two commitments carry over verbatim:

1. **Anti-cherry-pick by construction (FP-C3.5).** The θ-sweep reports the FULL surface
   + a family verdict and **crowns no argmax winner**. A non-FRAGILE cell carries a
   `→ C5 deflation required` flag. (The renderer enforces this in code.)
2. **Pre-flight void-if-fail.** Every report must print `generator: block-bootstrap-real`
   AND `bootstrap_mode: shared-index`, else the verdict is void.

---

## Why

### Why market-neutral, and why now (the vehicle, not the signal, failed)

The v0.1.0 long-only arm and the adjudication established three facts that, together,
make the market-neutral spread the **only experiment that actually tests the signal**:

1. **The long-only verdict is about the vehicle.** `r_longonly ≈ β·r_market +
   tilt_low_basis` — buy-and-hold plus a small long-leg tilt, benchmarked against
   buy-and-hold. The tilt (+0.05 p50, ~95% of which is beta) cannot out-run the +1.74
   beta-Sharpe it carries. Best-cell p50 +0.0486 vs BH +1.735 = a 35.7× gap.
2. **Fees are NOT the killer.** The R-BR.LOAD fee-sweep was designed to catch a
   turnover/fee-bleed death; instead it **falsified that hypothesis** — p50 moves only
   ~0.002 Sharpe from 0→5bps (g0: +0.0280→+0.0259; g2: +0.04864→+0.04690), two orders
   of magnitude below the 1.70 gap to the bar. The arm is fragile at **0 bps gross**.
   The failure mode is **beta**, not friction.
3. **A market-neutral arm fixes exactly this.** `r_mn ≈ tilt_low − tilt_high −
   funding_cost_short − 2×fee`, β-stripped. Beta drops out (the +1.74 hurdle vanishes →
   the null is ≈0); BOTH legs of the spread are captured (where the spike's full −0.10
   IC lives). The arm is finally benchmarked against the right null.

The economic channel is the same the spike found: the basis is a readout of
**leveraged-positioning pressure**; crowded-long (high-basis) names subsequently
underperform. The long-only arm could only express the long-low-basis half while
dragging market beta; the market-neutral arm expresses the full reversal spread with
beta stripped. **This is the first time the signal the spike measured gets a fair test.**

### What we are buying — and the funding-confound we must resolve

Two decision-grade outputs, either of which justifies the build:

1. **Does the basis-reversal SPREAD beat a dollar-neutral ≈0 null net of real costs?**
   A ROBUST result is the program's **first survivable active signal** — a new product
   direction. A FRAGILE result retires the derivatives-positioning family **with
   finality** (price-rank + funding + basis, long AND short — there is no remaining
   vehicle to wonder about) and routes the next dollar to **on-chain** with full
   justification.
2. **Is basis-reversal genuinely orthogonal to funding-carry, or its mirror?** The basis
   and funding share +0.47/+0.66 level corr (spike BS.3b); carry already failed. The
   build runs THREE market-neutral arms on the SAME paths — basis-spread, funding-spread,
   and **basis-orthogonalized-to-funding** — to settle whether the basis carries alpha
   *beyond* funding. This disambiguation is unanswerable today and gates all future
   derivatives-positioning work. **Resolving it once, with banked data, is durable.**

### The honest prior (what would make the spread FRAGILE too)

State the failure modes up front so the verdict is read honestly:

- **The spread's gross edge may be too small even beta-stripped.** The −0.10 IC is the
  gross ceiling, not net P&L. Leg-doubling a small long-only tilt is still small; the
  decisive lever is the **benchmark change (1.74 units)**, not the leg-doubling. If the
  spread's gross tilt is FRAGILE even at 0bps on the ≈0 null, the signal has no
  harvestable edge (kill-criterion k1).
- **The funding-confound could close the domain (k2).** If the basis⊥funding arm shows
  no residual edge, basis-reversal IS the funding mirror — and funding already failed.
  The +0.47/+0.66 long-leg overlap makes this a **likely-enough** outcome to be a named
  kill-criterion, not a footnote.
- **The short-leg funding cost could eat the edge (k3).** High-basis names (the short
  leg) carry high positive funding; a short PAYS that funding. If the funding paid ≈ the
  underperformance captured, the spread is a wash by construction.
- **The robustness axis judges resampled real 2023/2024 only** (inherited scope limit).

If the spread ALSO comes back FRAGILE, that is **again a methodology win** — the machine
will have ruled out the last untested vehicle of the strongest post-OHLCV signal — and
on-chain is the clean, pre-registered next domain. The brief does not overclaim.

---

## Requirements

> **Naming:** requirements are tagged **R-MN.\*** (MN = Market-Neutral). Each maps to a
> carry / v0.1.0 sibling where a precedent exists. The SHORT-LEG ENGINE (R-MN.2), the
> SHORT-LEG FUNDING COST (R-MN.3), and the FUNDING-ORTHOGONALIZATION arm (R-MN.6) are
> the three things that must be exactly right — they are the difference between a real
> test and a confounded one.

### R-MN.1 — Signal: the SAME cross-sectional basis-reversal score (inherited verbatim)

_(v0.1.0 sibling: R-BR.1/R-BR.2)_ The basis score is unchanged from v0.1.0:
`basis_reversal_score(s,t) = −mean(basis[s,τ] for τ in [t−L, t))` over the basis-close
series (`data/binance-basis`, pin `aa72409a…`), CROSS-SECTIONAL rank. The LOAD-BEARING
minus (tilt AGAINST the basis) is inherited and stays in ONE place. The ONLY change is
the **selection**: v0.1.0 longs the top-K lowest-basis; v0.2.0 ALSO shorts the bottom-K
highest-basis (R-MN.2).

_Acceptance: the v0.1.0 `basis_reversal_score` is reused unchanged; the sign-assertion
falsifier (a flip = a basis-momentum payer) carries over and stays GREEN._

### R-MN.2 — The SHORT-LEG ENGINE: dollar-neutral long-low / short-high (LOAD-BEARING)

_(carry sibling: the deferred D-CARRY.0 framing (b); v0.1.0 sibling: the deferred
Q-BR-2 framing (b))_ This is the genuinely-new engine work. The arm must:

- **Un-gate short sizing** — allow `k_short > 0` (today `config.rs:308` hard-rejects it
  with `[unsupported_short_sizing] k_short > 0 is not supported in v1`). Add the
  bottom-K short selection (the `top_k_long` mirror — short the K highest-basis names).
- **Open `Side::Sell` legs** in `run_path` (today `montecarlo.rs` opens only
  `Side::Buy`). Dollar-neutral notional: ½ long-low-basis, ½ short-high-basis.
- **Short-side solvency / margin accounting** — the long-only Bug-B solvency cap
  (`montecarlo.rs`) needs a short analogue (a short position's loss is unbounded above;
  the engine must cap/liquidate consistently).
- **A `SelectionMode::LongShort`** (or the architect's equivalent) wiring the
  long-low/short-high split through to `run_path`.

> **A naive long/short that is not dollar-neutral re-introduces beta** — the exact thing
> this feature exists to strip. **R-MN.7 (the dollar-neutrality falsifier, day 1) is
> mandatory:** assert Σnotional ≈ 0 (long notional ≈ short notional) at every rebalance
> — RED if the book carries net directional exposure.

> **OPEN QUESTION Q-MN-2 (architect M-T1 — load-bearing).** The short-side engine in
> `run_path` is the FIRST time `run_path` is touched since v0.1.0 and carry deliberately
> kept it byte-untouched to preserve the anchors. The architect must design the
> short-side path **additive/defaults-off** so the long-only path is byte-identical when
> `k_short = 0` (the 107 existing anchors hold). This is the load-bearing anchor risk
> (R-MN.8). Decide: (i) a `k_short`-gated branch inside the existing `run_path` (smallest
> blast radius, but `run_path` is now conditionally long/short) vs (ii) a sibling
> `run_path_long_short` (cleaner separation, but +1 call-site + duplicated solvency
> logic). **Analyst lean: (i)** — the gated branch keeps ONE engine and the
> anchor-neutrality proof is "the `k_short = 0` path is the unchanged code"; (ii)
> duplicates the solvency logic (a correctness risk) for separation the feature does not
> yet need. The architect ratifies.

### R-MN.3 — The SHORT-LEG FUNDING COST (the binding cost; the model ALREADY EXISTS)

_(carry sibling: R-CARRY.8 / D-CARRY.7 — the accrual block)_ **This is the binding cost
of the whole feature, and the model is already written.** `montecarlo.rs:322-373`
accrues `cash += notional × (−funding_rate)` at every 8h settlement boundary. Line 350
is currently `continue; // no short legs in framing (a)` — it **skips** short positions.
For a short (`qty < 0`, `notional < 0`), the existing formula `notional × (−rate)`
*already produces the correct cost*: a short on a positive-funding name yields
`(negative notional) × (−positive rate) = negative cashflow = a cost`. **The cost model
is correct as written; only the `continue` skip gates it.** The change is to enable the
short branch.

- **The short leg pays FUNDING; the signal is BASIS.** v0.2.0 needs BOTH sidecars live
  **simultaneously** (the short-leg funding lookup AND the basis-reversal score) — this
  retires the v0.1.0 "basis + funding mutually exclusive, reuse the `funding_by_symbol`
  channel" simplification (D-BR.3). **The sibling `basis_by_symbol` field that D-BR.3
  deferred is now owed** (Q-MN-3).
- High-basis names (the short leg) carry high positive funding (funding is the basis's
  mean-reversion mechanism), so the short pays a real headwind. **Whether the reversal
  edge exceeds this funding is the experiment (k3).**

_Acceptance: a short-leg funding-cost non-no-op falsifier (mirror carry R-CARRY.10b):
zero the short-leg accrual → assert equity diverges (the cost is load-bearing, not a
computed-and-ignored no-op); RED on revert._

### R-MN.4 — Universe + data pins (the SAME 10 large-caps + funding, for comparability)

_(v0.1.0 sibling: R-BR.4)_ The universe is the **SAME 10 large-caps** under
`data/binance` (pin `3a8b96c4…`) + basis `data/binance-basis` (pin `aa72409a…`) + **the
funding side `data/binance-funding` (pin `bf1ede44…`)** — now load-bearing for the
short-leg cost. Keeping the universe identical to v0.1.0 + carry + the four retired
families makes the result directly comparable. In-sample = 2023-FY; 2024-FY gating, both
regimes day 1 (the carry/horizon E1 precedent).

### R-MN.5 — As-of joins, strict no-look-ahead (inherited, now TWO sidecars)

_(v0.1.0 sibling: R-BR.5; carry sibling: R-CARRY.6)_ Both the basis (`basis_close[t-1]`
at open of t) AND the funding (as-of the prior 8h settlement) join causally, past-only.
Both inherit their proven no-look-ahead falsifiers. The two-sidecar simultaneity (R-MN.3)
must not introduce a leak in either.

_Acceptance: the inherited no-look-ahead falsifiers (basis + funding) both stay GREEN
under simultaneous threading; shifting either series into the future changes the result._

### R-MN.6 — The FUNDING-ORTHOGONALIZATION arm (the confound resolver — decision-grade)

**This is the highest-value output of the feature.** To settle whether basis-reversal is
genuinely orthogonal to funding-carry or its mirror, the build runs **THREE
market-neutral arms on the SAME bootstrap paths**:

| Arm | Long leg | Short leg | What it isolates |
|---|---|---|---|
| **(1) basis-spread** | lowest basis | highest basis | the raw basis-reversal spread |
| **(2) funding-spread** | most-negative funding | most-positive funding | the funding-carry spread (the mirror candidate) |
| **(3) basis⊥funding** | lowest basis-RESIDUAL | highest basis-RESIDUAL | basis after regressing out funding — the DISTINCT basis alpha |

where the basis-residual in (3) is `basis − β̂·funding` (the cross-sectional residual of
basis on funding). **If (1) ≈ (2)**, basis-reversal IS the funding mirror → domain closes
with finality (k2). **If (3) retains a positive net edge**, the basis carries genuine
orthogonal alpha beyond funding → the signal is real and distinct.

_Acceptance: all three arms run on the same N paths under the same § 0 rule; the
three-way comparison is the headline deliverable; (3)'s net edge vs (1)/(2) is reported
explicitly (NOT a silent omission)._

### R-MN.7 — Day-1 falsifiers (each RED-on-revert)

_(carry siblings: R-CARRY.10a/10b/2/6; v0.1.0 siblings: R-BR.7)_ Per CLAUDE.md and the
carry/v0.1.0 precedent, the market-neutral arm ships these falsifiers on day 1:

1. **Dollar-neutrality (R-MN.2).** Σnotional ≈ 0 at every rebalance; RED if the book
   carries net directional exposure (the beta-leak guard).
2. **Short-leg funding-cost non-no-op (R-MN.3).** Zero the short accrual → equity
   diverges; RED on revert (the carry R-CARRY.10b analogue — guards against a
   computed-and-ignored cost).
3. **Baseline-equity-divergence e2e (CLAUDE.md non-negotiable).** The market-neutral
   arm's equity diverges from the un-tilted baseline by ≥ 1 bp when the signal is
   non-trivial. Pattern: `crates/strategy/tests/vol_targeting_overlay_end_to_end.rs`.
4. **Sign-assertion (inherited R-BR.2).** A sign flip = a basis-momentum payer → RED.
5. **No-look-ahead (R-MN.5).** Both basis + funding joins past-only; RED on a future
   shift of either.
6. **Two-run byte-identity.** Same `ensemble_seed` → identical body-SHA.
7. **The basis⊥funding orthogonalization arm produces a DIFFERENT result from the raw
   basis-spread** (proving the residualization is load-bearing, not a no-op).

### R-MN.8 — Determinism & anchoring: the 107 anchors hold byte-identical (the run_path re-proof)

_(v0.1.0 sibling: R-BR.8; carry sibling: R-CARRY.11)_ **This is the load-bearing
anchor gate, and it is harder than v0.1.0's** because the short-side engine touches
`run_path` — the one thing v0.1.0 and carry kept byte-untouched. Every new seam (the
short selection, the `Side::Sell` path, the short solvency, the short funding accrual,
the second sidecar) must be **additive and defaults-off**, gated on `k_short > 0`, so the
long-only path is **byte-identical when `k_short = 0`**. **The 107 existing anchors
(`spec/anchors.toml`) stay byte-identical — a hard gate, NOT a goal:**

- `run_path` with `k_short = 0` must produce identical equity to today (the new short
  branch is never entered) — this is the FIRST `run_path` anchor-neutrality re-proof and
  the M-T1 design risk (Q-MN-2).
- The second sidecar (`basis_by_symbol`) must be `None` for every non-MN run.
- `SelectionMode::LongShort` must not change the default `CrossSectionalTopK`
  serialization.

**Anchor unit = the three-arm market-neutral θ × fee surfaces.** New anchors are added
after the developer's anchored run (the tester locks them at M-TEST PASS). **Anchored
report files in `spec/*/reports/` remain byte-immutable** (ADR-0038 § D6). ADR-0051
**§ D6.10 amendment owed** — the 6th anchor-additive instance and the FIRST to touch
`run_path`.

### R-MN.9 — Decimal money throughout (inherited)

_(inherited, ADR-0003)_ The basis, funding, the residual regression (the β̂ in R-MN.6),
the short-leg notional, and the funding accrual stay `rust_decimal::Decimal`. No `f64`
in the money path or the signal. The cross-sectional residualization (R-MN.6) is the one
new computation — confirm it is Decimal-exact (the architect may need a Decimal OLS
slope, or a rank-based residual to avoid float regression — Q-MN-4).

---

## Requirements summary (consolidated)

- **R-MN.1** — SAME basis-reversal score (inherited); only the selection changes.
- **R-MN.2** — SHORT-LEG ENGINE (LOAD-BEARING): un-gate `k_short`, `Side::Sell` legs,
  short solvency, dollar-neutral split, `SelectionMode::LongShort`. Day-1
  dollar-neutrality falsifier mandatory.
- **R-MN.3** — SHORT-LEG FUNDING COST (the binding cost; the accrual model ALREADY
  EXISTS — only line 350's `continue` skip gates it). Needs BOTH sidecars live.
- **R-MN.4** — SAME 10 large-caps + basis pin `aa72409a…` + funding pin `bf1ede44…`;
  2023 + 2024 both day 1.
- **R-MN.5** — As-of basis + funding joins, strict no-look-ahead (inherited, two
  sidecars).
- **R-MN.6** — The FUNDING-ORTHOGONALIZATION arm (basis / funding / basis⊥funding,
  three arms, same paths) — the confound resolver, the headline.
- **R-MN.7** — Day-1 falsifiers (each RED-on-revert): dollar-neutrality, short-funding
  non-no-op, baseline-divergence e2e, sign-assertion, no-look-ahead, two-run identity,
  orthogonalization-non-no-op.
- **R-MN.8** — Additive/defaults-off → the 107 anchors hold byte-identical (the FIRST
  run_path anchor-neutrality re-proof — the load-bearing gate).
- **R-MN.9** — Decimal money throughout (the residual regression is the one new compute
  — confirm Decimal-exact).

---

## Design
_Architect M-T1 fills this. Q-MN-1..5 resolved + justified. The deferred carry
framing (b) / v0.1.0 Q-BR-2 framing (b) is the precedent the architect lifts._

## Backtest Scenarios

_Analyst PLAN (architect ratifies + LOCKS at M-T1). The grid, the fee ladder, and N
become LOCKED hashed body fields per the MR/carry/TS/horizon/v0.1.0 precedent._

The primary anchored deliverable is a **three-arm × θ × fee surface** — the
market-neutral basis spread, the funding spread, and the basis⊥funding spread, swept
over the signal-bearing lookback band, at the realistic fee level, on 2023 + 2024, vs
the **dollar-neutral ≈0 null** (NOT buy-and-hold).

**The θ-axis (signal lookback) — inherit v0.1.0's signal-bearing band:** `L ∈ {60, 168}`
bars (the IC peak; the analyst proposes DROPPING the faster L=24 cell — for a spread the
turnover lever matters less since fees aren't the killer, and the IC peaks at L=60-168;
the architect may keep L=24 for continuity). SKIP L=720 (noise, inherited).

**The arms axis — the headline (R-MN.6):** {basis-spread, funding-spread, basis⊥funding},
three arms on the same paths.

**The fee-axis — {0, 5} bps (the R-BR.LOAD minimum, NOT the full ladder).** The v0.1.0
fee-sweep falsified fee-bleed (p50 moves ~0.002 Sharpe across the full ladder), so the
full {0,2,5,10} ladder is **low-value** here. The {0,5}bps × {2023,2024} read (gross
ceiling + realistic decision point) is sufficient. (The architect may run the full ladder
if cheap, but it is not required — this is a deliberate scope economy justified by the
v0.1.0 fee-invariance finding.)

**The sizing axis:** K ∈ a small locked set (the long-K = short-K dollar-neutral split);
rebalance cadence as a turnover lever (less critical than v0.1.0 since fees aren't the
killer).

**N:** N = 200/cell on 2023 + 2024 (the carry/MR/TS/v0.1.0 tractable shape; the architect
re-validates the wall-clock — note the arms axis multiplies the surface count: 3 arms ×
2 fee × 2 regime = up to 12 surfaces, so the architect must confirm `|arms| × |L| × |K|
× |fee| × N × 2 regimes` is tractable, mirroring the v0.1.0 D-BR.WALLCLOCK gate).

**Plan to anchor the surfaces** (the tester locks them at M-TEST PASS):

1. **MN-SPREAD-θ×FEE (PRIMARY, ANCHORED)** — the three-arm market-neutral surface,
   N=200/cell, shared-index block-bootstrap of 2023-FY real OHLCV + the shared-index
   basis AND funding series. Per-cell FRAGILE/MARGINAL/ROBUST + family verdict + per-cell
   `→ C5` flags + the trades + funding-cost columns + **the net-of-cost edge vs the ≈0
   null at each fee level** (R-MN.3) + **the three-way arm comparison** (R-MN.6).
2. **Dollar-neutral null (in the surface)** — a ≈0-Sharpe cash-equivalent null under the
   same N paths. Carries no verdict; it is the bar the spread must clear.
3. **2024-FY surface** — the SAME locked grid + arms on 2024-FY (the harder regime).

> **Namespace (proposed — architect ratifies):** a NEW `perp-basis-mn-spread` anchor
> namespace (a new program phase — the first market-neutral arm), with `verify_anchors.sh`
> extended (an additive `elif` branch after the `perp-basis-signal-robustness` branch).
> Scenario names carry the arm + fee level:
> `v2-mn-{arm}-fee{NN}bps-theta-surface-{year}-block-bootstrap-real-fy`
> (`{arm}` ∈ {basis,funding,basisperp}; `{NN}` ∈ {00,05}).

---

## Verification
_Tester links reports here after the M-TEST gate. The gates the tester must clear:_

1. **The three-arm comparison (R-MN.6) — the headline.** The net-of-cost edge of
   basis-spread vs funding-spread vs basis⊥funding is reported at each fee level vs the
   ≈0 null; the confound verdict (is basis the funding mirror, or distinct?) is explicit.
2. **The dollar-neutral verdict.** Does the spread clear the ≈0 null on the frozen § 0
   bands at the realistic fee? (NOT the +1.74 BH bar.)
3. **The day-1 falsifiers RED-on-revert (R-MN.7).** Each GREEN as written AND RED when
   its guard is reverted (genuine guards).
4. **The 107 existing anchors byte-identical** (`verify_anchors.sh` 107/107) with
   `k_short = 0` — the FIRST run_path anchor-neutrality re-proof — + the new MN anchors
   locked.
5. **Two-run byte-identity** of the MN surface body-SHA.
6. **Pre-flight void-if-fail** — `generator: block-bootstrap-real` AND
   `bootstrap_mode: shared-index`.
7. **The frozen § 0 composite verdict** read per
   [`robustness-decision-rule-2026-05-30.md`](../dev-notes/robustness-decision-rule-2026-05-30.md)
   § 0 (weakest-link), at the realistic fee, against the **dollar-neutral ≈0 null**.

---

## Open design questions for the architect (M-T1)

> Framed, not resolved. The deferred carry framing (b) (D-CARRY.0) and v0.1.0 Q-BR-2
> framing (b) are the precedents — the architect lifts the design intent and resolves
> the engine specifics.

- **Q-MN-1 — the short-leg engine: gated branch in `run_path` (i) vs sibling
  `run_path_long_short` (ii)?** Analyst leans (i) — one engine, the anchor-neutrality
  proof is "`k_short = 0` is the unchanged code", avoids duplicating solvency logic.
  Resolve the short-side solvency/margin model (a short's loss is unbounded above —
  the long-only Bug-B cap needs a short analogue).
- **Q-MN-2 — the run_path anchor-neutrality (LOAD-BEARING).** How is the 107-anchor
  byte-identity proven when `run_path` is touched for the first time? (The `k_short = 0`
  path must be byte-identical.) This is the load-bearing risk — the architect designs it
  out, exactly as v0.1.0/carry kept `run_path` untouched.
- **Q-MN-3 — the second sidecar.** v0.1.0 reused the `funding_by_symbol` channel because
  basis + funding were mutually exclusive. v0.2.0 needs BOTH live (the short pays
  funding, the signal is basis) → the sibling `basis_by_symbol` field D-BR.3 deferred is
  now owed. Confirm the +1 `Option` field on `GeneratedPath`/`BlockBootstrapPathGen`/
  `TcnScenarioInput` and its anchor-neutrality (`None` for every non-MN run).
- **Q-MN-4 — the basis⊥funding residualization (R-MN.6).** How is `basis − β̂·funding`
  computed Decimal-exact? Options: (a) a Decimal OLS cross-sectional slope per rebalance;
  (b) a rank-based residual (rank basis, rank funding, residual of the ranks — avoids
  float regression and matches the rank-IC the spike measured). Analyst leans (b) — it
  is Decimal-clean and matches the cross-sectional rank channel the signal lives in.
- **Q-MN-5 — the θ-grid + arms + fee cross-product + N.** Lock the lookback band ({60,168}
  proposed, L=24 optional), the K split, the {0,5}bps fee read (NOT the full ladder —
  justified by the v0.1.0 fee-invariance), the three arms, and N. Confirm the wall-clock
  for the `|arms| × |L| × |K| × |fee| × N × 2 regimes` cross-product is tractable (the
  v0.1.0 D-BR.WALLCLOCK gate — up to 12 surfaces).

---

## Assumptions & limits (challengeable by operator / architect)

1. **The benchmark change (BH → dollar-neutral ≈0) is the whole game, and it is a
   CORRECTION, not a goalpost move.** A beta-neutral book's null is cash. The bands are
   frozen; only the control changes. (Fork note § 2.3.)
2. **The spread's gross edge is unproven and may be small.** The −0.10 IC is the gross
   ceiling. The bet is structural (beta-strip + the ≈0 null), not a promised Sharpe.
   Deliberately NO manufactured target Sharpe (the fabricated-"1.40" precedent). k1 kills
   it if FRAGILE at 0bps gross on the ≈0 null.
3. **The funding-confound (k2) could close the domain** — the +0.47/+0.66 overlap is
   real; the basis⊥funding arm (R-MN.6) is the resolver and is a likely-enough negative
   to be a named kill-criterion.
4. **`run_path` is touched for the first time (R-MN.8) — the load-bearing anchor risk.**
   The build is ~5-8 dev-days (vs v0.1.0's ~3-5d) precisely because of the short-side
   engine + the run_path anchor re-proof, NOT because the cost model is hard (it already
   exists — R-MN.3).
5. **The robustness axis judges resampled real 2023/2024 only** (inherited scope limit).
6. **On-chain is deferred, not abandoned.** If the spread is FRAGILE, on-chain is the
   pre-registered next domain, entered with the derivatives-positioning family
   retired-with-finality. (Fork note § 4-5.)

---

## Changelog

- 2026-06-06 (analyst, perp-basis-mn-spread): authored the v0.2.0 feature brief +
  opened the trace row `REQ-PERP-BASIS-MN-SPREAD-001` (state `proposed`). Follow-on to
  `perp-basis-signal-robustness` v0.1.0 (closed PASS / FAMILY-UNIFORM-FRAGILE at all
  fees incl. 0bps gross). Why = the v0.1.0 verdict is a VEHICLE verdict, not a signal
  verdict (the long-only arm carries full market beta + captures only the long-low-basis
  leg, benchmarked against BH +1.74; fees falsified as the killer — p50 moves ~0.002
  Sharpe across the {0,2,5,10}bps ladder). The market-neutral spread strips beta (null
  → ≈0, removing the 35× hurdle) + captures both legs (the spike's full −0.10 IC).
  Requirements R-MN.1..9 + the LOAD-BEARING short-leg engine (R-MN.2), short-leg funding
  cost (R-MN.3 — the model ALREADY EXISTS at montecarlo.rs:325-363, only line 350's
  `continue` skip gates it), and the FUNDING-ORTHOGONALIZATION arm (R-MN.6 — three arms
  basis/funding/basis⊥funding on the same paths, the confound resolver + headline).
  ONE frozen-rule change: BH control → dollar-neutral ≈0 null (a correction, not a
  goalpost move). Pre-registered H0/H1 + k1/k2/k3 kill-criteria. Build ~5-8 dev-days
  (the short-side engine + the FIRST run_path anchor-neutrality re-proof for the 107 are
  the bulk + the dominant risk, NOT the cost model). 5 framed design questions Q-MN-1..5
  for the architect M-T1. Carry framing (b) / v0.1.0 Q-BR-2 framing (b) are the deferred
  precedents. No Design section, no tasks.md, no code authored by the analyst. Full
  adjudication: [basis-reversal-vehicle-vs-signal-fork-2026-06-06.md](../dev-notes/basis-reversal-vehicle-vs-signal-fork-2026-06-06.md).
