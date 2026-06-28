# Application — Forecasting vs. Simple Baselines & the Significance Layer

> **Audience:** analyst + architect, planning the next big steps.
> **Source:** `research/deep-learning/knowledge.md` (100-paper synthesis) + the cited
> `deep-learning[N]` ledger entries in `research/deep-learning/papers.md` + `research/SYNTHESIS.md`.
> **Scope of this file:** the *forecasting* arc (deep transformers vs. trivial linear
> baselines) and the *significance/reproducibility* arc (an attractive backtest that
> a paired-t cannot reject; false-positive-machine numbers; statistical power). These
> two arcs belong together because they share ONE through-line and ONE conclusion:
> **does the deep model beat a simple baseline OOS, net of costs, reproducibly AND
> significantly? Across the full-text evidence, no.** That negative result is what
> *motivates the significance/DSR gate layer* and tells us *what NOT to build*.
> The deep-RL / hedging / adversarial-robustness material lives in the sibling file
> `application-deep-rl-and-hedging.md`.

> **Our app (ground every claim against this):** a Rust **single-coin crypto
> investment advisor** — paper/sim only, NOT advice, NOT live. Journey: pick ONE coin
> + budget → **bake off EVERY strategy** → **rank** under a FROZEN robustness gate
> (1000-path moving-block bootstrap; FRAGILE ⇒ cannot be crowned; **buy-and-hold is
> always the benchmark + exempt**) → forward rule-based plan → **watch it paper-trade**.
> Validated thesis: **no active strategy robustly beats buy-and-hold net of costs.**
> The product sells **measured honesty** — operator goal: "a framework for trading
> with traceable and plausible trading."

---

## 1. Summary of the research

This corpus is, for our purposes, the strongest *negative* evidence in the whole
research program. It splits into two mutually reinforcing arcs.

**Arc A — deep forecasters do not beat trivial baselines on the data we have.**
The transformer-forecasting line (Informer `deep-learning[6]`, Autoformer
`deep-learning[7]`, FEDformer `deep-learning[85]`) was refuted on its own benchmarks
by *embarrassingly simple* models: one-layer linear DLinear/NLinear beats those
transformers by **20–50% across 360+ comparisons on nine datasets**, and on the one
*financial* set (Exchange-Rate) the gap is largest — DLinear 0.081 vs FEDformer 0.148
(~45%), where even a naive *repeat-last-value* baseline beats the transformers
`deep-learning[8]`. The decisive mechanism experiment: **randomly shuffling the input
sequence barely changes transformer error (~0%) but degrades DLinear 27.26%** — i.e.
the transformers were *not using the temporal ordering they claim to model*
`deep-learning[8]`. The pattern repeats across the field's own newer, *simpler*
forecasters: N-HiTS `deep-learning[86]` (univariate, ~20% better AND 50× faster),
TSMixer `deep-learning[67]` (an all-MLP whose ablation shows the cross-variate
machinery adds ~nothing on univariate-ish data), FreTS `deep-learning[92]`, TimeMixer
`deep-learning[98]`. The transformer "comebacks" win *only* via structure a single
coin lacks: PatchTST `deep-learning[22]` recovers a single-digit-% edge after careful
design (on physics/utility data, never cost-net P&L); iTransformer `deep-learning[88]`
wins by inverting to **cross-variate** attention and **ties one-line DLinear** once the
variate count is low (21-dim Weather); TimesNet `deep-learning[87]` wins via
**periodicity**. A generic TCN `deep-learning[90]` beats LSTM/GRU, and GRU ≈ LSTM
`deep-learning[55]`. The rigorous asset-pricing benchmark says the same from the other
direction: ML's genuine edge is **cross-sectional, nonlinear, and tiny in absolute R²**
(sub-1% monthly), realized only across thousands of names — irrelevant to one coin
`deep-learning[24][32]`.

**Arc B — accuracy ≠ alpha, and an attractive backtest ≠ a significant edge.**
Even when a model forecasts well, the trading claim collapses. Directional accuracy,
price-level RMSE, and image-AUC are reported *without* cost-net P&L throughout
`deep-learning[3][4][11][33][40][55][56][74][82][89]`; F1 collapses once the decision
threshold equals the spread `deep-learning[40]`; a low RMSE on a near-random-walk
*price level* is the easy, meaningless metric `deep-learning[55][56][74]`. The single
sharpest lesson is statistical: a cost-aware (0.1%), 23-year walk-forward LSTM-ARIMA
hybrid **appears to beat buy-and-hold on every risk-adjusted ratio and halves
drawdown, yet a paired t-test returns p = 0.24–0.92 — it cannot reject "no real edge"**
`deep-learning[99]`. A rigorous 34-fold equity walk-forward had only **~12% statistical
power at the observed effect (d=0.17) and would need ~540 folds for 80%**
`deep-learning[69]`. And the multiplicity teeth: under a pure-noise null, **at K=100
trials the nominal 1.96 t-threshold is wrong 99.9% of the time**, the in-sample/OOS
magnitude gap ΔZ is itself the diagnostic, correlated bake-off variants do *not* escape
(the right count is the spectral `K_eff = K²/‖Σ‖_F²`), and any thresholding rule
re-inflates it ~2.68× `deep-learning[39]`. Per-coin / single-window "wins" are
regime- or asset-conditional luck `deep-learning[100][63]`.

**Net:** the architecture treadmill (LSTM → CNN → Transformer → Mamba → KAN) keeps
arriving with the same methodological holes; the honest comparative evidence keeps
concluding *simple ≥ fancy* and *the deep edge is cross-variate breadth a single coin
does not have*. This is direct, primary-source vindication of our skepticism — and it
points at exactly one piece of *constructive* engineering: a **significance / selection-
bias layer** on the bake-off.

---

## 2. Possible solutions / what can be done with this research

1. **Build the significance / selection-bias layer the literature is screaming for.**
   This is the one *additive*, high-value, code-able outcome of the whole topic — and
   it is already the program-wide **P0** in `SYNTHESIS.md §2`. It turns "we ran a
   1000-path bootstrap" into "we ran a bootstrap AND corrected for the multiple-testing
   bias of crowning the best-of-N." Components (all derivable from the per-strategy
   return matrix the bake-off already stores): effective trial count `N_eff`, Deflated
   Sharpe Ratio (DSR), PBO via CSCV, a MinBTL pre-flight veto, SPA-style studentization,
   and a ΔZ / Backtest-Inflation-Factor readout. (`deep-learning[39][99][69]` are the
   DL-side motivation; the exact formulas converge with `backtesting`/`strategies`.)

2. **Add a null-data falsification CI test.** Run the *entire* bake-off + ranking on
   each of five structural nulls — white noise, regime-switching vol, bid-ask bounce,
   zero-alpha factor, GARCH(1,1) — and assert it crowns **nothing** above buy-and-hold
   `deep-learning[39]`. Because the paper proves a naive search produces a "winner"
   under the null with near-certainty, a pipeline that *refuses* to crown noise is
   positive evidence it is leak-free; one that crowns noise has a demonstrable bug.

3. **Keep DL forecasters OFF the alpha rail; if a model is ever wanted, fix the rules
   by this literature.** Beat a DLinear-style linear baseline AND buy-and-hold *first*
   `deep-learning[8][22]`; prefer a simple MLP/decomposition `deep-learning[67][86][92][98]`
   or a causal TCN `deep-learning[90]` over a transformer; forecast a *distribution*,
   not a point `deep-learning[5]`; report a cost-sensitivity curve and expect the edge
   to vanish within a few bps `deep-learning[15][52]`; report across many seeds + paths
   with a **significance measure AND statistical power** `deep-learning[47][69][99]`.
   The honest expected result of every one of these probes is "does not beat hold."

4. **Adopt the cheap, interpretable forecasting primitives, not the heavy models.**
   Trend/seasonal **decomposition** `deep-learning[7][8][68]` and a one-line linear
   "is there ANY time-series signal here?" probe `deep-learning[8]` are cheap sanity
   checks that fit our bake-off philosophy without importing transformer overfitting
   surface.

---

## 3. Relevance for the project

**This is mostly "what NOT to do" — and that is exactly the kind of guidance a
measured-honesty advisor needs.** The relevance is high precisely because the research
is negative:

- **It externally validates the frozen gate + always-benchmark-vs-buy-and-hold design.**
  The literature's failures are all *absences* of what our gate hard-codes: a trivial
  baseline, cost discipline, multi-path validation, a significance test. Where rigorous
  papers DO include those, they reach our exact conclusion `deep-learning[42][52][77]`.
  The gate is a *competitive advantage*, not a limitation.

- **It makes the gate architecture-agnostic — liberating for the roadmap.** Because the
  honest evidence keeps saying "simple ≥ fancy" and "the deep edge needs cross-variate
  breadth a single coin lacks," our cost-net, buy-and-hold-benchmarked bootstrap **filters
  out the fad of the year automatically** `deep-learning[8][67][86]`. We never need to
  chase the new backbone (Mamba `deep-learning[56][74]`, KAN `deep-learning[75]`, the next
  one) — a direct argument for *not* spending engineering on DL forecasters.

- **It directly motivates the significance layer = "traceable & plausible."** The
  operator goal is a framework with *traceable and plausible* trading. `deep-learning[99]`
  is the cautionary archetype: a strategy can beat buy-and-hold on every risk-adjusted
  ratio and **halve drawdown**, yet fail a paired t-test (p=0.24–0.92) — and a
  drawdown-flattered ratio win is *not* plausible alpha. A DSR/PBO scorecard next to the
  verdict is precisely the "show me this isn't luck" artifact that makes a crown
  *traceable*. The power result `deep-learning[69]` (12% power at 34 folds) is the
  citable reason our **1000-path bootstrap exists**: it manufactures the resamples needed
  to have power, where a handful of OOS windows never can.

- **The `forecast` crate already embodies this verdict.** The retired TCN/PatchTST/GARCH-σ
  overlay chain in `crates/forecast/` is opt-in / narration-only and concluded
  *not-beating-passive* — this file explains, from the literature, *why* that was the
  correct conclusion and why those overlays must stay opt-in/narration-only.

---

## 4. Advantages for the project

- **Cheap, decisive, additive.** The significance layer reuses inputs the bake-off
  already stores (the T×N per-strategy return matrix + N). No new data, no new I/O, no
  model training. It is **additive** to the FROZEN classifier bands — it does not touch
  the weakest-link verdict, only adds a scorecard beside it.
- **Turns honesty into a feature surface.** A per-run overfitting scorecard (N_eff, DSR,
  PBO, MinBTL pass/fail, ΔZ) is a *product differentiator* for a measured-honesty
  advisor — it is the visible proof that "we tried hard to beat hold and here is the
  statistical evidence we couldn't." Competitors ship the headline; we ship the haircut.
- **A standing leak/overfit alarm.** The null-data falsification test is a permanent CI
  tripwire: any future change that makes the pipeline crown noise is caught immediately
  `deep-learning[39]`. This is the pipeline-level analogue of the project's day-1
  baseline-divergence e2e discipline.
- **Saves engineering by closing a whole lane.** The clearest advantage is *avoided
  work*: this research is the documented justification for **not** building a deep-learning
  alpha engine, so the team can decline every "let's try a Transformer/Mamba/KAN on the
  coin" proposal with a citation instead of a debate.

---

## 5. Problems and challenges (risks + HARD CONSTRAINTS bumped)

- **FROZEN gate / bands are additive-only.** The significance layer must be **additive**:
  it reports a *deflated* statistic and a scorecard *beside* the verdict. It must NOT
  alter the FRAGILE band (p5 Sharpe < 0) or the 5-signal weakest-link composite in
  `crates/backtest/src/bakeoff/robustness.rs`, nor the F2 crown comparator in
  `rank.rs`. (If a future decision *uses* DSR/PBO to gate a crown, that is a band change
  and is out of scope here — propose it as a separate, versioned spec.)
- **Anchored report SHAs are byte-immutable (119/119).** Adding scorecard fields to the
  ranking report mutates the report body. Any anchored report touched needs the
  ADR-0038 §D6 re-emission protocol; do **not** silently edit anchored files. New
  fields belong in a *new* report section / new anchor, not an edit to a frozen one.
- **`M > T` breaks naive `N_eff`.** Our exact situation (more swept configs than window
  bars) makes the return-correlation matrix ill-conditioned, so a naive ρ̄ is itself
  overfit. We **must dimension-reduce / cluster before estimating `N_eff`** (ONC / PCA)
  — a primary-source requirement, not a nicety (`SYNTHESIS.md §2 P0.1`, converging with
  `backtesting`).
- **Crypto fat tails shrink the survivable trial budget.** A given Sharpe clears DSR at
  far fewer trials under skew/kurtosis than under Normal returns — heavy-tailed coins
  warrant *more* suspicion of large sweeps, not less. The DSR variance term must use the
  cross-trial dispersion of the baked-off Sharpes, not the standard error of one Sharpe.
- **The haircut is nonlinear — "halve the Sharpe" is provably wrong.** For the sub-0.4
  net Sharpes a single coin realistically produces, the correct haircut is >50% to
  near-total. Risk: a naive linear haircut would *under*-correct and crown an overfit
  pick. The gate should crown almost nothing by construction — and the team must be
  comfortable shipping that as the honest, expected outcome.
- **Decimal-not-f64 discipline.** Excess-return and equity quantities are `Decimal`
  (USDT-denominated). The DSR/PBO math is inherently floating-point (Z-scores, logits),
  so it must live behind a clear boundary that converts at the edge and never lets f64
  leak into the money path — mirroring the existing `#![allow(clippy::float_arithmetic)]`
  scoping in `robustness.rs`.
- **Power is a permanent ceiling, not a bug to fix.** `deep-learning[69]` shows even 34
  honest folds have ~12% power. The bootstrap manufactures resamples to *raise* power,
  but on a thin edge the gate will often be "insignificant" — we must present that as a
  finding, not a failure, or risk pressure to loosen the gate.
- **Forecasters are opt-in / narration-only (retired chain).** Any future DL forecaster
  is constrained to opt-in/narration; if one ever produces a *sizing* or *decision*
  input it becomes an overlay and inherits the **day-1 baseline-equity-divergence e2e**
  mandate (the v3-vol-overlay-noop precedent; cf. `crates/forecast/tests/patchtst_overlay_neutrality.rs`).

---

## 6. Concrete next steps / candidate work items

> Most DL items here are **P2 / avoid**. The single actionable, high-value item is the
> **significance layer**, which feeds the program-wide **P0**.

| # | Item | Codebase location | Priority | Notes |
|---|------|-------------------|----------|-------|
| F-1 | **Selection-bias / significance scorecard** — `N_eff` (cluster-first when M>T) · DSR (>0.95 AND beats B&H) · PBO via CSCV · MinBTL veto · SPA studentization · ΔZ/BIF readout | `crates/backtest/src/bakeoff/{robustness.rs, rank.rs}` + ranking report (new section/anchor) | **P0** | *The* actionable item. Additive scorecard beside the FROZEN verdict; report deflated statistic, not a binary. Reuses stored T×N matrix. Motivated by `deep-learning[39][99][69]`, formulas in `SYNTHESIS.md §2`. |
| F-2 | **Null-data falsification CI test** — run full bake-off+rank on white-noise / regime-switch-vol / bid-ask-bounce / zero-alpha / GARCH(1,1); assert crowns nothing above B&H | new test under `crates/backtest/tests/` | **P0/P1** | Standing leak/overfit tripwire. Cheap, high value. `deep-learning[39]`. |
| F-3 | **DLinear-style linear-baseline sanity probe** — "is there ANY cost-net time-series signal on this coin?" one-line linear/last-value forecaster as a bake-off arm | `crates/forecast/` (probe only) or a bake-off baseline arm | P2 | Hypothesis: does NOT beat hold. Cheap minimal probe; a *baseline*, never an alpha claim. `deep-learning[8]`. |
| F-4 | **Distributional + decomposition primitives** — trend/seasonal decomposition; quantile output if any forecaster is ever surfaced | `crates/forecast/` | P2 | Only if a forecaster is ever surfaced; keep interpretable. `deep-learning[5][7][8]`. |
| F-5 | **Do NOT build a deep forecaster alpha engine** (Transformer / Mamba / KAN / TSFM) | — | **Avoid** | Documented decision with citations: simple ≥ fancy; deep edge is cross-variate breadth a single coin lacks. `deep-learning[8][24][67][86][88]`. |

**Highest-value item: F-1 (the significance scorecard).** It is the only constructive
deliverable the entire forecasting/significance corpus points to, it is additive and
cheap, and it directly advances the operator's "traceable & plausible" goal.

---

## 7. Open questions for analyst & architect

1. **Scorecard vs. gate.** F-1 ships the scorecard *beside* the FROZEN verdict (additive,
   in scope). Is there appetite for a *later, versioned* change where DSR<0.95 or a high
   PBO actually **vetoes** a crown? That is a band change (out of scope here) — but the
   scorecard is the prerequisite, so should F-1 be designed to make that future veto a
   one-line switch?
2. **`N_eff` clustering method.** ONC vs PCA vs a simpler correlation-cluster for the
   `M > T` case? Architect call on what fits the existing bootstrap data structures with
   the least new dependency.
3. **Report surface & anchors.** Where does the scorecard live in the ranking report so
   we add a *new* anchor rather than mutating a frozen one (119/119 byte-immutable)?
   Does the cockpit UI surface it (subject to `ui` not depending on strategy/exec/llm/
   models/forecast)?
4. **Threshold derivation.** Hard-code DSR ≥ 0.95, or derive the crown threshold from an
   explicit "a false 'beats-hold' is N× costlier than a miss" cost asymmetry (the ORATIO
   odds-ratio approach in `SYNTHESIS.md §2 P0.5`)? The latter is more honest but adds a
   product decision.
5. **f64 boundary.** Where exactly does the Decimal→f64 conversion happen for the DSR/PBO
   math so no float leaks into the money path?
6. **Power presentation.** How do we present "insignificant / underpowered" to a retail
   user without it reading as a product failure? This is a UX-of-honesty question that
   the significance layer forces.

---

## 8. What NOT to do / out of scope

- **No deep nets, no time-series foundation models, as the alpha engine.** Simple linear
  ≥ Transformers (shuffle test: DLinear −27%, transformers ~0%); the deep edge is purely
  cross-variate; lower forecast MSE does **not** mean more profit `deep-learning[8][86][99]`.
- **Never chase the new backbone.** Mamba/state-space `deep-learning[56][74]` and KAN
  `deep-learning[75]` repeat the same holes (no costs, no benchmark, one window,
  implausible metric — a Sharpe of 12.02 is a leakage flag, not a result). The gate is
  architecture-agnostic; let it do the filtering.
- **Never treat accuracy / price-level RMSE / image-AUC / a single-window Sharpe as a
  verdict.** Only equity-vs-hold net of costs over a path distribution counts. Beware
  **drawdown-flattered ratio wins** (a low-return/low-DD strategy topping B&H on a ratio
  while a paired-t can't reject "no edge") `deep-learning[99][69]`.
- **Do not import transformer/MLP forecasting complexity for one thin series.** TSMixer's
  own ablation shows the cross-variate machinery degenerates to the linear baseline
  without cross-variate breadth `deep-learning[67]`; a single coin is exactly that case.
- **Do not touch the FROZEN gate/bands or anchored reports** while adding the significance
  layer — additive scorecard only; new anchors, not edits.
