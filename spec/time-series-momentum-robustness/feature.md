---
slug: time-series-momentum-robustness
version: 0.1.0
status: proposed
owner: analyst
priority: P2
updated: 2026-06-02
---

# Time-series momentum (per-asset absolute momentum, long/flat) — the FIRST non-cross-sectional method, the program's thesis-closing test — v0.1.0

> **The method fix, not a 4th cross-sectional family.** The robustness program
> has now retired all THREE cross-sectional families on this 10-symbol 1h
> universe: momentum (FAMILY-UNIFORM-FRAGILE, path + parameter), mean-reversion
> (FAMILY-UNIFORM-FRAGILE, parameter), and carry/funding (FAMILY-UNIFORM-FRAGILE
> on BOTH 2023 + 2024) — each dominated end-to-end by passive equal-weight
> buy-and-hold of the same coins (**+1.74 Sharpe 2023 / +1.10 Sharpe 2024**). The
> [universe-vs-method diagnosis](../dev-notes/universe-method-diagnosis-2026-06-02.md)
> (2026-06-02) computed, from the banked OHLCV via the harness's own reader, that
> the **cross-sectional RANKING channel carries ≈ 0 forward information** on this
> universe (rank IC within ±0.07 of zero at every horizon, no stable sign, both
> years) — and the broader-universe spike CONFIRMED this: a more-dispersed 35-name
> mid-cap universe LOWERED common-beta (avg R² 0.715 → 0.598) but did **NOT**
> revive rank IC. **The dead channel is the ranking method, not the basket.**
>
> Time-series (absolute) momentum **removes the ranking channel entirely**: each
> asset is traded long/flat on its OWN trailing-return sign — no cross-sectional
> sort. This is a structurally different method, the single most-cited crypto
> effect not yet examined, and the clean disambiguator the diagnosis pre-scoped
> (§ 4.1 logic table). It is the **first time-series family in the program**.
>
> **The load-bearing question this feature answers (either way is decision-grade):**
> *does removing the ranking channel produce a robust directional edge over passive
> buy-and-hold, or is active trading on this universe dominated end-to-end* — i.e.
> does TS-momentum clear the +1.74 / +1.10 bar where x-sec could not (→ **pivot
> the product to time-series**), or is it ALSO FRAGILE (→ implicates universe +
> horizon, **closes the active-trading thesis on this universe**, and routes to the
> pre-positioned broader-universe / horizon axis)?
>
> **This is the analyst brief** — Why / Requirements / Backtest Scenarios /
> mandatory day-1 falsifiers / framed design questions for the architect. It
> commits NO code, triggers NO engine run, and writes NO Design section, tasks, or
> implementation — those are the architect's next, per the workflow. The 89
> existing anchors MUST hold byte-identical; TS-momentum slots in defaults-off
> exactly as carry did.

---

## 0. Pre-registration & anti-cherry-pick (inherited verbatim, frozen now)

TS-momentum is vetted under the **already-frozen** pre-registered decision rule
([`robustness-decision-rule-2026-05-30.md`](../dev-notes/robustness-decision-rule-2026-05-30.md) § 0)
— the SAME ruler that scored momentum, mean-reversion, AND carry. Nothing about
the rule is re-opened. Three commitments carry over verbatim:

1. **The bands are frozen.** p5 Sharpe ≥ +0.5 ROBUST / **< 0 FRAGILE**; prob-of-loss
   ≤ 15% ROBUST / > 35% FRAGILE; p95 MaxDD ≤ ~50% ROBUST / > ~70% FRAGILE; p50
   Sharpe ≥ 1.0 ROBUST; P(Sharpe>1) ≥ 60% ROBUST. Composite = **worst primary band
   wins** (weakest-link). TS-momentum is scored against these, not the reverse.
2. **Anti-cherry-pick by construction.** The θ-surface reports the FULL surface +
   a family verdict and **crowns no argmax winner** (the FP-C3.5 renderer enforces
   this in code). A non-FRAGILE cell carries a `→ C5 deflation required` flag. A
   grid that picked argmax would inflate the false-ROBUST rate (`1 − 0.95^G`).
3. **Pre-flight void-if-fail.** Every report body must print
   `generator: block-bootstrap-real` AND `bootstrap_mode: shared-index`, else the
   verdict is void (the tail is not a fair adversary otherwise).

**The buy-and-hold control (+1.74 Sharpe 2023, +1.10 Sharpe 2024) is the bar
TS-momentum must clear to matter.** A method that does not beat simply holding the
same coins net of fees on this universe is not worth promoting, however internally
"robust." TS-momentum's honest a-priori edge is that, by going FLAT in
down-trends, it can *avoid* the drawdowns buy-and-hold sits through — that is the
falsifiable claim, and the only structural reason it could clear a bar the three
cross-sectional families could not.

---

## Why

### Why time-series momentum, and why now (the program's pre-scoped method fix)

The robustness program's result is now a uniform negative across all three
**cross-sectional** families:

| Family | Axis result | Killer |
|---|---|---|
| **Momentum** (x-sec top-K winners) | C2 FRAGILE + C3 FAMILY-UNIFORM-FRAGILE (6/6) | turnover / fee-bleed + dead ranking channel |
| **Mean-reversion** (x-sec bottom-K) | C3 FAMILY-UNIFORM-FRAGILE (6/6) | turnover / fee-bleed + dead ranking channel |
| **Carry** (x-sec funding rank) | FAMILY-UNIFORM-FRAGILE on BOTH 2023 + 2024 | funding < price-vol; dead ranking channel |
| **Buy-and-hold** (passive) | p50 **+1.74** (2023) / **+1.10** (2024) | — (this is the bar) |

The [universe-method diagnosis](../dev-notes/universe-method-diagnosis-2026-06-02.md)
explained the *uniformity* with one signal-agnostic fact: **cross-sectional rank
IC ≈ 0 at every horizon, both years** (§ M4). Three different signals (trend,
reverse-trend, funding) all fed the SAME ranking channel, and that channel is
empty. The broader-universe spike (§ S) then ruled out "just broaden the basket"
as a cross-sectional fix: a 35-name mid-cap universe lowered common-beta by ~12
points but rank IC stayed pinned at the noise floor with no stable positive sign.
**The diagnosis's firmed recommendation (§ S.5) is exactly this feature:** proceed
to a TIME-SERIES method that removes the ranking channel, on the ORIGINAL
10-symbol set, with the universe-axis pre-condition CLEARED.

### Why this is the structurally-different bet (not a 4th cross-sectional one)

Time-series (absolute) momentum trades **each asset independently on its own
trailing-return sign**: long if the asset's own trend is positive, flat otherwise.
There is **no cross-sectional sort, no ranking-across-names**. The portfolio is the
equal-weight average of the per-asset long/flat rules. Two structural properties
distinguish it from everything the program has tried:

1. **It does not touch the dead channel.** The rank-IC-≈-0 finding is a property of
   *relative-strength ranking*. A per-asset long/flat rule never ranks names against
   each other — it asks only "is THIS asset trending up right now?" That is a
   different question with a different (and as-yet-unmeasured) information content.
2. **It can go FLAT — the only structural escape from the buy-and-hold drawdown.**
   Buy-and-hold's +1.74/+1.10 comes with the full drawdown of sitting through every
   downtrend (the single-real-path momentum showed 73% MaxDD). A long/flat trend
   rule that exits to cash in sustained downtrends can, *in principle*, harvest the
   same up-drift while sidestepping the worst drawdowns — a higher Sharpe via lower
   denominator. **This is the entire a-priori case for TS-momentum and the
   falsifiable claim** (R-TSM.4, the goes-flat falsifier, makes "it actually exits"
   testable; if it is always-long it is just buy-and-hold with extra fees).

### The honest prior (what would make TS-momentum fragile too) — MEDIUM

TS-momentum is the most robust documented crypto effect, but this is NOT a
guarantee. State the failure modes up front so the verdict is read honestly:

- **Whipsaw / fee-bleed at 1h.** A 1h trend rule on a choppy, 0.63–0.68-correlated
  large-cap basket may flip long↔flat constantly in ranging regimes, paying fees on
  every flip to chase trends that reverse before they pay — the same turnover-killer
  that retired the price families, now in time-series form. The flat/entry threshold
  (the no-trade band, R-TSM.1) is the structural defense; the θ-surface spans it.
- **Late exits eat the drawdown anyway.** A trailing-return trend signal lags the
  turn; by the time the trend goes negative the drawdown is partly taken. If the
  exit lags too much, TS-momentum keeps most of buy-and-hold's drawdown while giving
  up some of its up-capture (fees + whipsaw) → a strictly worse buy-and-hold.
- **The up-drift is the edge, and BH captures it more cheaply.** 2023–2024 were
  net-up years (BH +1.74/+1.10). If TS-momentum is long ~most of the time (because
  the assets trended up most of the time), it largely *replicates* buy-and-hold but
  pays fees and occasionally mistimes the flat — i.e. it is dominated by the free
  passive version. The goes-flat + baseline-divergence falsifiers exist precisely to
  detect this degenerate ≈-BH case.
- **The robustness axis judges resampled real 2023 + 2024 history only** — it cannot
  speak to a regime those years never contained (inherited scope limit,
  decision-rule § 5). A different horizon (daily) is outside what the banked 1h data
  can test.

**If TS-momentum ALSO comes back FAMILY-UNIFORM-FRAGILE, that is the thesis-closing
result and is itself decision-grade** (the § 4.1 "NO" branch): the machine will
have shown that on this 10-symbol 1h universe NO active method — cross-sectional
OR time-series, trend OR reverse-trend OR carry — beats passive holding net of
fees, which strongly implicates the **universe + horizon** as the binding limiter
and routes the next move to the pre-positioned broader-universe / horizon axis
(the `data/binance-broaduni` tree, pin `518b4d40…`, is already banked per the
spike § S.6). The brief does not overclaim a TS edge.

---

## Requirements

### R-TSM.1 — Signal: per-asset ABSOLUTE momentum, LONG/FLAT (the method, stated precisely)

For **each asset independently** (NO cross-sectional ranking):

```text
own_trend(s, t)  = the asset's OWN trailing return / trend over a lookback L
                   (e.g. cumulative log-return over the last L bars, or its sign)
position(s, t)   = LONG  if own_trend(s, t) > entry_threshold
                   FLAT  otherwise
```

- **Absolute, not relative.** The decision for asset `s` uses ONLY `s`'s own price
  history — never a comparison to the other 9 names. This is the load-bearing
  difference from the three retired families and the whole point of the experiment.
- **LONG / FLAT only — NO shorts, NO cross-sectional ranking.** Long when the
  asset's own trend is positive (above the entry threshold), flat (in cash) when it
  is not. This deliberately fits the existing long-only, solvency-guarded engine
  (apples-to-apples with the momentum/MR/carry anchors) and reuses the banked OHLCV
  with no new data source.
- **The portfolio is the equal-weight average of the per-asset long/flat rules** —
  when `k` of the 10 names are long, capital is split across those `k` (or held in
  cash when 0 are long), under the existing exposure cap.

_Acceptance: the per-asset position is a pure function of that asset's own price
history over the lookback; a unit test on a synthetic price series confirms the
rule goes long above the entry threshold and flat below it, independent of the
other symbols' series._

> **The entry/flat threshold is the no-trade band — and it IS a swept θ-axis.**
> A zero-threshold rule (long whenever trailing return > 0) whipsaws hardest; a
> wider band requires a more decisive trend to enter and a more decisive reversal
> to exit, trading responsiveness for fewer fee-paying flips. This band is
> TS-momentum's analogue of carry's rebalance cadence — the turnover lever — and
> the θ-surface spans it (R-TSM.7 / Backtest Scenarios).

### R-TSM.2 — Universe & data: the banked 10-symbol OHLCV, REVISION-pinned

- **Universe = the ORIGINAL 10-symbol large-cap set** under `data/binance`
  (`ADAUSDT, AVAXUSDT, BNBUSDT, BTCUSDT, DOGEUSDT, DOTUSDT, ETHUSDT, LINKUSDT,
  SOLUSDT, XRPUSDT`), the SAME universe the entire robustness program (momentum/MR/
  carry surfaces, the frozen decision rule, the BH control, the 89 anchors) lives
  on. Per the diagnosis § S.5.2, this keeps TS-momentum directly comparable to the
  three retired families and reuses the banked pin with **zero new data-plumbing
  risk** — a time-series rule trades each asset's own trend and gains nothing from
  cross-name dispersion, so there is no TS-specific reason to pay the broader-universe
  cost.
- **OHLCV revision pin `3a8b96c4…`** (`data/binance/REVISION.toml`), 1h bars, verified
  by the harness's existing `RealDataBarSource` loader exactly as momentum/MR did.
  **NO new data source** — this is the key simplification over carry (which had to
  load, align, and co-resample a funding series). TS-momentum reads the SAME closes
  the price families already read.
- **Both regimes from day 1: 2023-FY + 2024-FY** (both banked, both REVISION-pinned),
  per the frame-diagnostic E1 precedent that the carry build adopted — the 2024 bar
  (BH +1.10, tail-negative) is the harder/fairer regime.

### R-TSM.3 — Harness, decision rule, and control: reuse verbatim

- **Through the EXISTING block-bootstrap robustness harness** — `run_path` +
  `DistributionSummary` + `BlockBootstrapPathGen`, shared-index, exactly as the
  three cross-sectional families ran. No harness redesign.
- **Scored against the frozen § 0 decision rule** (the 5-signal weakest-link
  composite; void-if-`gbm-smoke`-or-per-symbol-independent), pre-registered BEFORE
  the run.
- **The SAME buy-and-hold control** (equal-weight, auto-L bootstrap) re-asserting
  the +1.74 (2023) / +1.10 (2024) bar. The TS-momentum family verdict is read
  relative to it.

### R-TSM.4 — Money & timing discipline (inherited non-negotiables)

- **Decimal money throughout** (ADR-0003) — no `f64` in any equity / sizing path.
- **Strict no-look-ahead** — the per-asset position at bar `t` uses ONLY price
  information available at or before `t` (the trailing return over `[t−L, t]`,
  decided at `t`, acted on no earlier than `t`). A look-ahead falsifier (below)
  guards this.

### R-TSM.5 — Determinism & additivity: the 89 anchors hold byte-identical

- **TS-momentum slots in DEFAULTS-OFF, exactly like carry.** Whatever seam the
  architect chooses (Q-TSM-1 below), it MUST default to the existing behavior so
  that **every momentum / MR / carry / buy-and-hold run is byte-identical by
  construction** and the **89 existing anchors** (87 pre-existing + carry #88 2023
  + carry #89 2024) stay byte-unchanged with no re-lock. This is the SAME additive
  discipline MR used for `direction` and carry used for `score_source` + the
  optional funding path.
- **Two-run byte-identity** of the TS-momentum θ-surface body-SHA on the canonical
  box (ADR-0051 D2/D3 precedent).
- **+1 or +2 anchors** (89 → 90, or → 91 if both regimes are locked) after the
  developer's anchored run; the tester locks them. The grid + N are locked at design
  time (architect, per the MR/carry precedent — the grid IS a hashed body field).

### Requirements summary (consolidated)

- **R-TSM.1** — Signal = per-asset absolute momentum, LONG/FLAT on the asset's OWN
  trailing-return sign vs an entry/flat threshold. NO shorts, NO cross-sectional
  ranking. Pure function of that asset's own price history.
- **R-TSM.2** — Universe = the banked 10-symbol large-cap set (`data/binance`, pin
  `3a8b96c4…`), 1h, 2023 + 2024. NO new data source.
- **R-TSM.3** — Through the existing block-bootstrap harness + frozen § 0 decision
  rule + the SAME buy-and-hold control (+1.74 / +1.10 bar).
- **R-TSM.4** — Decimal money; strict no-look-ahead.
- **R-TSM.5** — Additive / defaults-off → 89 anchors byte-identical; two-run
  byte-identity; +1/+2 TS anchors after the anchored run.
- **R-TSM.6** — Mandatory day-1 falsifiers (next section), each RED-on-revert.
- **R-TSM.7** — θ-surface at N=200 on 2023 + 2024 vs the BH control (Backtest
  Scenarios); the architect locks the exact θ-axes + cells.

---

## Mandatory day-1 falsifiers (NON-NEGOTIABLE — modeled on carry R-CARRY.2/6/10a/10b)

Per CLAUDE.md (every strategy overlay / sizing-modifier ships a
baseline-equity-divergence e2e test from day 1, the v3-vol-overlay no-op precedent)
and the program's both-axes-from-day-1 discipline, TS-momentum ships the following
falsifiers, **each RED-on-revert** (the test must FAIL if the behavior it guards is
reverted — the carry build's `r_carry_10a_red_on_revert_*` pattern is the template).
They ship in the test file with the strategy, NOT after.

1. **F-TSM.1 — Baseline-equity-divergence e2e (the headline anti-no-op; the
   CLAUDE.md non-negotiable).** A fast e2e (small N, short synthetic price series,
   NO real data — about wiring) runs the SAME path through the TS-momentum strategy
   and a passive equal-weight buy-and-hold, and asserts the TS-momentum equity curve
   **measurably diverges** from the passive-hold equity by **≥ 1 bp** when the trend
   signal is non-trivial (the synthetic series is constructed to contain at least one
   sustained downtrend the TS rule exits and BH sits through). This is the carry
   R-CARRY.10a analogue and the EXACT pattern of
   `crates/strategy/tests/vol_targeting_overlay_end_to_end.rs` that CLAUDE.md
   mandates. **RED-on-revert:** an always-long (no-op) TS rule produces Δ=0 vs BH →
   the test fails, proving it detects the no-op.
2. **F-TSM.2 — Signal-non-no-op (the trend signal is load-bearing, not decorative).**
   Force the trend signal to a constant / degenerate value (e.g. always-positive →
   always-long) and assert the equity curve **collapses** to the buy-and-hold case
   (Δ < ε) — proving the long/flat decision is what produces the divergence, not an
   incidental sizing artifact. (The sibling of carry's R-CARRY.10b cashflow-non-no-op
   gate: the value is computed AND applied, not computed-and-ignored.)
3. **F-TSM.3 — No-look-ahead falsifier.** Assert a bar's position uses ONLY the
   trailing return available at or before its decision time — shifting the price
   series one bar into the future changes the position/equity (proving the trailing
   window is causal, the carry R-CARRY.6 analogue). RED if a future bar leaks into
   the current decision.
4. **F-TSM.4 — Goes-flat falsifier (TS-specific — the must-actually-exit gate).**
   The strategy MUST actually exit to FLAT at least sometimes on a series that
   contains a downtrend — else it is just always-long ≈ buy-and-hold and the entire
   method collapses to the passive control it is meant to beat. Construct a synthetic
   series with a clear sustained downtrend and assert the TS rule holds a FLAT (zero/
   cash) position on ≥ 1 bar during it. **RED-on-revert:** a rule wired to never exit
   (always-long) fails this test. *This is the falsifier that proves TS-momentum is a
   genuinely different strategy and not buy-and-hold wearing a trend hat — it has no
   carry/MR analogue because going-flat is the load-bearing TS mechanism.*
5. **F-TSM.5 — Two-run byte-identity of the TS θ-surface body-SHA** (ADR-0051
   D2/D3/§D6.4): run the small-N TS sweep twice at the same `ensemble_seed`; assert
   identical `report_body_hash`. Catches any unordered fold in the per-asset signal
   loop or the surface renderer.

Pattern references the architect/developer should reuse:
`crates/strategy/tests/vol_targeting_overlay_end_to_end.rs` (the CLAUDE.md
no-op-overlay non-negotiable — directly applicable to F-TSM.1/F-TSM.2);
`crates/backtest/tests/carry_divergence_e2e.rs` (the carry sibling divergence +
RED-on-revert + sign + no-look-ahead + two-run gates F-TSM.1/3/5 mirror);
`crates/backtest/tests/param_sweep_e2e.rs` (the θ-surface two-run + anti-cherry-pick
gates).

---

## Backtest Scenarios
_analyst proposes the SHAPE; the **architect LOCKS the exact θ-axes + cells + N**
before the tester anchors (per the MR/carry precedent — the grid IS the hashed
anchor input)._

The primary anchored deliverable is a **TS-momentum θ-surface** of the SAME shape
as the carry-C3 6×200 surface — a small LOCKED θ-grid spanning **the TS lookback ×
the flat/entry threshold** (the two TS-specific axes; the architect locks the exact
values), at **N=200 paths/cell on 2023-FY AND 2024-FY**, against the buy-and-hold
control. Both regimes run from day 1 (the carry/E1 precedent); the 2024 surface is
the harder bar (BH +1.10, tail-negative).

1. **TSM-C3 (PRIMARY, ANCHORED) — 2023-FY**:
   `v1-ts-momentum-theta-surface-2023-block-bootstrap-real-fy` (architect confirms
   the scenario slug). The LOCKED θ-grid (lookback × flat/entry threshold), N=200/
   cell, shared-index block-bootstrap of 2023-FY real Binance OHLCV (pin
   `3a8b96c4…`), 6 bps fees (2 slippage + 4 taker, inherited). ONE anchored
   θ-surface report: per-cell FRAGILE/MARGINAL/ROBUST + family verdict + per-cell
   `→ C5` flags + the trades column (turnover legibility) + (TS-specific) a
   **time-in-market / fraction-flat** column so the goes-flat behavior and the
   whipsaw cost are legible per cell.
2. **TSM-C3 (PRIMARY, gating) — 2024-FY**: the SAME LOCKED grid on 2024-FY as the
   multi-regime corroboration (BH +1.10 bar). SEPARATE run → SEPARATE anchor if the
   tester elects to lock it (the durable choice → 91), OR a gating-but-anchor-optional
   read (the tester's call at lock time, exactly as carry handled #89).
3. **Control (in each surface)** — buy-and-hold equal-weight under the same N paths
   + auto-L bootstrap, re-asserting the +1.74 (2023) / +1.10 (2024) bar. This row
   carries no verdict; the TS family verdict is read relative to it.

**Plan to anchor: +1 (2023 only → 90) or +2 (both regimes → 91).** The durable
choice is to lock both regimes (91); deferring the 2024 lock to a gating read is
the acceptable if-wall-clock-tight fallback (the carry precedent #88/#89). The grid
+ N + the TS axes are hashed body fields (K3) once locked.

> **Wall-clock gate (carried to the developer).** The carry 6×200 + control ran
> ~2 min extrapolated (N=3 smoke 1.7s → ~113s at N=200). TS-momentum is **cheaper**
> than carry per path — no funding gather, a per-asset trailing-return is O(n_bars)
> over the SAME closes — so the 6×200 (or whatever cell count the architect locks)
> × 2 years is expected to be comfortably within the ≲30 min gate. The developer
> MUST re-validate the wall-clock before locking (the C3 lesson:
> `wall-clock ≈ grid × N × per-path cost`), but no blow-up is expected. Emit a
> `watch -n 30 'tail -n 5 <progress-log>'` block when kicking off the anchored run
> (per the long-running-task recipe).

---

## Open design questions (FOR THE ARCHITECT M-T1 — framed, NOT answered)

These are the analyst-framed decisions the architect resolves next (with the
analyst's lean noted where I have one, NOT locked). I deliberately do **not** answer
them — the seam, the grid, and the engine-fit are architect M-T1 calls.

- **Q-TSM-1 (the headline — what IS time-series momentum in the type system?).**
  Is TS-momentum a new **`ScoreSource`** (a sibling to `VolAdjustedReturn` /
  `FundingCarry` on the cross-sectional config), a new **`Direction`** variant, or a
  genuinely new **strategy type**? The tension: the three cross-sectional families
  share a *ranking-then-select-top-K* shape, and TS-momentum deliberately has NO
  ranking — every asset decides long/flat on its own, so the `top_k_long` selector
  may not be the right reuse at all. The architect must judge whether TS-momentum is
  expressible as a per-asset transform on the existing score path (cheap, reuses the
  engine, risks contorting a ranking machine into a non-ranking rule) OR warrants a
  distinct long/flat-per-asset strategy. This is the load-bearing M-T1 decision and
  governs the answers to Q-TSM-2..4.
- **Q-TSM-2 (how does LONG/FLAT sizing slot into `run_path` / PaperEngine?).** The
  engine is currently long-only **cross-sectional top-k**: it ranks, takes the top
  K, and equal-weights them under the solvency cap. TS-momentum's portfolio is "the
  equal-weight set of all assets currently above their own entry threshold" — a
  *variable-cardinality* long/flat basket (0..10 names long, the rest in cash), NOT
  a fixed top-K. How does that variable-cardinality long/flat sizing slot into the
  existing `run_path` sizing + exposure cap? (Is "all names above threshold" just
  `top_k_long` with `K=10` and a threshold gate that drops sub-threshold names to
  flat? Or a new sizing path?) The architect locks the exact mechanism and confirms
  it composes with the solvency guard.
- **Q-TSM-3 (the exact θ-grid — the locked, hashed cells).** Ratify or revise the
  proposed two-axis shape (lookback × flat/entry threshold) and LOCK the exact cell
  values + N before the tester anchors (the grid IS the hashed anchor input, per the
  MR/carry precedent). The analyst's lean: span the threshold axis from zero (pure
  long/flat-on-sign, the whipsaw extreme) to a deliberately wide band (the low-churn
  corner — TS-momentum's best structural shot at clearing the BH bar), × a short and
  a long lookback — the analogue of the carry grid's turnover-axis span. The exact
  cells are the architect's to lock.
- **Q-TSM-4 (does it reuse `run_path`'s CONCRETE `MomentumStrategy`, or need a
  variant?).** `run_path` is typed to the concrete `MomentumStrategy`
  (montecarlo.rs) — the SAME constraint that forced MR to be an enum-on-config and
  carry to be a `ScoreSource` + `funding_override` rather than a new struct (a
  sibling struct forces `run_path` generic/`dyn` and risks all 89 anchors, the
  ADR-0051 § D6.5.2 trap). Can TS-momentum live as an additive config variant on
  `MomentumStrategy` (the anchor-safe path, consistent with Q-TSM-1's `ScoreSource`
  framing), or does its non-ranking, variable-cardinality, long/flat nature force a
  variant that the architect must reconcile with the concrete-`run_path` /
  anchor-preservation constraint? This is the additivity question — whatever the
  answer, R-TSM.5 (89 anchors byte-identical, defaults-off) is non-negotiable.

---

## Verification (the tester gates)
_tester links to reports here after the build_

The tester closes the loop with the standard report template and these gates:

- **The 5 day-1 falsifiers RED-on-revert** (F-TSM.1 baseline-equity-divergence,
  F-TSM.2 signal-non-no-op, F-TSM.3 no-look-ahead, F-TSM.4 goes-flat, F-TSM.5
  two-run byte-identity) — each must FAIL when its guarded behavior is reverted (the
  carry `red_on_revert` discipline).
- **The 89 existing anchors stay byte-identical** (`scripts/verify_anchors.sh` →
  89/89 PASS) — the TS path is additive / defaults-off, so momentum #86, MR #87,
  carry #88/#89, and all pre-existing anchors are byte-unchanged by construction.
- **The new TS θ-surface anchor(s)** locked after the developer's anchored N=200 run
  (89 → 90, or → 91 if both regimes are locked) — the tester locks them only after
  the verify-anchors PASS.
- **Two-run byte-identity** of the TS θ-surface body-SHA on the canonical box.
- **Pre-flight void-if-fail** — both surface headers print
  `generator: block-bootstrap-real` AND `bootstrap_mode: shared-index`.
- **Anti-cherry-pick (FP-C3.5 reused)** — family-summary ∈ allowed values; any
  non-FRAGILE cell carries `→ C5 DEFLATION REQUIRED` (and IF a cell is non-FRAGILE,
  the C5 PBO/Deflated-Sharpe deflation pass is genuinely owed — unlike the
  uniform-negative momentum/MR/carry results where C5 was moot).
- **The family verdict is read relative to the buy-and-hold control** (+1.74 / +1.10)
  under the frozen decision rule, pre-registered.

---

## Scope & honesty (no overclaim)

- This brief scopes a method and its falsifiers + Backtest Scenarios; it commits NO
  code, triggers NO engine run, and writes NO Design / tasks / implementation —
  those are the architect's next (the analyst stays at altitude).
- TS-momentum reuses the banked 10-symbol OHLCV (pin `3a8b96c4…`) with **NO new data
  source** — it is materially simpler than carry (no funding load / align /
  co-resample). The new engineering is the per-asset long/flat signal + sizing + the
  5 falsifiers; the harness, bootstrap, decision rule, BH control, and anchor
  machinery already exist.
- The robustness axis judges **resampled real 2023 + 2024 history** only — it cannot
  speak to a regime those years never contained, nor to a different horizon (daily);
  the banked data is 1h (inherited scope limit, decision-rule § 5).
- **No alpha is claimed.** This is uncertainty quantification of a candidate method,
  not prediction (inherited framing). The +1.74 (2023) / +1.10 (2024) buy-and-hold
  bar is the honest benchmark TS-momentum must clear to matter.
- **Honest prior: MEDIUM.** TS-momentum is the most robust documented crypto effect,
  but the same 1h large-cap universe that killed the cross-sectional families may be
  efficient enough at the directional level too. The value is in the clean
  disambiguation (the diagnosis § 4.1 logic table) regardless of sign — a FRAGILE
  result here CLOSES the active-trading thesis on this universe and routes to the
  pre-positioned broader-universe / horizon axis; a non-FRAGILE result is the FIRST
  robust strategy in the program and pivots the product to time-series.

---

## Changelog

- 2026-06-02 (analyst, feature scoping): authored the `time-series-momentum-robustness`
  feature brief — the FIRST time-series (non-cross-sectional) family, greenlit by the
  operator after the [universe-method diagnosis](../dev-notes/universe-method-diagnosis-2026-06-02.md)
  proved the cross-sectional RANKING channel is the dead channel (rank IC ≈ 0 at
  every horizon both years; broader-universe spike LOWERED common-beta 0.715→0.598
  but did NOT revive rank IC → METHOD-limiter, HIGH confidence). **Why:** TS-momentum
  removes the ranking channel entirely (per-asset absolute momentum, long/flat) —
  does that produce a robust edge over passive BH (+1.74/+1.10), or is active
  trading on this universe dominated end-to-end (closing the thesis)? **Requirements
  (R-TSM.1-7):** per-asset absolute momentum LONG/FLAT on the asset's OWN trailing
  return (NO shorts, NO ranking); the banked 10-symbol OHLCV (pin `3a8b96c4…`, NO
  new data source); through the existing block-bootstrap harness + frozen § 0
  decision rule + the SAME buy-and-hold control; Decimal money; strict no-look-ahead;
  additive/defaults-off → 89 anchors byte-identical. **Mandatory day-1 falsifiers
  (F-TSM.1-5, modeled on carry R-CARRY.2/6/10a/10b, each RED-on-revert):**
  baseline-equity-divergence e2e (CLAUDE.md non-negotiable), signal-non-no-op,
  no-look-ahead, the TS-specific **goes-flat** falsifier (must actually exit to flat
  — else ≈ buy-and-hold), and two-run byte-identity. **Backtest Scenarios:** a
  θ-surface (TS lookback × flat/entry threshold — architect locks the axes) at N=200
  on 2023 + 2024 vs the BH control, the same shape as the carry-C3 6×200; plan to
  anchor +1 (→90) or +2 (→91). **Open design questions framed for the architect
  (Q-TSM-1..4, NOT answered):** is TS-momentum a new `ScoreSource` / `Direction` /
  strategy type; how does variable-cardinality long/flat sizing slot into
  `run_path`/PaperEngine (currently long-only cross-sectional top-k); the exact
  locked θ-grid; does it reuse the concrete `MomentumStrategy` or force a variant
  (the ADR-0051 § D6.5.2 anchor-risk constraint). status proposed; `[[req]]` row
  `REQ-TIME-SERIES-MOMENTUM-ROBUSTNESS-001` created (operator greenlit). No code, no
  build, no engine run; no anchors.toml touch (tester owns those at lock time).
