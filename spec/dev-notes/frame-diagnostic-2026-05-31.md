---
slug: frame-diagnostic-2026-05-31
status: draft
owner: analyst
updated: 2026-05-31
tags: [frame-diagnostic, regime, fees, turnover, buy-and-hold, robustness, carry, oos, 2024-fy, monte-carlo, pre-build-gate]
related:
  - spec/momentum-parameter-robustness-sweep/presentations/momentum-robustness-closure-2026-05-30.md
  - spec/cross-sectional-mean-reversion-strategy/feature.md
  - spec/dev-notes/robustness-verdict-adversarial-review-2026-05-30.md
  - spec/dev-notes/robustness-decision-rule-2026-05-30.md
  - spec/momentum-parameter-robustness-sweep/reports/robustness-sweep-20260530-180006-v1-momentum-theta-surface-2023-block-bootstrap-real-fy.md
---

# Frame diagnostic — is the BAR a regime artifact, and are FEES the structural killer?

> **Mandate (cheap, ~hours, pre-build gate).** Two active cross-sectional
> strategies (momentum, mean-reversion) both came back FAMILY-UNIFORM-FRAGILE
> on 2023-FY and both lost to a passive buy-and-hold control (BH p50 Sharpe
> +1.74 vs active ≈ 0). Before committing ~4.5–7.5 dev-days to a 3rd strategy
> (carry), settle two questions that could reframe everything: **(E1 REGIME)** is
> BH's +1.74 dominance a 2023-bull artifact? **(E2 FEE)** is the friction the
> structural killer? Both experiments run on the EXISTING harness
> (`param_robustness_sweep`), varying only run config. NO new strategy, NO engine
> logic change.

---

## 0. TL;DR — the two numbers, and the steer

| Question | Decisive number | Answer |
|---|---|---|
| **E1: Is BH's +1.74 dominance a 2023-bull artifact?** | 2024 BH p50 Sharpe **+1.10** (was +1.74); 2024 momentum **still FAMILY-UNIFORM-FRAGILE**, all 6 cells p5 < 0, P(S>1)=0% | **PARTLY** — BH's edge shrank ~37% (and its tail turned negative: p5 +0.12 → −0.68), so a bull-component is real. BUT momentum loses in BOTH years on its own merits. The bar is **FAIR**, not a pure bull artifact. |
| **E2: Are fees the structural killer?** | At **0 bps** (all friction removed) on the SAME 2023 paths, momentum is **STILL FAMILY-UNIFORM-FRAGILE**: every cell p5 < 0, **P(S>1) = 0.0% in all 6 cells**, p95 MaxDD still 81–92% | **NO** — fees are a real drag (they lift the median / cut loss-rate ~10–23pp in churny cells) but removing 100% of them does **not** manufacture an edge. The signal itself is weak. |

**RECOMMENDATION — build carry, but on the SIGNAL thesis, not the low-turnover thesis; and add a multi-regime guard.**

This is the brief's **"Bar fair + signal-weak-even-at-0bps"** branch with a regime
nuance layered on top:

1. **The bar is real.** Active cross-sectional price strategies (momentum AND its
   inverse, MR) lose to passive holding in two independent years, and momentum
   loses *even at zero fees*. There is no "the test was unfair" escape. Any 3rd
   strategy must clear a genuine bar.
2. **Low-turnover alone will NOT rescue a price-trend/price-reversion signal** —
   E2 proves the fee-drag is not the binding constraint; the signal is. So the
   *motivation* for carry must be **"a structurally different return source
   (funding/basis, not price-direction)"**, NOT merely "carry trades less so it
   dodges the fee trap." If carry's only pitch is low turnover, E2 says it will
   still fail. Carry must earn from its *signal* (the funding/basis premium).
3. **Add an OOS/multi-regime evaluation to carry's plan from day 1.** E1 shows the
   2023 single-year bar carries a bull-component (BH +1.74 → +1.10 across years).
   Evaluating carry on 2023-only would inherit that distortion. Carry should be
   vetted on **both 2023 AND 2024** (both banked on disk) through the same C2/C3
   harness, and judged on the harder/fairer 2024 bar too.

**If-budget-tightens fallback:** if the ~4.5–7.5d carry build is too expensive
right now, the cheaper-but-still-informative move is **NOT** another price-based
strategy (momentum's family verdict + MR's family verdict + the 0bps result make
the whole price-direction class a poor bet on this 10-coin 1h universe). The
cheaper fallback is a **data/feasibility spike on carry**: confirm funding-rate /
basis data is fetchable and bankable for these 10 symbols before committing to the
full strategy build. That de-risks the single biggest unknown (carry needs a data
source momentum/MR did not) at ~0.5–1d cost.

---

## 1. What was run (reproducibility)

Both runs reuse `crates/backtest/src/bin/param_robustness_sweep.rs` (the C3
θ-surface sweep) at the **same** 6-cell Tier-1 momentum grid, N=200,
ensemble_seed=0xC0FFEE, generator=block-bootstrap-real, shared-index, auto-L,
revision `3a8b96c4…`. Outputs written to `/tmp/frame-diag/` (throwaway —
NOT anchored; the 2023 anchor #86 + MR anchor #87 stay byte-identical).

- **E1 (REGIME):** `--year 2024` (zero code change — the bin already has a
  `--year` flag and `bar_count` handles 2024 = 8784 bars). The 2024 OHLCV for all
  10 symbols is on disk and covered by the pinned revision SHA. Wall-clock 1254.7s.
  Report: `robustness-sweep-20260531-163149-v1-momentum-theta-surface-2024-block-bootstrap-real-fy.md`.
- **E2 (FEE):** `--year 2023 --match-slippage-bps 0 --match-taker-fee-bps 0`.
  Wall-clock 1222.8s. Report:
  `robustness-sweep-20260531-165419-v1-momentum-theta-surface-2023-block-bootstrap-real-fy.md`.

### ⚠️ CLI gap found + how E2 was made feasible (the brief's premise was off)

The brief said: *"find the slippage CLI flag (`--sim-slippage-bps` or similar from
the v5 work)."* **No such flag exists in `param_robustness_sweep`.** The bin
**hardcoded** `slippage_bps: 2, taker_fee_bps: 4` in the `TcnScenarioInput` literal
(was lines 1268–1269) — there was no way to vary friction via CLI args alone.

Per the brief's own contingency clause (*"flag it as a small dev change needed"*),
E2 required a **minimal, DISPOSABLE, uncommitted** CLI change: two flags
(`--match-slippage-bps`, `--match-taker-fee-bps`, defaults 2/4 = anchor baseline)
threaded into the **already-existing** `TcnScenarioInput.slippage_bps` /
`.taker_fee_bps` scalar fields that `montecarlo::run_path` *already* reads
(montecarlo.rs:93–94, 112–113). **No engine logic changed** — the fill math in
`paper.rs` is byte-untouched; the change only makes two existing scalar inputs
settable instead of hardcoded. **This edit MUST be reverted before any commit**
(it is +21/−2 in `param_robustness_sweep.rs`; see § 5). At default flag values the
bin reproduces anchor #86 byte-identically (the defaults are 2/4).

### ⚠️ Friction-model correction (the "8bps" in the brief is NOT what the harness runs)

The brief states *"the v5 friction is 8bps slippage (operator-ratified Q-D1=(a))."*
That 8bps is the **v5-realdata-medium scenario** friction (`LatencySlippageSimConfig`
namespace `v5-realdata-medium-2026-05`). **The Monte-Carlo robustness harness
(`param_robustness_sweep` / `monte_carlo`) does NOT use it** — it passes
`LatencySlippageSimConfig::default()` which is **noop (Linear{bps:0})**. The
*active* cost in the robustness sweeps is the `PaperEngine` `MatchConfig`:
**slippage_bps=2 (per side) + taker_fee_bps=4 (per side)** = **6 bps per side ≈ 12
bps round-trip** at the fill level (paper.rs:77–88). The closure deck's "6 bps
round-trip" undercounts (it's 6 per side); either way the friction that produced
FAMILY-UNIFORM-FRAGILE is the 2+4 MatchConfig, NOT the v5 8bps layer. **E2's
"0bps" zeroes the active MatchConfig friction (the 2+4), which is the correct
thing to zero.** maker_fee_bps is irrelevant — all harness fills are taker
(Market/IOC → FeeTier::Taker).

---

## 2. Experiment 1 — REGIME (2024-FY)

### Buy-and-hold: dominance shrank but held

| | 2023-FY (anchor) | 2024-FY (E1) | Δ |
|---|---|---|---|
| BH p50 Sharpe | **+1.735** | **+1.105** | −0.63 (−37%) |
| BH p5 Sharpe (tail floor) | +0.124 | **−0.682** | tail flipped **negative** |
| BH P(loss) | 4.5% | 16.5% | ×3.7 |
| BH P(Sharpe>1) | 77.5% | 53.5% | −24pp |
| BH p95 MaxDD | 51.2% | 64.8% | +13.6pp |

**Reading:** 2024 was a *less generous, more two-sided* drift environment than the
2023 bull. BH's median still clears the Sharpe-1 promotion bar (+1.10), but its
**bad-case tail now loses money** (p5 −0.68) and it loses outright 16.5% of the
time. So part of the 2023 "+1.74 BH crushes everyone" headline **was** a bull
artifact — in a flatter year the passive edge is materially smaller and no longer
tail-safe.

### Momentum 2024: still FAMILY-UNIFORM-FRAGILE (all 6 cells)

| g | lookback | p5 | p50 | P(loss) | P(S>1) | p95 MaxDD | verdict |
|---|---|---|---|---|---|---|---|
| 0 | 60 | −0.0223 | −0.0013 | 56.0% | 0.0% | 82.2% | FRAGILE |
| 1 | 24 | −0.0224 | −0.0017 | 57.0% | 0.0% | 84.6% | FRAGILE |
| 2 | 168 | −0.0189 | +0.0075 | 30.5% | 0.0% | 79.0% | FRAGILE |
| 3 | 720 | −0.0131 | +0.0312 | 16.5% | 0.0% | 66.7% | FRAGILE |
| 4 | 60 | −0.0151 | −0.0019 | 59.5% | 0.0% | 67.5% | FRAGILE |
| 5 | 60 | −0.0262 | +0.0012 | 48.0% | 0.0% | 86.3% | FRAGILE |

**Reading:** momentum is fragile in 2024 by the same mechanism as 2023 — **p5 < 0
in every cell, P(Sharpe>1) = 0% in every cell.** The best cell (g=3, 1mo lookback +
wide hold-band) again has the best median (+0.031, even better than 2023's +0.014)
and its loss-rate (16.5%) now *equals* BH's 2024 loss-rate — but its p5 is still
−0.013 < 0, so still FRAGILE.

### E1 verdict: the bar is FAIR (with a regime caveat)

- BH's dominance is **partly** a bull artifact (it shrank ~37% and lost its tail
  safety in 2024). The framing "fragile vs BH partly means you can't beat a bull
  by trading" has *some* truth — BH was a tougher benchmark in 2023 than 2024.
- BUT momentum loses on its **own** merits in BOTH years (negative tail, 0% clear
  the bar), independent of how strong BH is. **The bar is not an artifact of BH
  being unbeatable; active momentum is genuinely weak across two regimes.**
- **Consequence:** evaluating a new strategy on **2023-only** would inherit a
  bull-distorted benchmark. Multi-regime (2023 + 2024) evaluation is warranted —
  this is the actionable regime finding.

---

## 3. Experiment 2 — FEE (0 bps, 2023-FY)

### Momentum at 0bps vs the 8bps* baseline (anchor #86), SAME 2023 paths

(*baseline = the active MatchConfig 2+4; see § 1 friction correction.)

| g | p5  8→0bps | p50  8→0bps | P(loss)  8→0bps | P(S>1)  8→0bps | p95 MaxDD  8→0bps | verdict |
|---|---|---|---|---|---|---|
| 0 | −0.0491 → −0.0460 | −0.0081 → −0.0036 | 76.0% → 53.0% | 0.0% → 0.0% | 91.5% → 91.1% | FRAGILE |
| 1 | −0.0482 → −0.0381 | −0.0215 → −0.0124 | 93.5% → 69.0% | 0.0% → 0.0% | 93.3% → 91.5% | FRAGILE |
| 2 | −0.0583 → −0.0555 | +0.0017 → +0.0076 | 45.0% → 31.0% | 0.0% → 0.0% | 88.2% → 88.7% | FRAGILE |
| 3 | −0.0320 → −0.0291 | +0.0137 → +0.0173 | 18.5% → 16.5% | 0.0% → 0.0% | 81.7% → 81.6% | FRAGILE |
| 4 | −0.0773 → −0.0742 | −0.0070 → −0.0029 | 83.0% → 63.5% | 0.0% → 0.0% | 89.3% → 89.1% | FRAGILE |
| 5 | −0.0462 → −0.0322 | −0.0046 → +0.0074 | 61.5% → 35.0% | 0.0% → 0.0% | 92.0% → 90.2% | FRAGILE |

**Family verdict at 0bps: FAMILY-UNIFORM-FRAGILE** (unchanged).

BH control at 0bps = **+1.7353** (byte-identical to the 8bps baseline +1.7353) —
a free correctness check: BH does one round-trip at bar 0, so friction barely
touches it, and the harness reproduced the 2023 baseline exactly.

### E2 verdict: fees are a DRAG, not the KILLER

- **The killer signals do not move:** p5 Sharpe stays **negative in all 6 cells**
  even at zero friction (best is still −0.029 at g=3); **P(Sharpe>1) stays exactly
  0.0% in all 6 cells**; p95 MaxDD barely budges (81–92%). The fragility is
  **structural to the signal**, not a fee-bleed artifact.
- **Fees are nonetheless a real, measurable drag:** removing them lifts the median
  modestly and cuts the loss-rate by ~10–23pp in the high-churn cells (g=0:
  76%→53%, g=1: 93.5%→69%, g=5: 61.5%→35%). So lowering fees *helps*, but it helps
  a strategy that **still has no positive edge** — it raises a losing distribution
  toward break-even, never past the bar.
- **Direct answer to the brief's E2 fork:** this is the **"momentum stays FRAGILE
  even at 0bps → fees are NOT the (sole) killer; the signal itself is weak"**
  branch. Therefore **carry's edge must come from its signal, not just from low
  turnover.** A low-turnover/low-fee reframe is necessary-but-not-sufficient: it
  removes a drag that, on this universe, was never the binding constraint.

---

## 4. Synthesis → the steer (build carry / reframe / something else)

**Build carry — with two conditions — is the recommended path.** Grounding:

- **Why not "reframe / different bar"?** E1 already partly reframes the bar (it has
  a bull-component), but the reframe does NOT rescue active price strategies:
  momentum loses in 2024 too, and at 0bps too. Re-cutting the bar further (e.g. a
  cost-free or different-venue benchmark) cannot turn a p5<0 / 0%-clear-the-bar
  signal positive. The reframe's only actionable output is "evaluate on 2 regimes,"
  which is folded into the carry plan below.
- **Why not another price-direction family (breakout, x-sec value)?** The
  accumulated evidence — momentum FAMILY-UNIFORM-FRAGILE (2 years), MR
  FAMILY-UNIFORM-FRAGILE (2023), momentum FRAGILE at 0bps — indicts the whole
  **price-direction class** on this 10-coin 1h universe. Breakout is trend-adjacent
  (same prior). X-sec value is the longest-horizon price-ranking but still price-
  based; E2 says low turnover alone won't save it. Spending the next sprint on
  another price strategy is the low-expected-value choice.
- **Why carry (the durable choice)?** Carry (funding-rate / basis capture) is the
  **only candidate whose return source is structurally independent of price
  direction** — it harvests the perpetual-funding / spot-basis premium, which is
  present (and often positive) regardless of whether price trends or mean-reverts.
  It is the cleanest test of the sharpest open question the controls raise: *"is
  there ANY active family that beats simply holding, net of fees, on this universe
  — and is that edge a non-price one?"* That is worth a correct, durable build.

### The two conditions on the carry build (lock these into its feature.md)

1. **Pitch carry on the SIGNAL, not on low-turnover.** E2 forbids the "carry trades
   less so it dodges the fee trap" thesis as the primary motivation — that thesis
   predicts failure (the fee trap was not binding). Carry's `feature.md § Why` must
   claim the edge from the **funding/basis premium itself**; low turnover is a
   secondary nice-to-have, not the load-bearing argument.
2. **Multi-regime evaluation from day 1 (2023 + 2024).** Both years are banked.
   Run carry through C2 (path-robustness) and C3 (θ-surface) on **both** years and
   judge it on the harder/fairer 2024 bar (BH +1.10, tail-negative), not just the
   bull-inflated 2023 bar. This is the actionable E1 finding and is cheap (the
   `--year` flag already exists; +1 sweep per year).

### Data feasibility is the one genuine new unknown

Carry needs **funding-rate / basis data** that momentum/MR did not (they used
OHLCV only). Before (or as the first task of) the carry build, confirm this data is
fetchable and bankable for the 10 symbols. If budget is tight, do **this spike
first** (~0.5–1d) — it de-risks the largest unknown and is the recommended
"if-budget-tightens" fallback over any further price-strategy work.

---

## 5. Housekeeping / cleanup (MUST do before any commit)

- **REVERT the disposable CLI edit** in
  `crates/backtest/src/bin/param_robustness_sweep.rs` (+21/−2: the two
  `--match-slippage-bps` / `--match-taker-fee-bps` flags + the threading + the
  `slippage_bps`/`taker_fee_bps` params on `run_one_path_with_config`). Restore the
  hardcoded `slippage_bps: 2, taker_fee_bps: 4` literal. The bin must return to
  anchor-#86-reproducing state. **`git checkout -- crates/backtest/src/bin/param_robustness_sweep.rs`**
  is the clean revert (nothing else of mine is in that file).
- **Throwaway outputs:** `rm -rf /tmp/frame-diag/` — none of it is anchored.
- **Anchors untouched:** confirmed `spec/anchors.toml`,
  `spec/momentum-parameter-robustness-sweep/reports/`, and
  `spec/cross-sectional-mean-reversion-strategy/reports/` are byte-identical (empty
  `git diff`). E1/E2 wrote only to `/tmp`. The 8bps-2023 + MR anchors are intact.
- **Pre-existing working-tree changes NOT from this diagnostic** (flag, do not
  touch): `data/yahoo/REVISION.toml` (in the session-start snapshot) and
  `scripts/verify_anchors.sh` (MR anchor-resolution wiring from prior MR work).

---

## 6. Assumptions & limits (challengeable)

1. **N=200, single seed (0xC0FFEE).** Same resolution as the anchored C3 runs; tail
   percentiles have 0.5% steps. The verdicts are not close calls (p5<0 by wide
   margins, P(S>1)=0% exactly), so N=200 noise does not threaten either conclusion.
2. **2024 used the SAME momentum grid + auto-L (L=200 for 2024 vs L=204 for 2023).**
   The grid was not re-optimized for 2024 — deliberately, since the question is
   "does the 2023 verdict hold in 2024," not "find 2024's best θ." A θ tuned to
   2024 is out of scope (and would invite the overfit the robustness program
   exists to prevent).
3. **0bps removes the MatchConfig 2+4 only.** It does not model maker rebates,
   negative-fee venues, or the v5 square-root impact model — it is the cleanest
   "no friction at all" bound, which is exactly what the E2 fork needs. Any real
   venue has cost ≥ 0bps, so 0bps is the most generous possible case, and momentum
   fails it.
4. **Carry's edge is hypothesized, not measured here.** This note does NOT claim
   carry works — it argues carry is the best-motivated *next test* given that the
   price-direction class is indicted and fees are not the lever. Carry must still
   clear the same C2/C3 bar (on 2 regimes) before any paper/live consideration.
5. **Block bootstrap resamples real return blocks** — it cannot synthesize a regime
   neither 2023 nor 2024 contained. "Multi-regime" here means "two real banked
   years," not "all possible futures."

---

## Changelog

- 2026-05-31 (analyst, frame-diagnostic): ran two pre-build diagnostics on the
  existing `param_robustness_sweep` harness (NO new strategy, NO engine-logic
  change). **E1 (REGIME, `--year 2024`):** BH p50 Sharpe +1.74→+1.10 (shrank ~37%,
  tail flipped negative p5 +0.12→−0.68) but momentum STILL FAMILY-UNIFORM-FRAGILE
  in 2024 (all 6 cells p5<0, P(S>1)=0%) → bar is FAIR with a real bull-component.
  **E2 (FEE, 0bps via a disposable uncommitted CLI flag):** momentum STILL
  FAMILY-UNIFORM-FRAGILE at zero friction (p5<0 + P(S>1)=0% in all 6 cells; p95
  MaxDD 81–92% unchanged) — fees are a drag (loss-rate −10..−23pp in churny cells)
  but NOT the killer; the signal is weak. **STEER: build carry**, but pitched on the
  funding/basis SIGNAL (not low-turnover, which E2 shows is not the binding lever)
  and evaluated multi-regime (2023+2024). If-budget-tightens fallback: a
  funding/basis DATA-feasibility spike first (carry's one genuine new unknown), NOT
  another price-direction strategy (the whole price class is now indicted).
  Corrected two brief premises: (a) no slippage CLI flag existed in the bin
  (hardcoded 2/4) — E2 needed a disposable flag, MUST be reverted (§5); (b) the
  harness friction is the MatchConfig 2+4 (≈12bps round-trip), NOT the v5 8bps
  layer (which is noop in this harness). Outputs in /tmp (throwaway); anchors #86/#87
  byte-identical; NO git.
