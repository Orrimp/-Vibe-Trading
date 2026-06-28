# Application — Volatility-Targeting & Drawdown Overlays

*Decision doc for analyst + architect. Distilled from `research/risk-and-sizing/`
(100 papers, deep-read pass 2026-06-28). Citations `risk-and-sizing[N]` resolve in
`research/risk-and-sizing/papers.md`; the synthesis is `knowledge.md`. This file does
not add papers — it turns the completed research into candidate work.*

> **Our app:** Rust single-coin crypto **advisor** (paper/sim, not advice, not live).
> Pick coin + €200 → bake off every strategy → rank under a FROZEN 1000-path
> moving-block-bootstrap gate (FRAGILE ⇒ can't crown; buy-and-hold always the
> benchmark + exempt) → forward rule-based plan → watch it paper-trade. Validated
> thesis: **no active strategy robustly beats buy-and-hold net of costs.** We ALREADY
> ship a vol-targeting overlay (`crates/strategy/src/vol_targeting_overlay.rs`) + a
> vol kill-switch overlay (`vol_killswitch_overlay.rs`) + budget-aware fixed-fraction
> sizing (`crates/risk/src/sizing.rs`).

**Scope of this file:** the two overlays where this research is *most actionable* —
(1) **repositioning the shipped vol-targeting overlay** as a risk-shaping tool (not a
Sharpe tool), and (2) **a new drawdown-control overlay with high-water-mark restart**.
Plus the vol-forecasting input (σ̂) that feeds both. Position sizing / Kelly / bet-sizing
lives in the sibling file `application-position-sizing-and-bet-sizing.md`.

**One-line verdict:** this is the ONE area of the whole research program where active
management *plausibly* adds value — but as **drawdown / tail / variance reduction**,
**never** as a Sharpe or alpha gain on crypto. The crypto leverage effect runs the wrong
way; promise risk-shaping, prove it with a baseline-divergence e2e, and frame the upside
give-up honestly. That honesty *is* the product ("traceable and plausible trading").

---

## 1. Summary of the research

**Volatility targeting splits cleanly into a reliable half and an unreliable half.**
The canonical bull case [1] (Moreira–Muir) scales exposure by 1/σ² and reports large
factor-Sharpe gains — but those are *spanning-regression alphas* that secretly use
end-of-sample weights, **not** a real-time strategy. The decisive OOS rebuttal [2]
(Cederburg et al., 103 strategies) reproduces those alphas yet shows a genuine real-time
combination **underperforms simple holding in 72/103 cases**, driven by *structural
instability* (parameter breaks). The reconciliation [16] (Harvey et al., 60+ assets) is
the load-bearing result: vol targeting raises **Sharpe only for "risk assets" with a
strong leverage effect** (negative return→vol correlation: equities, credit) and has
negligible Sharpe impact for bonds/FX/commodities — but it **universally reduces extreme
returns, vol-of-vol, and maximum drawdown across all 60+ assets**, *even where Sharpe is
unchanged*. So: **risk-shaping reliable, Sharpe fragile.**

**Crypto's leverage effect is reversed — now quantified.** [16]'s Sharpe mechanism needs
vol to spike *after down* moves. The broad-panel deep read [93] (Brini–Lenz, 87 cryptos,
5-min) measures the **opposite**: daily leverage parameter **γ = −0.261\*\*\* for crypto
vs +0.115\*\* for equity** (opposite signs, both significant); *positive*-return
semivariance (ϕd+ = 1.262\*\*\*) drives future crypto vol while negative does not — the
"FOMO" effect. EGARCH [75] finds *no* asymmetry; [78] finds a present-but-regime-dependent
one. The honest synthesis: the equity mechanism that makes vol targeting Sharpe-accretive
is **absent, reversed, or unstable in crypto**. De-risking on a vol spike means de-risking
*after* the rally that caused it — cutting into the ~66%/yr historical premium [76], not
dodging a crash. → **Expect risk-shaping, not Sharpe. Full stop.**

**Tight vol-target tracking is not worth its turnover.** The most architecturally
on-point paper [90] (Boyd/Candès/Hastie, single-asset feedback control — *exactly our
construction*) shows closing the loop to track the target tightly cut tracking error
2.3%→0.4% (5.75× better) but blew **turnover 93% → 1105%/yr**; the authors themselves say
**open-loop is preferable for low-liquidity assets** (= crypto). Constant vol scaling on
futures momentum even *lowered* Sharpe (0.59→0.39) once turnover costs counted [48]. → Use
a **slow EWMA (~126-day half-life [90]) + a no-trade band [61], de-risk-only, rebalance
rarely.** Don't chase the target.

**Downside vol is the cleaner trigger.** Scaling by *downside* (semi-)deviation beats
total-vol scaling — and uniquely shows up in direct comparisons and real-time tests, not
just spanning regressions [59]; downside deviation is the Sortino object [84] and the
crash-relevant component [93][35].

**Drawdown control is the most deployable overlay, and the RESTART is load-bearing.**
Three independent derivations give the *same* drawdown-state position multiplier — scale
exposure UP with the cushion (distance above a high-water-mark floor), to ZERO at the
limit: the discrete modulator **M(k) = (d_max − d(k))/(1 − d(k))** [13]; the
model-independent, *provably growth-optimal* fraction **π_α = (1−α)(X/X\*)/[α+(1−α)(X/X\*)]**
[31]; and the convex risk-aversion ramp **γ_t = γ_0·D^max/(D^max − D_t)** [12]. [31]
settles that this family *is* the correct solution to a drawdown floor, not a hack. The
decisive empirical fact [96] (Hsieh, deep read): on **BTC Jan-2020–Sep-2022 with 0.1%
per-trade costs**, drawdown modulation **with a high-water-mark restart** cut max drawdown
**72%→20%** AND held **Sharpe 1.521**, while the *same* controller **without** restart
collapsed to **Sharpe −0.043** (lock-out-then-churn bleeds). It still gave back ~40% of
B&H's upside (+101.82% vs +170.43%) — the honest "protection costs return" number. The
restart is precise: when drawdown comes within ε≈d_max/10 of the floor, **re-base the
high-water mark to current equity and re-enter with a shrunk gain**.

**But every floor guarantee is probabilistic on a gapping asset.** CPPI/drawdown
controllers assume continuous trading; a crypto jump larger than the cushion between bars
breaches the floor [11][13][46]. Crypto's signed jumps dominate vol far more than equity's
[93]; Bitcoin's daily 99% ES ≈ −22% [89] and it has drawn down **76.4%** (Terra/FTX) [83].
The leverage-cycle/liquidation-cascade mechanism [37][56] is the economic "why" — and the
deep vindication of our no-leverage design: the unlevered holder *survives* the spiral that
force-liquidates levered holders. The "probability-one" max-drawdown promise is therefore
**approximate** — disclose it.

**σ̂ choice is settled toward simple.** HAR-RV [66] (daily+weekly+monthly realized vol,
OLS) and EWMA/RiskMetrics λ≈0.94 [85] are the parameter-light workhorses; GARCH/EGARCH
[75] are fine for conditional vol; a 10-day-to-6-month half-life EWMA or GARCH(1,1) is what
the crypto studies actually use [49][90]. Fancier DL/stochastic-vol models forecast the
*point* slightly better but **do not reliably improve the TAIL (VaR/ES) numbers that govern
de-risking** [51][68]. Implied vol is **unreliable for crypto** (thin options) except as a
*combined* signal at weekly+ horizons [95][67]. On daily bars, approximate realized
variance with squared returns or a Garman–Klass/Parkinson range.

**Stops are mostly cost, and a lone trailing stop is value-destroying on a drifting RW.**
Stop-losses lower expected return under a random walk and help only under momentum [8];
even optimized stops beat no-stop in barely >50% of assets [9]; a trailing stop ALONE is
provably suboptimal — pair it with a profit-take [44], and (Glynn–Iglehart, quoted in [44])
it "would be optimal to NEVER use the trailing stop if the stock followed a GBM with a
positive drift." Crypto = huge positive drift + near-random-walk → **do not ship a lone
trailing stop.** If a stop ships at all, attach it to a *trend* pick, tune it to volatility
(ATR-based [94][91]), pair it with a profit-take, and bootstrap-test significance.

---

## 2. Possible solutions / what can be done with this research

1. **Reposition the shipped vol-targeting overlay as a documented risk tool.** Keep the
   mechanics; change the framing, the metrics, and the defaults. Measure each coin's
   per-window return-vol correlation and *report* it; never advertise Sharpe.
2. **Make the vol overlay loose & slow & cost-survivable.** Slow EWMA σ̂ (long half-life),
   a cost-and-vol-scaled no-trade band, de-risk-only, optional downside-deviation trigger.
3. **Build a new drawdown-control overlay** = cushion multiplier M(k) [13] + high-water-mark
   restart [96], operator-set d_max, de-risk-only. Offer static (CPPI-like) vs ratcheting
   (TIPP-like [72]) floor as an explicit choice.
4. **Upgrade the σ̂ input** to a HAR-style multi-horizon realized-vol blend [66] or a
   two-half-life EWMA [85] — a cheap, more-stable de-risk trigger than a single lookback.
5. **Upgrade the risk reporting** the overlays are judged by: report Sortino/Calmar [84],
   CVaR/ES at 90/95/99% [34][82], CDaR [41], skew [54][58], and **median** terminal wealth
   [55] — these surface the overlays' real (risk-shaping) benefit and crypto's asymmetry,
   where Sharpe hides it.
6. **(Lower priority, gated)** A regime-flat de-risk overlay [35] and/or a funding/liquidity
   de-risk trigger [65][37]; an ATR stop+take-profit *only* on a crowned trend pick [94].

Every one of these is a **risk-shaping** lever. None is sold as edge.

---

## 3. Relevance for the project

**Directly load-bearing — this folder's work *is* the overlay surface we already ship.**
The vol-targeting overlay (`VolTargetingOverlay`) and budget-aware sizing
(`FixedFractionSizer`) are live; this research tells us *how to frame, parameterise, and
extend* them honestly.

- **The shipped vol overlay is Kelly with μ held constant.** f ≈ μ/σ² [4][80]; the 1/σ²
  is inverse-variance scaling, so our overlay's de-risking-in-high-vol behaviour is the
  *well-founded* half and the lever-up-in-low-vol half is the *dangerous* half — and it's
  off the table anyway (we are long-only spot, no leverage [2][15]). We can **only de-risk**.
- **The crypto-specific verdict is decisive and on-thesis.** [93]'s reversed leverage
  effect (γ=−0.261) means our overlay should be expected to give the *universal* benefit
  [16] found (thinner tails, lower max drawdown, less vol-of-vol) and **not** a Sharpe gain.
  This is the strongest single-coin-crypto confirmation of our whole "risk-shaping, not
  edge" product thesis — the research and the product agree.
- **The validated discipline is already ours.** Evaluate the overlay by **direct comparison
  vs the un-targeted baseline equity** (our mandatory baseline-divergence e2e —
  `crates/strategy/tests/vol_targeting_overlay_end_to_end.rs`,
  `crates/risk/tests/budget_sizing_divergence_end_to_end.rs`), *never* by a spanning-
  regression alpha that smuggles in future info [2]. Block-bootstrap significance testing
  of a risk overlay vs alternatives [62][63] is exactly our FROZEN gate philosophy —
  independent endorsement.
- **The honest message is the product.** [63]'s **mutual non-dominance** result is our
  thesis stated rigorously from the risk side: a defensive overlay does *not* beat
  buy-and-hold (lower average return, no stochastic dominance) — **but buy-and-hold does
  not dominate it either**, because the overlay delivers downside protection that
  mean/Sharpe comparisons miss. "This drawdown overlay won't beat holding on return, but
  it's not strictly worse — it trades expected return for a smaller, more tolerable left
  tail, which a loss-averse user may rationally prefer." That is **traceable and plausible**
  framing — let the operator's risk preference (and the 1000-path distribution) decide.
- **A drawdown overlay gives the operator a HARD, interpretable promise.** "Never lose more
  than 20% of peak paper-equity" [10][13] is far more useful to a retail user than a Sharpe
  number — and it is the spot, no-leverage move we *can* make (sell down toward cash). It is
  the same convex, capped-downside robustness [5] that makes a sizing rule forgiving of the
  bad μ̂ our thesis says is unavoidable.

**Honest caveat baked in everywhere:** the Sharpe lift is fragile/absent on crypto; the
return COST of protection is *large* (crypto's premium is huge [76]); the floor is
probabilistic (gap risk [89]). We sell the *drawdown/tail reduction* and the *discipline*,
never the alpha.

---

## 4. Advantages for the project

**Drawdown / tail reduction is a UNIVERSAL benefit even when no edge exists** — this is the
core advantage, and it survives every skeptical reading in the ledger.

- **The risk-shaping benefit is the one thing that held up across 60+ assets** [16],
  *independent of the leverage effect's sign* — so it transfers to crypto even though the
  Sharpe gain does not. Crypto-specific studies confirm it: vol management is "crash/tail
  mitigation on momentum" [97][87], CVaR/regime methods "consistently limit drawdowns during
  stress" [73], and dilute-with-cash-to-a-vol-target manages crypto's extreme risk [49][79].
  We can credibly promise **a thinner left tail and a shallower max drawdown** vs holding.
- **A drawdown overlay delivers a guarantee a retail user actually wants and understands.**
  The BTC numbers [96] are concrete and operator-facing: **72%→20% max drawdown** for ~40%
  of upside given up. That is a clear, honest trade we can put in front of the operator and
  let them choose — exactly the "framework for trading with traceable and plausible trading"
  the operator asked for.
- **It differentiates us by honesty.** Most crypto "risk-managed" frameworks over-claim
  (Sharpe 5.72 over 30 days [99]; 2.41 over 36 months across 150 pairs [91]) — precisely
  the multiple-testing illusion the gate exists to deflate. Our advisor reporting *mutual
  non-dominance* [63] and *median* outcomes [55] with deflated statistics is a competitive
  advantage: measured honesty sells.
- **Cash is the only real diversifier we have, and it works.** [79] quantifies "cash is a
  volatility dampener, not a hedge" — a cash sleeve cuts vol and downside *proportionally*.
  For a single-coin advisor that *is* the de-risking lever, and translation-invariance of a
  coherent risk measure [82] formalises it (holding cash m reduces coherent risk by m).
- **The benefit is largest exactly where the user is most exposed.** With one coin there is
  no cross-sectional averaging to dampen its crashes [97], so the de-risking overlay's
  tail-trimming is the *main available defense* — its value is *higher* for us than for a
  diversified book, not lower.
- **Both overlays reuse the existing engine.** The vol overlay is shipped; the drawdown
  overlay is a handful of lines (a position multiplier) needing only the de-risking
  direction — low blast radius (see §6).

---

## 5. Problems and challenges

**Turnover / cost blow-up from chasing the target — the central trap.**
- Closing the feedback loop to track the vol target tightly cost **1105%/yr turnover vs
  93% open-loop** [90]; constant vol scaling *lowered* Sharpe net of turnover [48][28].
  **Mitigation:** slow EWMA + no-trade band [61] + de-risk-only + rare rebalancing. This is
  a HARD design constraint, not a tuning nicety — a twitchy overlay is net-negative on a
  high-cost crypto coin.

**The day-1 baseline-equity-divergence e2e is mandatory (the v3-vol-overlay-noop precedent).**
- This folder's work is *exactly* the kind of overlay the v3-volatility-forecaster-noop bug
  bit: `scale` computed but never applied. Per CLAUDE.md non-negotiables, **every
  sizing-modifier/overlay ships a day-1 e2e asserting its output equity diverges from the
  un-targeted baseline by ≥ a testable epsilon when the decision variable is non-trivial.**
  The vol overlay already has `vol_targeting_overlay_end_to_end.rs`; **the new drawdown
  overlay MUST ship its own divergence e2e on day 1** (pattern:
  `crates/strategy/tests/vol_targeting_overlay_end_to_end.rs`,
  `crates/risk/tests/budget_sizing_divergence_end_to_end.rs`). Note: codegraph reports
  `VolTargetingOverlay` itself has *no covering unit tests* — the e2e is file-level; worth a
  glance that it actually exercises the populated path, not a no-op.

**HARD CONSTRAINTS (named explicitly):**
- **USDT-denominated, `Decimal` not `f64`.** σ̂, EWMA recursions, the cushion multiplier,
  the no-trade band width, and the restart trigger are all arithmetic that must stay in
  `Decimal` (or a clearly-bounded fixed-point) — no `f64` sizing math. `FixedFractionSizer`
  already does this (`rust_decimal::Decimal`); the overlays must match.
- **The budget cap is a HARD limit — sizing may never exceed the simulated budget.**
  `FixedFractionSizer::with_budget_cap` enforces qty·price ≤ budget *even after equity grows*.
  The drawdown overlay only ever *shrinks* exposure (de-risk-only), so it cannot breach the
  cap — but any "re-enter with a shrunk gain" on restart must clamp to the budget cap, and
  the overlay must compose with the sizer, never bypass it.
- **`ui` must NOT depend on strategy/exec/llm/models.** Drawdown/vol-overlay *state* shown
  in the cockpit (e.g. the existing `crates/ui/src/widgets/drawdown_band.rs`) must be fed via
  the existing report/data types, not by `ui` importing the overlay crate. Keep the overlay
  logic in `crates/strategy` / `crates/risk`; surface results through the report layer.
- **Gate/bands FROZEN; paper-only; single-coin long-or-flat.** The overlays are *candidate
  strategies/modifiers* that go *through* the gate; they do not touch the FROZEN classifier
  bands. Short-selling is a separate pre-registered arm — the drawdown/vol overlays here are
  de-risk-toward-cash only.
- **Anchored report SHAs byte-immutable (119/119).** Any new backtest report these overlays
  emit is *new* anchors (additive); do not edit existing anchored reports.

**Estimation / regime fragility.**
- The OOS failure of vol-timing was *structural instability* (parameter breaks) [2]; crypto
  is regime-shifty [78][83] and its tail index is time-varying [89]. Any trained overlay
  parameter (d_max, gain, λ, band width) goes stale → favour **simple, slow, few-parameter**
  rules and re-baking on regime events, and **gate the chosen parameters with PBO/DSR**
  [69][70] (a d_max/γ\* tuned on one window is an overfitting surface — [96]'s own caveat).

**Gap risk breaks the floor guarantee.**
- The "probability-one" max-drawdown promise is idealised; a crypto jump > cushion between
  bars breaches it [11][13][46], and [96] itself flags the instantaneous-restart assumption
  "may not hold." **Mitigation:** conservative multiplier/gain, disclose the floor is
  *probabilistic*, stress-test against a 70%+ drawdown slice (Terra/FTX [83]).

**The return cost is large and must be shown.**
- Crypto's premium is ~66%/yr historically [76]; de-risking forgoes a lot of it. [96] gave
  back ~40% of B&H upside; [13]'s Tesla example: 1.005× vs 1.136×. **The advisor must show
  the operator the give-up explicitly** (mutual non-dominance framing [63]), not bury it.

**What to be skeptical of:** any backtest where the overlay "beats" buy-and-hold on Sharpe
(check it's not the mechanical vol-scaling reshaping [86], and that it survives costs [48]
and deflation [69][70]); negative-skew smooth-Sharpe curves (likely an unexploded tail bomb
[54]); single-window crypto Sharpes ≥ 2 [91][99] (regime luck).

---

## 6. Concrete next steps / candidate work items

Priorities are **relative to the P0 gate upgrade in `SYNTHESIS.md`** (DSR/PBO/N_eff/MinBTL),
which is the program's highest-leverage action and gates everything below. These overlay
items are **mid-priority (P1)** — honestly-gated experiments expected ≈ null on Sharpe but
genuinely useful on drawdown.

### P1-A — Reposition the vol-targeting overlay as a risk tool (loose & slow)
- **What:** Reframe + reparameterise the shipped overlay. Default to a **slow EWMA σ̂**
  (long half-life, e.g. ~126-day equiv [90], slower than RiskMetrics 0.94 [85]); add a
  **cost-and-vol-scaled no-trade band** [61] so it does not re-size every bar;
  **de-risk-only** (clamp the multiplier ≤ 1, which a long-only no-leverage account is
  anyway [90]); optional **downside-deviation trigger** [59][84]. Compute and report the
  per-window **return-vol correlation** so the operator sees whether a Sharpe gain is even
  mechanistically possible [93].
- **Where:** `crates/strategy/src/vol_targeting_overlay.rs` (`VolTargetingConfig` gains
  band-width + half-life + downside-trigger knobs); report layer for the ρ(ret,vol) readout.
- **Gate:** existing baseline-divergence e2e stays green; bake off vs B&H under the FROZEN
  gate; expect drawdown/tail reduction, **not** Sharpe. Pairs with `SYNTHESIS.md` item 14.
- **Priority: P1.** Reframe + cost-hardening of a *shipped* surface — low blast radius.

### P1-B — Drawdown-control overlay with high-water-mark restart *(highest-value new build)*
- **What:** A new de-risk-only overlay: position multiplier **M(k) = (d_max − d(k))/(1 − d(k))**
  [13] (operator sets `d_max` = "max % of peak paper-equity I'll tolerate losing") **plus the
  high-water-mark RESTART** [96] — when d(k)+ε > d_max (ε≈d_max/10), re-base the HWM to current
  equity and re-enter with a shrunk gain K = γ\*·α·e^(−k₀/N)·M. The restart is **not optional**:
  without it the controller's Sharpe collapsed to −0.043 net of 0.1% costs [96]. Offer the
  operator **static floor (CPPI-like, more upside)** vs **ratcheting floor (TIPP-like [72],
  protects profits, costs upside)**. Disclose the floor is *probabilistic* (gap risk
  [46][89]) and that it costs ~tens-of-% of upside (BTC: ~40% of B&H return for the cut [96]).
- **Where:** new `crates/strategy/src/drawdown_control_overlay.rs` (mirror the
  `VolTargetingOverlay`/`Strategy` shape; compose with `FixedFractionSizer`, never bypass the
  budget cap). New cockpit readout can reuse the existing `crates/ui/src/widgets/drawdown_band.rs`
  via the report layer (no `ui`→strategy dependency).
- **Gate / day-1 requirement:** **ship a baseline-equity-divergence e2e on day 1** (the
  v3-vol-overlay-noop precedent) — assert overlay equity diverges from un-targeted baseline by
  ≥ epsilon when a real drawdown occurs. Bake off vs B&H under the FROZEN gate; run **PBO/DSR**
  on the chosen `d_max`/gain [69][70]; stress-test against a 70%+ drawdown slice [83].
- **Priority: P1 (do after the vol reframe; this is the single most actionable new overlay in
  the topic).** All math in `Decimal`; de-risk-only so it cannot breach the budget cap. Pairs
  with `SYNTHESIS.md` item 15.

### P1-C — σ̂ upgrade: multi-horizon realized vol (HAR-style)
- **What:** Replace/augment the single-lookback vol estimate feeding both overlays with a
  **HAR-RV blend** (daily+weekly+monthly trailing realized variance, OLS) [66] or a
  **two-half-life EWMA** [85] (poor-man's HAR). On daily bars approximate realized variance
  with squared returns or Garman–Klass/Parkinson range [51][49]. Do **not** over-engineer
  (DL/SV don't reliably improve the TAIL [51][68]); keep implied vol out of the daily trigger
  ([67]; combine only at weekly+ [95]).
- **Where:** a vol-estimator module shared by `vol_targeting_overlay.rs` and the new drawdown
  overlay; there is already a GARCH vol-target scenario at
  `crates/backtest/src/scenarios/garch_vol_target_overlay.rs` to align with.
- **Priority: P1/P2.** Cheap, improves both overlays; can land with either.

### P2-D — Risk-reporting upgrade (surfaces the overlays' real benefit)
- **What:** Add **Sortino + Calmar** [84], **CVaR/ES at 90/95/99%** [34][82] (a trivial readout
  from the bootstrap loss distribution), **CDaR** [41], **skew** [54][58], and **median
  terminal wealth** [55] to the bake-off/forward report. Optionally a spectral / risk-aversion-
  weighted tail number [98]. Report **CVaR, not VaR** (VaR is non-coherent [82]).
- **Where:** the bake-off ranking report + forward-plan report (read from the existing
  bootstrap loss distribution in `crates/backtest/src/bakeoff/`); **additive — does not touch
  the FROZEN classifier bands.**
- **Priority: P2.** Makes the risk-shaping benefit *visible* (it's invisible on Sharpe alone).

### P2-E — Regime-flat / funding-liquidity de-risk trigger (gated experiment)
- **What:** A bull/bear regime-flat overlay [35] (jump-model de-risk-to-cash with an explicit
  switching penalty, OOS-CV params, detection-lag model) and/or a **downside trigger augmented
  by funding/liquidity stress** [65][37][56] (crypto-native crash signal beyond realized vol).
- **Where:** new candidate overlay; its own **day-1 divergence e2e**; event-driven re-baking,
  not calendar.
- **Priority: P2.** Pairs with `SYNTHESIS.md` item 18; expect drawdown benefit > Sharpe
  benefit; cross-reference crypto-market-structure findings before trusting funding signals.

### What NOT to do
- **No tight vol-target tracking / feedback control** for our cost-charged crypto setting —
  the authors of the feedback-control paper themselves prefer open-loop here [90]. Loose &
  slow, banded, de-risk-only.
- **No lone trailing stop on buy-and-hold** — value-destroying on a drifting random walk
  [8][44]. If a stop ships, it's ATR-scaled, paired with a profit-take, on a crowned *trend*
  pick only [94][91], bootstrap-tested [9].
- **No RL / heavy-ML sizing or vol model** — RL needed ~8,000 years of daily data even on
  clean sim [50]; DL vol doesn't improve the tail [51]. Closed-form, parameter-light only.
- **No Sharpe promise on the overlays.** Ever. Risk-shaping only [16][93].
- **No options/put tail hedge** — out of scope (spot-only) *and* it bleeds (−0.61%/yr) [53];
  the realistic "tail hedge" is the defensive de-risking of a trend/regime rule.

### Effort & blast radius (summary)
| Item | Where | Nature | Blast radius |
|---|---|---|---|
| P1-A vol reframe (loose+slow, band, downside) | `strategy/src/vol_targeting_overlay.rs` + report | reparameterise shipped overlay | low (2 callers in `strategy/src/lib.rs`) |
| P1-B drawdown overlay + restart | new `strategy/src/drawdown_control_overlay.rs` + day-1 e2e | new de-risk-only overlay | low–med (new file; composes with sizer) |
| P1-C σ̂ HAR/EWMA | shared vol-estimator module | input upgrade | low |
| P2-D risk reporting | bake-off + forward report | additive metrics | low (no FROZEN-band touch) |
| P2-E regime/funding de-risk | new candidate + day-1 e2e | gated experiment | med |

---

## 7. Open questions for analyst & architect

1. **Does our shipped vol overlay actually lower drawdown/variance vs buy-and-hold on real
   coin windows, even when Sharpe is unchanged?** This is the direct test that validates the
   reposition [16][93]. (Hypothesis: yes on drawdown/tail, no on net Sharpe.)
2. **Per-coin, is the realized leverage effect positive (equity-like) or inverse [93]?** Does
   a positive-leverage-effect coin/window actually get a Sharpe bump from the overlay while an
   inverse one doesn't? Confirm the mechanism on *our* data before promising anything.
3. **Drawdown modulation + restart [13][96] vs static CPPI-style floor vs B&H** on real coin
   windows with our cost model: does the restart actually improve net-of-cost terminal wealth,
   and **how often does the floor get breached by gaps** (gap-frequency)?
4. **What no-trade-band width [61] is the break-even** on a high-cost crypto coin — i.e. where
   does the vol overlay flip from net-negative [48] to net-neutral? (Calibrate band ∝ cost & vol.)
5. **Does triggering on DOWNSIDE deviation [59][84] cut drawdown more than total-vol scaling**
   on the same (coin,window), net of costs?
6. **Static floor (CPPI) vs ratcheting floor (TIPP) [72] as the default operator choice** —
   which framing does a retail user understand and prefer, given TIPP protects profits but
   caps upside? (Product/UX decision for the analyst.)
7. **How do we surface the "protection costs return" trade-off** (mutual non-dominance [63],
   ~40% upside give-up [96]) so it's *traceable and plausible*, not buried? (Report design.)
8. **What is the right d_max / gain default**, and how do we gate it against overfitting
   [69][70] given [96]'s parameters were tuned on a single favourable window? (Architect:
   wire PBO/DSR around the overlay's tunable knobs.)
