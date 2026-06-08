---
slug: perp-basis-mn-spread
version: 0.2.0
status: arch-done
owner: architect → developer
priority: P1
predecessor: perp-basis-signal-robustness v0.1.0
updated: 2026-06-07
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

> **Architect M-T1, 2026-06-07.** Q-MN-1..5 resolved + justified below. The
> load-bearing decision record is [ADR-0051 § D6.10](../architecture/adr/0051-monte-carlo-determinism-and-distribution-report-anchoring.md#d610--the-short-side-engine-the-first-run_path-touch--a-second-simultaneous-sidecar-market-neutral-basis-spread-amendment-2026-06-07)
> (the FIRST `run_path` touch since C2; the 6th anchor-additive instance). The
> staged developer breakdown is [tasks.md](tasks.md) (M-DEV-0..N + M-TEST). Every
> design decision below is grounded in the code seams inspected this session; no
> paraphrase trusted. Tags **D-MN.\*** map to the requirements R-MN.\* and to the
> ADR D6.10.\* clauses.

### The dominant constraint — anchor-neutrality (the whole risk, designed out)

This is the **FIRST `run_path` touch since v0.1.0**. There are **107 byte-immutable
regression anchors** (`spec/anchors.toml`, confirmed 107 on disk this session) that
MUST stay byte-identical (`bash scripts/verify_anchors.sh` → 107/107 at every stage).
The non-negotiable rules, inherited from ADR-0051 + the v0.1.0 precedent and re-stated
for the `run_path` core:

1. **`run_path` stays CONCRETE.** It keeps its `strategy: strategy::MomentumStrategy`
   signature (`montecarlo.rs:92`). NO `dyn`, NO generics, NO trait-object dispatch —
   the § D6.5.2 trap that would re-touch both production call-sites
   (`monte_carlo.rs:878`, `param_robustness_sweep.rs:2725`) and risk all 107 anchors.
2. **Every new seam is additive / defaults-OFF.** The short-side path is gated on
   `k_short > 0`; the default (`k_short == 0` / long-only / no-short) path reduces to
   today's EXACT executed code. New struct fields (`basis_by_symbol`, `basis_override`,
   `basis_at_return`) default `None`; new enum variants (`SelectionMode::LongShort`)
   are non-default.
3. **New sidecars are additive; the existing co-resample + as-of-join are REUSED.**
   The second `basis_at_return` channel is built with the SAME `idx_seq` gather + the
   SAME `basis_as_of` join that v0.1.0 already shipped — no edit to the existing
   `funding_by_symbol` channel.
4. **Neutrality is proven by construction, then gated mechanically.** M-DEV-0 records
   the 107 floor FIRST; a `run_path` k_short=0 byte-identity unit test + the 107/107
   `verify_anchors.sh` gate run after EVERY seam.

### D-MN.1 — Signal: the v0.1.0 basis-reversal score, reused verbatim (R-MN.1)

`basis_reversal_score = −trailing_mean(basis)` over L bars, cross-sectional rank, is
**reused byte-for-byte** from v0.1.0 (`momentum.rs:326`). The load-bearing minus (the
R-BR.2 / R-MN.1 sign — tilt AGAINST the basis) stays in ONE place. The v0.1.0
sign-assertion falsifier carries over unchanged and stays GREEN. **The only change is
selection** (D-MN.2): v0.1.0 longs the top-K lowest-basis; v0.2.0 ALSO shorts the
bottom-K highest-basis. No new score code.

### D-MN.2 — Q-MN-1: the short-side engine = a `k_short`-gated branch in `run_path` (LOAD-BEARING)

**Decision: option (i) — a `k_short`-gated branch inside the EXISTING `run_path`.**
Option (ii) (a sibling `run_path_long_short`) is REJECTED. The anchor-neutrality
argument is front and centre and decisive:

- **Why (i).** It keeps ONE engine, and the anchor-neutrality proof becomes literal
  and trivial: *"the `k_short == 0` path is the unchanged code — every short branch is
  dead code when no shorts are requested"* (D-MN.3). `run_path` keeps its concrete
  `MomentumStrategy` signature, so neither production call-site changes (the §D6.5.2
  trap is avoided). The blast radius is the smallest possible.
- **Why NOT (ii).** A sibling `run_path_long_short` would (a) **duplicate the entire
  solvency / equity / drawdown / accrual machinery** — a correctness risk (two copies
  drift over time), and (b) add a 3rd `run_path` call-site to the dispatch fork. It
  buys separation the feature does not yet need. YAGNI + the D6.5.2 discipline both
  point to (i). The analyst lean is ratified.

**The threading.** A single read-only `k_short: u32` is read from the caller-supplied
strategy (the `MomentumStrategy` already carries `k_short` — `momentum.rs:35`). No new
`run_path` parameter; the gate is `if k_short > 0`.

**The short-side solvency / margin model (Q-MN-1 second half — the new correctness
surface).** A long's loss is bounded (price → 0); a **short perp's loss is unbounded
above** (price → ∞), so the long-only Bug-B cash-cap (`montecarlo.rs:232`, "never
spend more than cash on hand") has NO short analogue and a naive short would drive
equity negative on an adverse move. The model has three deterministic pieces:

1. **Notional sizing — symmetric + dollar-neutral.** Each leg books the EXISTING fixed
   fraction (`dec!(0.10)` of equity per name, `montecarlo.rs:220`). With K longs + K
   shorts the gross book is `2·K·0.10·equity` and the **net dollar exposure is ≈0**
   (Σ long notional − Σ short notional, asserted ≈0 by the R-MN.7 #1 falsifier). NO
   1/N rescale — that is an engine edit and breaks apples-to-apples with the long-only
   arm (the D6.7.2 precedent).
2. **A short INITIAL-MARGIN gate — the mirror of the long Bug-B cap.** Opening a
   `Side::Sell` leg reserves `margin = notional / max_leverage` against cash, where
   `max_leverage = dec!(1)` is a LOCKED Decimal constant for v0.2.0 (fully-collateralized
   shorts — the conservative + simplest choice; > 1 leverage is a future ADR). The
   short is SKIPPED (not partially filled) if cash cannot cover the reserved margin +
   the estimated fee — the exact structure of the long Bug-B skip, so the two solvency
   paths are visibly symmetric.
3. **A MAINTENANCE-MARGIN LIQUIDATION rule — bounds the unbounded loss (the new
   mechanism).** At each per-bar mark-to-market (the existing equity-curve update at
   `montecarlo.rs:376`), if total equity falls below `maintenance_margin_frac ·
   gross_short_notional` (LOCKED `maintenance_margin_frac = dec!(0.5)` for v0.2.0 — a
   conservative half-notional maintenance floor), **all short legs are force-closed at
   the current mark** (a deterministic buy-to-cover at `mark_prices[sym]`, the same
   fill path as a normal close), and a `liquidations` counter increments (surfaced in
   `PathRunResult` for report legibility). This bounds the short loss exactly as a real
   exchange's liquidation engine does, deterministically (no RNG, ordered `BTreeMap`
   iteration). Both constants are **hashed body fields** of the MN anchor (K3).

**The accounting (R-MN.3, mark-to-market on the short leg).** The short P&L is the
mirror of the long path, and the equity math needs NO change:

- **Open short:** `cash += notional − fee` (sell proceeds in); `position_book[sym] -=
  qty` (qty goes negative).
- **Close / cover:** `cash -= notional + fee` (buy-to-cover out).
- **Mark-to-market:** the existing `position_value = Σ qty·mark` (`montecarlo.rs:377`)
  **already handles `qty < 0` correctly** — a negative position contributes negative
  value, so equity FALLS when the short moves against the book. Confirmed by reading
  the equity-tail; only the order-OPENING branch (a `Side::Sell` open, gated
  `k_short > 0`) and the liquidation cap are new code.
- **The short-leg funding cost (R-MN.3 — the binding cost; the model ALREADY EXISTS).**
  `montecarlo.rs:322-373` accrues `cash += notional × (−funding_rate)` at every 8h
  settlement boundary. Line 350's `continue; // no short legs in framing (a)` is the
  ONLY gate. **Confirmed this session:** for a short (`qty < 0`, `notional = qty·mark <
  0`), the existing formula `notional × (−rate)` *already produces the correct cost* —
  a short on a positive-funding name yields `(negative notional) × (−positive rate) =
  negative cashflow = a cost`. The change is to REPLACE the `qty <= 0 → continue` skip
  with a branch that accrues for held shorts too (still gated on `funding_override`
  being `Some`, so non-MN runs are byte-identical). The cost model is correct as
  written; only the skip gates it.

### D-MN.3 — Q-MN-2: the `run_path` anchor-neutrality re-proof (THE load-bearing gate)

The 107-anchor byte-identity is proven in **three layers**, designed out explicitly
(this is the risk D6.5/6/7/8/9 never carried):

1. **By construction.** Every new statement in `run_path` is inside `if k_short > 0
   { … }` OR is reached only when `position_book` holds a `qty < 0` (impossible unless
   a short was opened, which requires `k_short > 0`). Concretely:
   - The short-open is a NEW match arm: `SignalKind::Sell if current_qty <= 0 &&
     k_short > 0 => { /* open short */ }`. It does NOT alter the existing
     `SignalKind::Buy` open or the `SignalKind::Sell if current_qty > 0` close arms
     (which stay byte-identical).
   - The funding accrual's `if qty <= Decimal::ZERO { continue; }` (`montecarlo.rs:349`)
     is replaced by a branch that accrues for shorts only when a short is held — and
     when `k_short == 0` no short is ever held, so the `continue` is taken every time,
     exactly as today.
   - The liquidation check is `if k_short > 0 && <maintenance breach> { … }`; inert
     when `k_short == 0`.
   - **When `k_short == 0`, every short branch is provably dead code, and the executed
     path is byte-for-byte HEAD's `run_path`.**
2. **The test (M-DEV-0 floor + a `run_path` k_short=0 byte-identity unit test).** Before
   any change, `bash scripts/verify_anchors.sh` → **107/107** is recorded as the floor
   (M-DEV-0, runs FIRST). A NEW unit test `run_path_k_short_zero_byte_identical_to_head`
   (in `montecarlo.rs` tests, mirroring the existing `funding_override_none`
   neutrality test at `montecarlo.rs:648`) asserts `run_path` on a fixed synthetic path
   with a `k_short == 0` strategy produces an equity curve bit-identical to the same
   path with the short-side code compiled but never entered — it goes RED the instant
   any short statement leaks out of its gate.
3. **The hard gate at every seam.** `verify_anchors.sh` → **107/107** is the
   non-negotiable gate after EVERY task (M-DEV-0..N), exactly as v0.1.0 gated 99/99.
   Any drop → STOP, the seam is not anchor-neutral. The 107 are ADDED to, never
   substituted.

### D-MN.4 — Q-MN-3: the second simultaneous sidecar `basis_by_symbol` (R-MN.3, R-MN.5)

v0.1.0 reused the single `funding_by_symbol` channel because basis + funding were
mutually exclusive (D6.9.2). v0.2.0 needs **BOTH live simultaneously** — the signal is
BASIS (selection), the short leg PAYS FUNDING (accrual). So the sibling
`basis_by_symbol` field that D6.9.2 explicitly deferred is now **owed and added**,
riding the SAME shared-index machinery D6.6 generalized:

- **`GeneratedPath`** (`crates/data/src/synth/mod.rs:54`) gains a NEW
  `basis_by_symbol: Option<Vec<Vec<Option<Decimal>>>>` field (defaults `None`) — the
  exact twin of `funding_by_symbol`.
- **`BlockBootstrapPathGen`** (`crates/data/src/synth/bootstrap.rs:71`) gains a NEW
  `basis_at_return: Option<Vec<Vec<Option<Decimal>>>>` field + a `with_basis(…)`
  builder (the twin of `with_funding`). The co-resample loop (`bootstrap.rs:332-380`)
  gathers `basis_at_return[s][idx_seq[k]]` at the **same `idx_seq`** that picks the
  return AND the funding → price ↔ funding ↔ basis all move contemporaneously. This is
  the three-series shared-index extension D6.6.5 explicitly anticipated. **ZERO new RNG
  draws** — `idx_seq` is materialized once; the basis gather is a second read of it (the
  D6.6.1 de-risk transfers verbatim).
- **`TcnScenarioInput`** (`crates/backtest/src/cli_types.rs:502`) gains a NEW
  `basis_override: Option<BTreeMap<(Symbol, Timestamp), Decimal>>` field (the twin of
  `funding_override`). For the MN arm BOTH are `Some`: the basis map is injected into
  the strategy via `with_funding` for the SCORE (the basis rides the strategy's
  `funding_map` score channel exactly as v0.1.0), and the funding map is passed as
  `funding_override` for the short-leg ACCRUAL.
- **The as-of join + loader are REUSED.** `basis_as_of` + `build_basis_at_return`
  (`basis_data.rs:357`/`:411`, shipped in v0.1.0) build the `basis_at_return` array;
  `BasisDataSource` (pin `aa72409a…`) loads it. The funding side (`funding_data.rs`,
  pin `bf1ede44…`) is unchanged. Both inherit their proven no-look-ahead falsifiers
  (R-MN.5); the two-sidecar simultaneity must not introduce a leak in either, asserted
  by the inherited falsifiers staying GREEN under simultaneous threading.
- **Anchor-neutrality.** `basis_by_symbol = None` and `basis_override = None` for EVERY
  non-MN run (all 31 `TcnScenarioInput` literals + all 6 `GeneratedPath` literals get
  the new field defaulted `None` — mechanical, additive, ~37 sites, no behavior
  change). When the basis source is absent the generator emits `None` and takes the
  existing reconstruction path byte-identically (the D6.6.2 argument applied to the
  second field). The 107 hold by construction.
- **Why a real second field now (vs the D6.9 reuse).** With both series live,
  overloading the one channel is impossible (it would carry two distinct values per
  (sym, ts)). A clean parallel field is the smallest correct change; it also resolves
  the naming-debt D6.9 flagged (the channel was "funding-specific only in name").

### D-MN.5 — the dollar-neutral selection: `SelectionMode::LongShort` + un-gated `k_short` (R-MN.2)

A NEW `SelectionMode::LongShort` variant (`config.rs:92`; serde-default stays
`CrossSectionalTopK` → the 107 anchors' serialization is byte-identical, the D6.7
precedent). Under `LongShort`, `build_rebalance_signals` (`momentum.rs:215`) selects:

- **the top-K by score → the LONG book** (the existing `top_k_long`, reused), AND
- **the bottom-K by score → the SHORT book** via a NEW `bottom_k_short` selector — the
  `top_k_long` mirror over the SAME `scores` map, taking the K LOWEST scores (with the
  load-bearing `−mean` basis sign, lowest-score = highest-basis = the crowded-long
  names the spread shorts). `bottom_k_short` is a deterministic `BTreeMap`-ordered pure
  fn (alphabetical tie-break, the `top_k_long` twin) → two-run byte-identity by
  construction (the D6.7.5 precedent).

The `config.rs:308` hard-reject of `k_short > 0` (`UnsupportedShortSizing`) is GATED:
`k_short > 0` is permitted ONLY when `selection_mode == LongShort` (a `k_short > 0`
under `CrossSectionalTopK` / `TimeSeriesLongFlat` still rejects — those modes have no
short semantics, so the existing error stays for them). The short signals are emitted
as `SignalKind::Sell` with a NEW evidence tag distinguishing "open short" from "close
long", so `run_path` forks the `Sell` arm on `current_qty` (`> 0` → close long
(existing, byte-identical); `<= 0 && k_short > 0` → open/extend short (new)).

### D-MN.6 — Q-MN-4: the basis⊥funding residualization — RANK-BASED, Decimal-exact (R-MN.6, R-MN.9)

**Decision: option (b) — a rank-based residual, NOT a Decimal OLS slope.** The 3-arm
headline needs a basis-orthogonalized-to-funding arm; the residual `basis − β̂·funding`
is computed:

- **The mechanism.** At each rebalance, over the warmed cross-section: rank the basis
  scores (1..N), rank the funding scores (1..N), and the residual score =
  `rank(basis) − rank(funding)` (an integer-valued Decimal). Long = highest residual
  (low-basis-relative-to-its-funding), short = lowest residual. This is the
  Spearman-style residual that matches the rank-IC channel the spike actually measured
  (the −0.10 is a *rank* IC).
- **Why rank-based, not OLS (the exactness + determinism argument — the decisive one).**
  A Decimal OLS slope `β̂ = Σ(x−x̄)(y−ȳ) / Σ(x−x̄)²` requires a **Decimal division that
  does NOT terminate in general** (e.g. `1/3`), forcing a rounding-mode + scale choice
  that is a hidden determinism surface and a cross-platform-drift risk (ADR-0003 + the
  ADR-0051 D2 f64-boundary discipline both forbid non-exact Decimal division in the
  hashed path). The rank residual is **pure integer arithmetic over Decimals** — ranks
  are exact integers, the subtraction is exact, NO division, NO rounding, NO float. It
  is Decimal-exact and bit-reproducible by construction, AND it is the correct statistic
  (the signal lives in the rank channel). Ties in either rank use the alphabetical
  `BTreeMap` order (the existing tie-break) → deterministic. The OLS slope is REJECTED
  on exactness grounds; rank-residual is the durable choice. The analyst lean (b) is
  ratified.
- The residualization is a NEW pure fn in the strategy/sweep layer; it reads the two
  sidecar maps (basis + funding) already threaded for the MN arm. The R-MN.7 #7
  falsifier asserts the basis⊥funding arm produces a DIFFERENT result from the raw
  basis-spread (the residualization is load-bearing, not a no-op).

### D-MN.7 — the day-1 falsifier set (R-MN.7 — each RED-on-revert, the CLAUDE.md non-negotiable)

The market-neutral arm is a sizing/selection modifier → it ships a **day-1
baseline-equity-divergence e2e** from the start, plus the v0.1.0 6-falsifier pattern,
plus a market-neutral-specific assertion. All seven, each GREEN-as-written AND
RED-on-revert (the genuine-guard proof), in a new `crates/backtest/tests/
mn_spread_divergence_e2e.rs` (mirroring `carry_divergence_e2e.rs` /
`basis_divergence_e2e.rs`):

1. **Dollar-neutrality (the MN-specific guard, R-MN.7 #1 — the beta-leak guard).**
   Σnotional ≈ 0 (long notional ≈ short notional) at every rebalance; the MN book's
   net dollar exposure ≈ 0. RED if the book carries net directional exposure (a naive
   non-dollar-neutral long/short re-introduces the beta this feature exists to strip).
2. **Short-leg funding-cost non-no-op (R-MN.7 #2, the carry R-CARRY.10b analogue).**
   Zero the short-leg accrual → assert equity diverges; RED on revert (guards against a
   computed-and-ignored cost — the binding cost must be load-bearing).
3. **Baseline-equity-divergence e2e (R-MN.7 #3 — the CLAUDE.md non-negotiable).** The
   MN arm's equity diverges from the un-tilted baseline (a `VolAdjustedReturn` /
   equal-weight long-only run on the SAME path) by ≥ 1 bp when the basis decision
   variable is non-trivial. Pattern: `crates/strategy/tests/
   vol_targeting_overlay_end_to_end.rs`. PLUS the MN-beta-strip assertion: the MN book's
   equity is beta-stripped vs both the long-only baseline AND a passive buy-and-hold (its
   return correlation to the market leg is ≈0, not ≈1) — the structural claim of the
   whole feature, tested directly.
4. **Sign-assertion (inherited R-MN.1 / R-BR.2).** A sign flip = a basis-momentum payer
   (it would long the crowded-high-basis names and short the cheap ones) → RED.
5. **No-look-ahead (R-MN.7 #5).** Both the basis + the funding joins past-only under
   simultaneous threading; RED on a future shift of EITHER series.
6. **Two-run byte-identity (R-MN.7 #6).** Same `ensemble_seed` → identical body-SHA
   (catches any unordered fold in the second co-resample, the `bottom_k_short`
   selection, the liquidation rule, or the renderer).
7. **The basis⊥funding orthogonalization is non-no-op (R-MN.7 #7).** The basis⊥funding
   arm produces a DIFFERENT result from the raw basis-spread arm on the same paths
   (proving the rank-residualization is load-bearing). RED if the residual collapses to
   the raw basis.

### D-MN.8 — Q-MN-5: the θ × arms × fee × regime cross-product + wall-clock (the LOCKED plan)

The primary anchored deliverable is a **three-arm × θ × fee surface** vs the
dollar-neutral ≈0 null (NOT buy-and-hold). LOCKED axes (hashed body fields, K3):

| Axis | LOCKED value | Rationale |
|---|---|---|
| **arms** | {basis-spread, funding-spread, basis⊥funding} | the R-MN.6 headline; 3 arms on the SAME paths |
| **θ (lookback L)** | `{60, 168}` bars | the IC-peak band; **L=24 DROPPED** (for a spread the turnover lever matters less since D6.9 falsified fees as the killer, IC peaks at 60–168); **L=720 SKIPPED** (noise, inherited) |
| **K split** | `K_long = K_short = 3` | a single LOCKED dollar-neutral split for v0.2.0; rebalance = the existing 8h default |
| **fee** | `{0, 5}` bps | the R-BR.LOAD minimum — D6.9's fee-sweep falsified fee-bleed (p50 ~0.002 Sharpe across the full ladder), so the full {0,2,5,10} ladder is low-value; {0,5}bps = gross-ceiling + realistic-decision read. Reuses the D6.9.3 `--taker-fee-bps`/`--slippage-bps` flags — NO new fee plumbing |
| **regime** | `{2023, 2024}` | both day 1 (the carry/horizon E1 precedent) |
| **N** | `200`/cell | the carry/MR/TS/v0.1.0 tractable shape |

**Surface count + wall-clock (the D-BR.WALLCLOCK / Q-MN-5 gate — tractable).**

- Surface count = `|arms|=3 × |fee|=2 × |regime|=2 = 12 surfaces`. Each surface =
  `|L|=2 θ-cells × |K|=1 × N=200 = 400 path-runs`. Total = **12 × 400 = 4,800
  path-runs**.
- Per-path cost: the carry/basis precedent measured **~0.094 s/path** at the N=3 smoke
  (D-BR.WALLCLOCK). The long/short book has ~2× the legs, but per-path cost is
  dominated by the (unchanged) bootstrap generation, so conservatively **≤ ~0.15
  s/path** for the 2-leg book. Per surface ≈ `400 × 0.15 ≈ 60 s`; **all 12 ≈ 12 min** —
  TRACTABLE, comfortably under the ≲30-min program gate.
- **Mandatory C3-lesson pre-flight:** the developer re-confirms the per-path cost at the
  N=3 smoke BEFORE the full run (M-DEV-8). If a material per-path regression shows, the
  documented economy is to drop to the **0bps-only gross read = 6 surfaces ≈ 6 min**
  (flagged) — the gross-ceiling that gates the rest.
- **Anchors:** + up to **6 anchors** (3 arms × {0,5}bps on 2023; +6 with 2024 → 12
  potential). The tester locks them at M-TEST PASS; the **minimum is the 3-arm × 0bps ×
  2023 = 3** (the gross-ceiling read that gates the rest). Scenario name:
  `v2-mn-{arm}-fee{NN}bps-theta-surface-{year}-block-bootstrap-real-fy` (`{arm}` ∈
  `{basis, funding, basisperp}`; `{NN}` ∈ `{00, 05}`). Both revision SHAs (basis
  `aa72409a…` + funding `bf1ede44…`) appear in the MN body (the D6.6.4 dual-pin
  precedent).

### D-MN.9 — namespace + anchoring (R-MN.8)

A NEW `perp-basis-mn-spread` anchor namespace (the market-neutral spread is a distinct
experiment axis from the long-only arm — the D6.8/D6.9 new-namespace precedent), with
`verify_anchors.sh` extended by an additive `elif` branch **after** the
`perp-basis-signal-robustness` branch (line 170; touch NO existing branch → the 107
resolve through their existing branches byte-identically). Reports under
`spec/perp-basis-mn-spread/reports/` using `robustness-*-<scenario>.md` naming. The
anchor unit = the three-arm MN θ × fee surfaces; the tester locks them at M-TEST PASS.
**Anchored report files in `spec/*/reports/` remain byte-immutable** (ADR-0038 § D6).

### Reconciliation with the pre-registered brief (assumptions changed)

The design changes / sharpens the following pre-registered items (all minor; flagged
per the mandate):

1. **Q-MN-5 θ-grid — L=24 DROPPED (the analyst's proposal, RATIFIED).** The brief's
   Backtest-Scenarios section proposed dropping L=24 and the architect "may keep L=24
   for continuity." The architect **drops it** — for a spread the turnover lever
   matters less (D6.9 falsified fees as the killer) and the IC peaks at 60–168. This
   shrinks each surface to 2 θ-cells (vs the v0.1.0 6-cell grid).
2. **The K split is a SINGLE locked value `K=3` (the brief left "K ∈ a small locked
   set" open).** The architect locks `K_long = K_short = 3` for v0.2.0 — one
   dollar-neutral split keeps the surface count + wall-clock tight and avoids a
   premature K-sweep before the spread has shown a net edge. A K-sweep is a clean
   follow-on if v0.2.0 is ROBUST.
3. **The margin model adds two LOCKED constants the brief did not specify
   (`max_leverage = dec!(1)`, `maintenance_margin_frac = dec!(0.5)`).** The brief
   required "a short analogue" to the Bug-B cap but left the model to the architect.
   These constants are the conservative fully-collateralized choice and are hashed body
   fields (a different margin model is a different surface). Flagged as a new
   pre-registered assumption for the tester to read the verdict against.
4. **The basis⊥funding arm is RANK-BASED (the analyst's lean (b), RATIFIED on exactness
   grounds).** No change to intent; the architect confirms the rank residual is
   Decimal-exact and the OLS slope is rejected (Q-MN-4).

All other pre-registered items (the frozen § 0 rule with the BH→dollar-neutral-null
correction, H0/H1, k1/k2/k3, the fee axis {0,5}, N=200, the universe + pins, the
3-arm headline) are carried verbatim.

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

- 2026-06-07 (architect, perp-basis-mn-spread M-T1): filled the `## Design` section
  (D-MN.0..9), authored [tasks.md](tasks.md) (M-DEV-0..10 + M-TEST), and wrote the
  load-bearing decision record [ADR-0051 § D6.10](../architecture/adr/0051-monte-carlo-determinism-and-distribution-report-anchoring.md)
  (the FIRST `run_path` touch since C2; the 6th anchor-additive instance; registered
  atomically in the ADR README). **Q-MN-1..5 RESOLVED:** (Q-MN-1) the short-side engine
  = a `k_short`-GATED BRANCH inside the EXISTING `run_path` (keeps the CONCRETE
  `MomentumStrategy` signature — NO dyn/generic, the §D6.5.2 trap avoided; sibling
  `run_path_long_short` REJECTED — duplicates solvency logic); the short SOLVENCY model
  = symmetric dollar-neutral notional + a short INITIAL-MARGIN gate (`max_leverage =
  dec!(1)` LOCKED) + a deterministic MAINTENANCE-MARGIN LIQUIDATION rule
  (`maintenance_margin_frac = dec!(0.5)` LOCKED) bounding the unbounded short loss; the
  equity math is UNCHANGED (`Σ qty·mark` already handles `qty<0`); the short-leg funding
  accrual ALREADY EXISTS (`montecarlo.rs:322-373`, `cash += notional × (−rate)` already
  correct for a short — only line 350's `continue` skip gates it). (Q-MN-2, THE
  load-bearing gate) the `run_path` anchor-neutrality RE-PROOF = by-construction (short
  branches are dead code when `k_short==0` → executed path byte-for-byte = HEAD) + a
  `run_path_k_short_zero_byte_identical_to_head` unit test + the hard `verify_anchors.sh`
  → 107/107 gate run FIRST (M-DEV-0 floor) + after EVERY seam. (Q-MN-3) a SECOND
  SIMULTANEOUS sidecar (the D6.9 channel-reuse RETIRED — basis + funding BOTH live):
  `GeneratedPath.basis_by_symbol` + `BlockBootstrapPathGen.basis_at_return` +
  `with_basis(…)` + `TcnScenarioInput.basis_override`, co-resampled at the SAME `idx_seq`,
  ZERO new RNG, `None` for every non-MN run (~37 default-None sites). (Q-MN-4) the
  basis⊥funding arm is RANK-BASED Decimal-EXACT (`rank(basis) − rank(funding)`, NO
  division → bit-reproducible; OLS slope REJECTED — non-terminating Decimal division is a
  hidden determinism surface). (Q-MN-5) a NEW namespace `perp-basis-mn-spread`; 3 arms ×
  {0,5}bps × {2023,2024} = 12 surfaces × 400 paths ≈ ~12 min TRACTABLE; θ L∈{60,168}
  (L=24 DROPPED, L=720 SKIPPED), K=3 LOCKED. **Assumptions changed (flagged):** L=24
  dropped, K locked to a single value 3, two LOCKED margin constants added, the
  basis⊥funding arm locked rank-based. Build estimate VALIDATED at ~5–8 dev-days (10 dev
  tasks + M-TEST); the dominant risk is the short-side engine + the FIRST run_path anchor
  re-proof, NOT the cost model. HANDOFF → developer. No code authored; files only.
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

---

## Implementation

**Stage 2 completed 2026-06-08 (M-DEV-4..10). Developer: claude-sonnet-4-6 / Stage 1 at HEAD 18334c9.**

### What was built

**M-DEV-4 — `ScoreSource::BasisFundingResidual`**
- `crates/strategy/src/cross_sectional/config.rs`: added `BasisFundingResidual` variant to `ScoreSource`; serde default stays `VolAdjustedReturn` → anchor-neutral.
- `crates/strategy/src/cross_sectional/selector.rs`: added `rank_residual` pure fn computing integer-Decimal `rank(basis) - rank(funding)` with NO division; BTreeMap alphabetical tie-break (deterministic).
- `crates/strategy/src/cross_sectional/momentum.rs`: `compute_scores_for_symbol` dispatches to `rank_residual` under the new variant; both sidecar maps (basis + funding) read.
- Tests: 60 strategy unit tests green; 107/107 anchors.

**M-DEV-5 — Sweep harness for MN arms**
- `crates/backtest/src/bin/param_robustness_sweep.rs`:
  - `SweepScoreSource` MN variants: `MnBasisSpread`, `MnFundingSpread`, `MnBasisFundingResidual`
  - `MN_TIER1_GRID`: 2 cells (L∈{60,168} bars, k_long=k_short=3, rebalance=480m)
  - `GridKind::MnTier1` + `grid_for_kind` arm
  - `mn_grid_def_string` fn: hashed body grid field including `k_short`, `max_leverage`, `maintenance_margin_frac`
  - `load_mn_path_gen`: loads both basis (SHA-pinned) and funding (SHA-pinned) datasets, builds dual-sidecar `BlockBootstrapPathGen` with `.with_basis()` and `.with_funding()`
  - `IndexedPathMetrics::liquidations` + `CellResult::total_liquidations` fields
  - `render_surface_report` MN branch: slug=`perp-basis-mn-spread`, MN table header with k_short + liquidations columns, FRAGILE/MARGINAL/ROBUST verdict, family conclusion
  - Scenario naming: `v2-mn-{arm}-fee{NN}bps-theta-surface-{year}-block-bootstrap-real-fy`
  - Out-dir routing: `spec/perp-basis-mn-spread/reports/`

**M-DEV-6/7 — 7 day-1 falsifiers in `crates/backtest/tests/mn_spread_divergence_e2e.rs`**
1. `mn_baseline_equity_divergence` — MN LongShort ≠ long-only by ≥ 1 bp
2. `mn_baseline_divergence_red_on_revert` — two identical long-only → Δ=0 (RED-on-revert proof)
3. `mn_dollar_neutral_approx` — MN equity < long-only when shorting rising name
4. `mn_dollar_neutral_red_on_long_only` — long-only > 100k, MN < 100k with rising universe
5. `mn_sign_assertion_short_leg` — correct vs flipped basis sign → different equity
6. `mn_two_run_identity` — two identical MN runs → identical equity (determinism)
7. `mn_residual_arm_diverges_from_basis_arm` — `BasisFundingResidual` selects different short leg than `BasisReversal`; confirmed via BBUSDT-flat vs AAUSDT-rising universe

All 7 tests: GREEN. Universe design ensures measurable P&L divergence via BTreeMap-ordered deterministic rank computation.

**M-DEV-8 — 12 anchored MN surfaces**

All 12 reports written to `spec/perp-basis-mn-spread/reports/`. All cells FRAGILE (expected first-pass; consistent with v0.1.0 long-only FRAGILE baseline and analyst forecast). Three-arm comparison legible: `mn-basis` (p50≈0.02-0.04 for 2024, negative for 2023), `mn-funding` (same as `mn-basis` — signals correlated), `mn-basisperp` (residual arm; FRAGILE in both years). Short-leg funding cost confirmed non-zero.

**M-DEV-9 — `scripts/verify_anchors.sh` handler**

Added `elif [[ "$version" == "perp-basis-mn-spread" ]]` branch searching `spec/perp-basis-mn-spread/reports/` for `robustness-*-${scenario}.md`. 12 new anchors (#108-#119) registered in `spec/anchors.toml`.

**M-DEV-10 — clippy + fmt clean**

- `mn_spread_divergence_e2e.rs`: replaced 120-line overindented `///` doc comment with concise doc + fixed 3 continuation indentation issues
- `montecarlo.rs`: `#[must_use]` on `maintenance_margin_frac()`
- `momentum.rs`: `let mut` → `let` (spurious mut)

### Gate summary

| Gate | Result |
|------|--------|
| `cargo clippy -p backtest -p strategy -p data --bins --tests -- -D warnings` | CLEAN |
| `cargo fmt --check` | CLEAN |
| `cargo test -p backtest --test mn_spread_divergence_e2e` | 7/7 PASS |
| `cargo test -p strategy --lib cross_sectional` | 60/60 PASS |
| `bash scripts/verify_anchors.sh` | 119/119 PASS |

### Verdict (developer read)

All three MN arms are FRAGILE at the tier-1 grid in both 2023 and 2024. The basis-spread and funding-spread arms yield near-identical surfaces (funding rank mirrors basis rank in this universe — a k2 confound signal per D-MN.8). The residual arm (`mn-basisperp`) diverges from the raw basis arm as confirmed by falsifier #7. The tester should read the frozen §0 verdict vs the dollar-neutral ≈0 null (not BH) and lock the 12 MN anchors at M-TEST PASS.
