---
slug: robustness-decision-rule-2026-05-30
date: 2026-05-30
authors: analyst
status: pre-registered
tags: [robustness, monte-carlo, decision-rule, pre-registration, presenter-backing, scientific-integrity]
related:
  - spec/strategy-robustness-harness/feature.md
  - spec/monte-carlo-bootstrap-path-generator/feature.md
  - spec/dev-notes/strategy-robustness-monte-carlo-direction-2026-05-29.md
  - spec/dev-notes/monte-carlo-robustness-architecture-readiness-2026-05-29.md
  - spec/architecture/adr/0051-monte-carlo-determinism-and-distribution-report-anchoring.md
  - spec/dev-notes/v3-vol-overlay-noop-discovery-2026-05-22.md
  - spec/product.md
---

# Robustness decision rule — pre-registered (C1+C2 Monte-Carlo lane)

> **PRE-REGISTRATION NOTICE — registered 2026-05-30, BEFORE C2 emits any
> number.** C2 (`strategy-robustness-harness`) is `arch-done` / in-flight; at
> the time of writing it has produced **no distribution summary**. This note
> defines how to *read* that distribution — the go/no-go ruler — so the eventual
> verdict is pre-registered, not post-hoc rationalized. This is the
> scientific-integrity discipline and the direct meta-lesson of the
> v3-vol-overlay no-op era (`v3-vol-overlay-noop-discovery-2026-05-22.md`): a
> number that is interpreted only *after* it is seen can be talked into meaning
> whatever the author wants. **The bands below are frozen now; C2's output is
> scored against them, not the reverse.** If a band threshold is ever changed
> after seeing C2's numbers, that change MUST be logged in this changelog with
> the before/after value and the operator's explicit signoff.

---

## 0. TL;DR for the presenter (lift this into the operator deck verbatim)

C2 will report, for v1 cross-sectional momentum at a single fixed θ\* over
**N=500 shared-index block-bootstrap paths of 2023-FY real Binance returns**:
Sharpe **p5 / p25 / p50 / p75 / p95**, **max-drawdown tail (p50 + p95)**,
**prob-of-loss** `P(final_equity < initial)`, and **P(Sharpe>0) / P(Sharpe>1)**.

Read those five primary numbers against these bands. **The verdict is a
read, not an arithmetic gate — the operator decides; this is the ruler.**

| Signal | ROBUST (edge is real) | MARGINAL (inconclusive) | FRAGILE (one lucky path) |
|---|---|---|---|
| **p5 Sharpe** (tail floor) | **≥ +0.5** | `0.0 … +0.5` | **< 0** (the tail loses money) |
| **p50 Sharpe** (central) | ≥ 1.0 | `0.5 … 1.0` | < 0.5 |
| **p95−p5 Sharpe spread** (dispersion) | ≤ ~1.5 | `~1.5 … ~2.5` | > ~2.5 (wildly path-dependent) |
| **prob-of-loss** `P(equity<start)` | **≤ 15%** | `15% … 35%` | **> 35%** (coin-flip-ish) |
| **P(Sharpe > 1.0)** (gate fraction) | ≥ 60% | `35% … 60%` | < 35% |
| **p95 max-drawdown tail** | ≤ ~50% | `~50% … ~70%` | **> ~70%** (≈ the single-path 73%) |
| **p50 vs single-real-path Sharpe** | p50 within ~0.3 of the real path (real path is *typical*) | — | real path sits **above p75** (real path was *favourable*; ensemble is worse) |

**Composite read (which band wins):** take the **worst** band any single primary
signal lands in — robustness is a weakest-link property, not an average.
A strategy is **ROBUST only if p5 Sharpe ≥ 0 AND prob-of-loss ≤ 15% AND the p95
drawdown tail is one the operator can stomach.** Any FRAGILE primary signal ⇒
FRAGILE verdict regardless of how good the median looks. (Rationale: §3.)

**One-line operator framing:** *"A single backtest told us the median. This tells
us the tail — and the tail, on a fair crash-like adversary, is where money is
actually lost."*

---

## 1. Why the distribution beats the single-path backtest (the epistemic delta)

A single-path backtest reports `f(strategy, θ*, the_one_2023_path)` — one Sharpe,
one drawdown, one final equity. It **cannot distinguish** a strategy that is
genuinely Sharpe-1.4 from one that is 1.4-only-on-that-exact-ordering-of-2023.
The load-bearing example on disk: v1 momentum shows **73% max-drawdown on the
single 2023-FY real path** (`strategic-reset-2026-05-23.md` §4.2). Is 73% the
typical bad day, or the worst the resampled histories ever produce, or
optimistic? A point estimate is silent on this.

The Monte-Carlo distribution answers exactly the questions the point estimate
cannot:

| Question the operator actually has | Point estimate | Distribution (C2) |
|---|---|---|
| "Could this have lost money on a plausibly different 2023?" | silent | **prob-of-loss** = mass below break-even |
| "Is the Sharpe a property of the *strategy* or of the *path ordering*?" | silent | **p5↔p95 spread** — tight ⇒ strategy property; wide ⇒ path artifact |
| "What is the *bad-case* drawdown, not the one-sample drawdown?" | one number | **p95 max-DD tail** — the number the `paper→live` gate should use |
| "Was the real 2023 path lucky, typical, or unlucky?" | unknowable | **p50 vs single-real-path** — where the real path sits in the ensemble |
| "How often does it clear the bar I care about?" | binary on one path | **P(Sharpe>1)** — fraction of resampled histories that clear it |

The distinction is categorical and is the whole reframe: this is **uncertainty
quantification of an already-shipped strategy**, NOT prediction. No alpha is
claimed from the synthetic data (contrast the three retired
alpha-engine-by-prediction bets — TCN/PatchTST, GARCH-σ, LLM-forecaster). The
ruler measures variance of outcome under input perturbation; it does not forecast
returns. (See `product.md` § Pillar stack — core pillar 2.)

---

## 2. Why surviving the p5 tail is *meaningful* here — the shared-index null (FP-C1.5)

The bands above put decisive weight on the **p5 tail** (the 5th-percentile,
bad-case path). That weight is only justified if the bad-case path is a **fair
adversary** — a plausible bad market, not an artificially easy or artificially
impossible one. C1's ratified design makes it fair, and this is the load-bearing
methodological link the presenter must state:

- The bootstrap is **shared-index** (C1 Q-MCB-2 = Option A, RATIFIED; D-C1.3).
  ONE resampling index sequence is drawn and applied to **all symbols
  simultaneously**, so contemporaneous cross-symbol co-movement on each selected
  real timestamp is **preserved** in every resampled path.
- The tester confirmed this is a **genuine guard, not a no-op**: FP-C1.5 shows
  per-symbol-*independent* resampling collapses cross-symbol correlation to
  **−0.079** (decorrelated), whereas shared-index retains the source co-movement.
- **Consequence for the ruler:** because co-movement is preserved, a resampled
  path can splice together the real crash-like return blocks **across the whole
  universe at once** — the diversification that a cross-sectional strategy
  *appears* to enjoy does **not** rescue it in a synthetic crash, exactly as it
  would not in a real one. A per-symbol-independent bootstrap (the rejected null)
  would manufacture crash-time diversification the real market never offers,
  making the strategy look **more robust than it is** and understating the p95
  drawdown tail. Shared-index does not.

Therefore the **p5 Sharpe and the p95 drawdown tail are fair adversaries** for a
cross-sectional momentum strategy: surviving them means surviving plausible joint
adverse moves, not a decorrelated toy market. **This is why "survive the p5 tail"
is the headline robustness criterion and not a strawman.** If C2's report ever
prints `bootstrap_mode: per-symbol-independent` instead of `shared-index`, the
tail numbers are NOT a fair adversary and this entire decision rule is **void
until re-run under shared-index** — the presenter must check that field first.

---

## 3. The bands, defended (one paragraph each — for the deck's appendix)

> Bands are **decision bands the operator reads, not hard pass/fail gates.** They
> are deliberately round numbers: the goal is a *ruler with legible gradations*,
> not false precision. Magnitudes are grounded in the standard
> robustness-testing literature already cited in the lane's direction note
> (PickMyTrade 2026 robustness guide; Build Alpha; López de Prado on overfit
> adjustment) and in the project's own single-path priors.

### 3.1 p5 Sharpe — the tail floor (the single most important number)

The 5th-percentile Sharpe is the bad-case outcome on a fair adversary (§2). **If
p5 < 0, the strategy loses risk-adjusted money in its bad case** — that is the
FRAGILE line. ROBUST requires p5 **≥ +0.5**: not merely "doesn't lose in the
tail" but "the tail still earns a respectable risk-adjusted return." The
`0 … +0.5` MARGINAL band is "the tail breaks even but does not earn" —
inconclusive, operator's call. This is the number that most directly separates a
genuine edge from a lucky ordering: a curve-fit strategy has a high median and a
**negative** p5 (it works only when the path cooperates).

### 3.2 p50 Sharpe — the central tendency (necessary, not sufficient)

The median is what a single-path backtest *approximates* if the real path is
typical. ROBUST ≥ 1.0 mirrors the existing `paper→live` single-path bar — but the
median **alone** is explicitly demoted from sufficient to necessary: a high
median with a negative p5 is the curve-fit signature. The median's job in the
composite is to confirm the strategy is centrally worth running; the p5/tail do
the real discriminating.

### 3.3 p95−p5 Sharpe spread — dispersion (is the outcome a strategy property?)

A **tight** spread means the outcome is a property of the *strategy* (it earns
similar Sharpe whatever the resampled ordering) → robust. A **wide** spread means
the outcome is a property of the *path* (it swings wildly with the resample) →
the Sharpe was an artifact of one ordering. The `≤ ~1.5 / ~1.5–2.5 / > ~2.5`
bands are heuristic ranges to be **re-centred once C2's first real spread is
seen** (logged per the pre-registration notice if changed). The *sign* of the
read — narrow good, wide bad — is frozen and not subject to revision.

### 3.4 prob-of-loss `P(final_equity < initial)` — the coin-flip test

The fraction of resampled histories that end below the starting equity. **> 35%
is approaching a coin-flip** — more than a third of plausible 2023s lose money
outright → FRAGILE. ≤ 15% (a roughly one-in-seven bad-history rate) is the ROBUST
line, intentionally aligned in spirit with the literature's PBO < 15% overfit
threshold (note: prob-of-loss is NOT PBO — see §5 — but 15% is the same
"acceptable adverse rate" intuition). The `15–35%` band is the inconclusive
middle.

### 3.5 P(Sharpe > 1.0) — the gate-clearing fraction

The fraction of paths that clear the project's existing Sharpe-1.0 promotion bar.
If only a minority of resampled histories clear it (< 35%), the single-path "it
cleared 1.0" was likely the lucky minority → FRAGILE. ≥ 60% (a clear majority of
plausible histories clear the bar) is ROBUST. This is the distribution-valued
version of the existing binary `paper→live` Sharpe gate.

### 3.6 p95 max-drawdown tail — the headline risk number

The direction note (§3.1) and both feature briefs are explicit: **the p95
max-drawdown tail, not the single-path drawdown, is the number the `paper→live`
gate should actually use.** A single path gave 73% (§1); the question is whether
73% is the tail or the typical. **> ~70% (i.e. the tail is no better than the
single-path 73%)** is FRAGILE — the drawdown risk is real and pervasive across
resampled histories, not a one-sample artifact. ≤ ~50% is the ROBUST line. The
operator's actual stomach for drawdown is the final arbiter here; the band gives
them the tail to judge instead of a single sample. **This is the number most
likely to be the binding constraint for v1 momentum** given its known drawdown
profile.

### 3.7 p50 vs the single-real-path Sharpe — was the real 2023 lucky?

A cross-check, not a primary gate. Compare the ensemble median to the single
deterministic real-2023-path Sharpe (the baseline C2's R-NR.6a divergence test
already computes). Three readings: (a) p50 ≈ real path ⇒ the real 2023 was a
**typical** draw, the ensemble corroborates the single-path story; (b) real path
**above p75** ⇒ the real 2023 was **favourable**, the single-path backtest
flattered the strategy and the ensemble is the more honest picture; (c) real path
**below p25** ⇒ the real 2023 was unlucky, the strategy may be better than the
single path suggested. Reading (b) is the one that should most change the
operator's mind relative to looking at the single backtest alone — and it is the
reading the whole robustness lane exists to surface.

---

## 4. The composite verdict procedure (frozen)

1. **Pre-flight check (void-if-fail):** confirm C2's report body prints
   `generator: block-bootstrap-real` (NOT `gbm-smoke` — a GBM run has no fat
   tails and understates every tail number; C2 K4) AND
   `bootstrap_mode: shared-index` (NOT per-symbol-independent — §2; the tail is
   not a fair adversary otherwise). If either fails, **the verdict is void** and
   the run is re-done; do not score voided output against the bands.
2. **Score each of the 7 signals** into ROBUST / MARGINAL / FRAGILE per §0.
3. **Composite = the worst band any *primary* signal lands in.** Primary signals
   are the five hard ones: p5 Sharpe, prob-of-loss, p95 drawdown tail, p50
   Sharpe, P(Sharpe>1). Spread (3.3) and p50-vs-real-path (3.7) are
   **interpretive** — they inform the read and the narration but do not by
   themselves force a FRAGILE verdict. (Rationale: robustness is a weakest-link
   property — a strategy that is excellent on 4 signals and loses money in the p5
   tail is not robust; the tail is where capital dies.)
4. **ROBUST** ⇒ the edge survives a fair resampled adversary; the strategy is a
   credible `paper→live` candidate on the robustness axis (other gates — cost,
   30-day paper, PM signoff — still apply independently).
   **MARGINAL** ⇒ inconclusive; the operator's call, and a candidate for the
   parameter-sweep follow-on (C3 plateau-vs-peak) to see if a nearby θ is
   cleaner, or for a larger N to tighten the tail estimate.
   **FRAGILE** ⇒ the single-path Sharpe was substantially a lucky ordering; do
   NOT promote on the strength of the backtest; the honest read is the strategy's
   edge is not demonstrated robust.
5. **Operator override is explicit and logged.** The bands are a ruler; if the
   operator promotes a MARGINAL/FRAGILE strategy anyway (or holds a ROBUST one),
   that is their prerogative and is recorded with rationale. The point of
   pre-registration is that the *band* was fixed first — the override is then a
   visible, accountable decision rather than a moved goalpost.

---

## 5. Scope of THIS rule — what C2 v0.1.0 does and does NOT measure

To keep the ruler honest about its own reach (so the presenter does not overclaim):

- **In scope for C2 v0.1.0 (the numbers these bands score):** Sharpe percentiles,
  max-drawdown tail, prob-of-loss, P(Sharpe>0/>1) — for **one strategy family
  (v1 cross-sectional momentum) at a single fixed θ\*** over N=500 shared-index
  block-bootstrap paths of 2023-FY real Binance returns. One anchored
  distribution-summary report under namespace `mc-robustness-2026-06`.
- **NOT in scope for C2 v0.1.0 (do NOT cite these as if the report contains
  them):**
  - **Parameter sensitivity / plateau-vs-peak** (is θ\* a robust plateau or a
    sharp curve-fit peak?). That is the **C3 param-sweep** Queue follow-on; C2's
    parameter-stability section is an explicit **stub/N-A at v0.1.0**. This rule
    therefore judges *path* robustness, not *parameter* robustness — a strategy
    can pass this rule and still be a sharp peak in θ-space. State that limit.
  - **PBO (Probability of Backtest Overfitting) and Deflated Sharpe.** These are
    the **C5 CPCV** Queue follow-on (López de Prado guards). C2 does NOT emit
    them. The §3.4 "15% intuition" borrows the *acceptable-adverse-rate* spirit
    of PBO<15% but prob-of-loss is a different quantity; do not present
    prob-of-loss as PBO.
  - **Out-of-history regimes.** Block bootstrap resamples *real* return blocks —
    it cannot synthesize a regime 2023 never contained (a generative model would;
    those are deferred, rank 5–6 in the direction note, for good overfit-risk
    reasons). The ruler scores robustness *to resampled real history*, not to
    hypothetical unseen regimes. A genuinely novel regime (a 2025-style event
    absent from the source year) is outside what this distribution can speak to.
- **Determinism caveat (inherited, not re-litigated):** the anchored summary is
  byte-identical **on the Apple-Silicon canonical box** (ADR-0051 D5 / ADR-0043
  precedent); cross-platform parity is NOT contracted. This does not affect the
  *decision* bands (the percentiles are the percentiles); it affects only the
  anchor-reproducibility gate, which is C2/tester scope.

---

## 6. Assumptions (challengeable by operator / architect)

1. The bands' *magnitudes* are first-pass, literature-and-prior-grounded round
   numbers; their *signs and orderings* (negative p5 = bad, wide spread = bad,
   high prob-of-loss = bad, fat drawdown tail = bad) are the load-bearing,
   non-negotiable part. If C2's first real distribution makes a magnitude
   obviously mis-calibrated, re-centring a threshold is allowed **once, logged,
   with operator signoff** — the pre-registration is about preventing *silent
   post-hoc* goalpost-moving, not about banning any calibration ever.
2. The composite "worst primary band wins" rule encodes that robustness is a
   weakest-link property. An operator who prefers an averaged or weighted read
   may say so; the weakest-link default is the conservative, capital-preserving
   choice and is the one pre-registered.
3. The shared-index null (§2) is assumed wired and genuine on the strength of the
   tester's FP-C1.5 PASS (corr collapse to −0.079 under the rejected null). If a
   future C2 run prints a different `bootstrap_mode`, this rule is void for that
   run (§4 step 1).
4. N=500 is assumed sufficient for stable p5/p95 tail estimates (C2 Q-RH-1
   ratified; N=200 fallback). At smaller N the tail percentiles are noisier and
   the FRAGILE/MARGINAL boundary should be read with more latitude.
5. This rule governs the **path-robustness** axis only; cost gates, the 30-day
   paper requirement, and PM signoff remain independent `paper→live` criteria
   (`product.md` § Strategy lifecycle).

---

## Changelog

- 2026-05-30 (analyst, monte-carlo-robustness-lane): pre-registered the
  robustness decision rule **before** C2 emitted any distribution number (C2 was
  `arch-done`/in-flight at authoring). Defined 7 read-signals (p5/p50 Sharpe,
  p95−p5 spread, prob-of-loss, P(Sharpe>1), p95 max-DD tail, p50-vs-single-real-
  path) as ROBUST/MARGINAL/FRAGILE decision bands; froze a composite
  weakest-link verdict procedure with an explicit void-if-`gbm-smoke`-or-per-
  symbol-independent pre-flight; tied the p5/p95-tail weighting to the
  shared-index null (FP-C1.5, corr-collapse −0.079) so the tail is a fair
  crash-like adversary; bounded the rule's scope (path-robustness only — NOT
  C3 parameter-sweep, NOT C5 PBO/Deflated-Sharpe, NOT out-of-history regimes).
  Structured §0 + §1–§3 for direct lift into the C1+C2 presenter operator deck.
  Registered as the meta-lesson of the v3-vol-overlay no-op era: the ruler is
  fixed first, the measurement scored against it.
