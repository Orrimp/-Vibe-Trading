# Application — Classical ML, ensembles, regime detection & honest baselines

_Decision doc for analyst + architect. Distilled from `research/ml-trading/` (100-paper ledger;
cite `ml-trading[N]` → `research/ml-trading/papers.md`) and cross-checked against
`research/SYNTHESIS.md`. This is the **"what-not-to-chase + how to test honestly"** half of the
ml-trading corpus: gradient boosting / random forests, regime detection, ensembles & the
Super-Learner oracle inequality, strong baselines, the M4 verdict, calibration, distributional/risk
outputs, and the on-domain costed negatives. The genuinely-actionable, gate-compatible pipeline
(triple-barrier, meta-labeling, the cost-aware filter, MinBTL/SR₀/DSR) lives in the companion
`application-ldp-pipeline-and-meta-labeling.md`._

> **The app this serves:** a Rust **single-coin crypto investment ADVISOR** — paper/sim only, NOT
> advice, NOT live. Pick ONE coin + budget → **bake off every strategy** on a (coin, window) →
> **rank** under a FROZEN robustness gate (1000-path moving-block bootstrap; FRAGILE ⇒ can't crown;
> **buy-and-hold always the benchmark and exempt**) → forward rule-based plan → watch it
> paper-trade. Validated thesis: **no active strategy robustly beats buy-and-hold net of costs**
> (hourly BTC: frictionless XGBoost +73.5%/yr → −64% at 10 bps; nothing beats B&H after Holm —
> `ml-trading[89]`). The product sells **measured honesty** — operator goal: *"a framework for
> trading with traceable and plausible trading."*

---

## 1. Summary of the research

This half of the corpus answers two questions: **"which ML, if any, is worth running?"** and
**"how do we test it so we don't fool ourselves?"** The answers are consistent and, for our app,
mostly cautionary.

**(a) Predictability is real but tiny, and the money is cross-sectional — unavailable to one coin.**
Gu–Kelly–Xiu (`ml-trading[1]`), the field benchmark, find ML predicts equity returns with monthly
OOS R² ≈ 0.4% even with 900+ features; the economic gain comes from a *large long–short cross-
section*, not any single asset. The crypto analogue (`ml-trading[98]`) reports a higher OOS
R²=4.855% — but it is **dominated by a one-day reversal effect** (→ maximal turnover → exactly the
cost-fragile regime `ml-trading[89]` flags) and is on-chain-fundamentals-driven long/short, none of
which a single long-only coin can harvest. Crypto efficiency itself is **frequency-dependent**:
daily returns ≈ random walk, weekly ≈ weak mean-reversion (`ml-trading[96]`). Time-series momentum
is real but a *diversified cross-asset vol-scaled* effect (`ml-trading[97]`) — n=1 can't capture it.

**(b) Classical/shallow ML is the right tool; complexity is double-edged and dies under costs.**
Tree ensembles beat linear/ARIMA on tabular data (`ml-trading[1][4][17]`); feature engineering +
classical models can beat deep learning on financial tabular tasks (`ml-trading[23]`). The named
learners are anchored from first principles: gradient boosting is **steepest descent in function
space** (`ml-trading[82]`), so XGBoost (L1/L2/γ leaf penalties — `ml-trading[79]`), LightGBM
(GOSS/EFB, leaf-wise — `ml-trading[80]`), and CatBoost (**ordered boosting** fixes a target-leakage/
prediction-shift *inside* the loop — `ml-trading[81]`) are refinements whose **regularizers, not
their pedigree, govern generalization**, to be tuned under CPCV, never on test. Random forests
generalize via Breiman's **strength-vs-correlation** bound (`ml-trading[85]`) — but OOB error
assumes IID and is **no substitute for purged CV** on overlapping labels. The genuine counter-
current is the **"virtue of complexity"** (`ml-trading[24]`): ridge-regularized overparameterized
(random-feature) models predict returns *better* OOS — but only **with shrinkage**, and the **gains
erode under transaction costs** (both caveats protect our thesis). If a predictor is ever wanted,
the principled high-complexity option is **random Fourier features + ridge** (still linear-in-
features, Rust-friendly) — expect costs + the single-asset constraint to neutralize it.

**(c) Ensembling cannot beat a benchmark already in its library — a theorem-shaped echo of our
thesis.** The **Super Learner / stacking oracle inequality** (`ml-trading[88]`): a CV-selected
stack is asymptotically **as good as the best single candidate in its library — no better**. So if
**buy-and-hold is in the library** and no active strategy robustly beats it, the optimal stack
*cannot* beat B&H either — ensembling is a defense against picking the wrong model, **not** a source
of alpha. The out-of-fold construction must be purged/embargoed (naive stacking on overlapping
labels leaks — `ml-trading[52]`), and the meta-learner adds a selection layer DSR must still cover.
The **M4 competition** (`ml-trading[91]`) agrees empirically from the other side: on 100k series,
**pure ML lost to simple statistical methods** (6 pure-ML entries all worse than the combination, 5
of 6 worse than Naïve2); only **combinations/hybrids won**, and only modestly (winner ES-RNN ~10%
over the combination benchmark). Bias-variance analysis confirms ensembling **balances** accuracy
and overfitting (`ml-trading[74]`) — it doesn't manufacture edge.

**(d) Regime detection helps risk more than return; all three branches share lag + "detection ≠
profit."** A *persistent* regime timer (statistical **jump model**, explicit switch penalty) beat
equity B&H on return *and* drawdown net of costs (`ml-trading[6]`); a *plain HMM* did **not** beat
B&H on return (too many flips → cost/whipsaw — `ml-trading[6][67][84]`). The three branches are
HMM, change-point (BOCPD — `ml-trading[11][12]`), and **distributional clustering** (Wasserstein/MMD
— `ml-trading[83]`, the right *labeler* concept but identification-only, backward-looking). All are
**lagged** (a window's regime is known only after observing it) → forward use is a **risk throttle /
strategy-selector candidate**, not timing alpha — and crypto lacks the equity-risk-premium tailwind
that made `ml-trading[6]` work.

**(e) Feature parsimony beats feature dumping; importance is unreliable on both axes.** Adding macro
+ social-media features *degraded* crypto direction accuracy; technical indicators were the workhorse
(`ml-trading[17][98]`). Tree feature-importance is untrustworthy by default — **MDI is inconsistent**
and collinearity-biased (`ml-trading[58]`), **MDA is confounded under correlated features**
(`ml-trading[49]`) — so use **TreeSHAP** / drop-column / Sobol-MDA, or (the LdP fix) **Clustered-MDA**
(`ml-trading[94]`), and report a bootstrap spread, not a point (`ml-trading[78]`). Mass feature
generation (tsfresh — `ml-trading[63]`, ROCKET's 20k — `ml-trading[47][54]`) needs its own
multiple-testing gate (an **FDR filter**), the feature-side twin of DSR.

**(f) The honest-testing canon — strong baselines + the negative results.** A **no-intelligence
baseline routinely beats published AI traders** under honest, high-volume testing (`ml-trading[13]`);
**stronger baselines** (interpretable + properly tuned + utility-aligned metric) often match/beat
complex ML (logistic regression beat a transformer 0.74 vs 0.70; a GAM beat an autoencoder AU-ROC
0.83 vs 0.70 — `ml-trading[100]`); a **weak/strawman baseline is as misleading as none**. The
cleanest on-domain costed negative: **13 RF models on SPY minute data, train accuracy 80–87% → test
48–50% (worse than a coin flip), train R² ~0.78 → test R² negative, every model lost to B&H +2.29%**
(`ml-trading[38]`). The most-cited crypto "beats the market" paper (`ml-trading[43]`, 1,681 coins,
astronomical returns) dissolves on inspection: **survivorship bias** (dead coins excluded),
zero-market-impact assumptions, no walk-forward, no significance test — astronomical compounded
returns are a *red flag*, like R²≈1.0 on price levels (`ml-trading[16]`). The 41-model Bitcoin
survey (`ml-trading[90]`) tuned hyperparameters to maximize PnL across a big grid (textbook
data-snooping) and admits backtests "don't translate forward" — no costs, no B&H.

**(g) Prefer distributional / risk outputs over point forecasts — predictability hides in the
tails.** The center of the return distribution is near-unpredictable, but there is genuine
*distributional* structure: penalized quantile predictors **reverse sign** from lower to upper
quantile (`ml-trading[30]`), and distribution-free quantile methods beat GARCH for **VaR/Expected-
Shortfall** tail forecasting (`ml-trading[31]`). The honest output for a single coin is a **return
interval** (Multi-step Split Conformal — `ml-trading[45]`; or Quantile Residual Simulation —
`ml-trading[44]`, both avoiding the quantile-crossing failure) or a **downside-risk number**, not a
point prediction — and a **wide interval straddling zero is itself the truthful "we don't know —
holding is fine" signal**. Conformal needs **exchangeability**, which temporal dependence/regime
change violate (`ml-trading[25]`), and several popular wrappers (EnbPI/SPCI/Nixtla) *fail* coverage
on dependent data (`ml-trading[45][46]`) — so **validate coverage empirically**. Calibration
(ECE/Brier/reliability) is mandatory for any probability we size on; **Platt beats isotonic when
calibration data is scarce** (our single-coin regime — `ml-trading[51][36]`). For volatility, **GARCH
remains a strong conservative baseline** — ML edges it on average error but **ML under-predicts vol
while GARCH over-predicts** (`ml-trading[28]`), so GARCH's conservatism is safer for a risk throttle;
the **HAR-RV** family (regress RV on lagged daily+weekly+monthly RV) is a trivially-Rust linear model
that **beats GARCH/ARFIMA** at vol forecasting (`ml-trading[48][44]`).

**(h) Concept drift is the operational risk of the forward phase — a re-bake trigger, not alpha.**
A strategy ranked best on a past window goes stale when the input→target relation shifts (concept
drift — `ml-trading[26][92]`). Cheap drift detectors can *track* the shift, but detection **lags**
(`ml-trading[11][12][26]`) and online learners' regret bounds are only *relative* ("close to the best
fixed rule" — so if no fixed rule beats B&H, low regret still doesn't beat B&H — `ml-trading[35]`).
Drift tooling is best as a **staleness monitor / re-bake trigger** paired with a minimum-evidence
period (`ml-trading[34]`), never timing alpha.

**Through-line:** classical ML is the right default *if you must model at all*, but on a single
noisy coin it overfits as readily as anything; the **gate, not the learner, governs**, and the most
honest thing the advisor can *add* is a calibrated uncertainty band / downside-risk number, not a
return prediction.

---

## 2. Possible solutions / what can be done with this research

1. **Keep naive/statistical baselines first-class in the bake-off** (B&H, predict-last, a tuned
   simple rule) — M4 and the strong-baseline literature make "did you beat a *strong* baseline" the
   verdict, not accuracy/AUC (`ml-trading[91][100][13]`).
2. **Add a calibrated uncertainty band / downside-risk output to the advisor** — MSCP
   (`ml-trading[45]`) or Quantile Residual Simulation (`ml-trading[44]`) for an interval; quantile
   VaR/ES (`ml-trading[31]`) for downside; HAR-RV / GARCH-conservative for a vol estimate
   (`ml-trading[48][28]`). Validate coverage empirically. This is an **honesty/disclosure win
   independent of alpha**.
3. **If a predictor is ever added, stay classical + heavily regularized** — gradient-boosted trees
   with regularizers tuned under CPCV (`ml-trading[79][80][81][14]`), or random Fourier features +
   ridge (`ml-trading[24]`); expect costs to neutralize both.
4. **Treat ensembling as a model-selection defense, not an alpha source** — if used, purged/
   embargoed out-of-fold, and remember the oracle inequality caps it at "as good as B&H if B&H is in
   the library" (`ml-trading[88][91][52]`).
5. **Use Clustered-MDA / TreeSHAP (not naive MDA/MDI) for any feature story**, bootstrap-checked,
   reporting a spread (`ml-trading[94][58][49][78]`).
6. **Keep the feature set small and technical; gate mass-generated features with FDR** if auto-
   extraction is ever used (`ml-trading[17][63]`).
7. **Add a staleness monitor + minimum-evidence period to the forward phase** as a re-bake trigger
   (`ml-trading[26][92][34]`) — a guardrail, expecting detection lag.
8. **Validate the gate on synthetic no-alpha series** (it must refuse to crown) — shared with the
   companion doc and `research/SYNTHESIS.md` P1.

---

## 3. Relevance for the project

This strand's primary value is **negative and disciplinary** — it tells us what *not* to build and
how to keep the gate honest — with one genuinely *additive* product idea (the distributional/risk
output).

- **It validates the FROZEN gate's whole reason to exist.** Every honest-testing paper
  (`ml-trading[13][38][89][90][91][100]`) shows the same thing: impressive in-sample / accuracy
  metrics collapse OOS and lose to B&H net of costs. Our gate is the institutional embodiment of
  that lesson. The **strong-baseline requirement** (`ml-trading[100]`) maps directly onto "B&H is
  always the benchmark and is exempt" — and warns that a *weak* baseline is as misleading as none,
  which our always-on, never-disabled B&H benchmark already enforces.

- **It tells the architect which ML to refuse.** The product's v1 posture is *ML/LLM narration-only*;
  this corpus is the evidence base for keeping it that way. Deep nets add nothing on a single coin
  (`research/SYNTHESIS.md` §3), pure ML loses to simple baselines (`ml-trading[91]`), ensembling
  can't beat B&H-in-library (`ml-trading[88]`), and the retired TCN/PatchTST/GARCH/LLM-forecaster
  overlays already concluded not-beating-passive. The honest door we hold *slightly* open —
  classical + heavy shrinkage (`ml-trading[24]`) — comes with the paper's own "dies under costs"
  caveat.

- **The distributional/risk output is the one additive, "traceable & plausible" idea here.** Since
  the mean is near-unpredictable but the tails carry structure (`ml-trading[30][31]`), the most
  defensible thing the advisor can *add* is not a return prediction but a **calibrated uncertainty
  band** and a **downside-risk number** — where a wide interval straddling zero *is* the honest
  "holding is fine" signal. That is precisely the "measured honesty" the product sells, and it
  doesn't require beating B&H to be valuable.

- **Regime detection is a risk-throttle candidate, not alpha** — and the jump-model result
  (`ml-trading[6]`) is honestly flagged as possibly equity-specific (no crypto risk-premium tailwind),
  reinforcing the existing vol-/regime-overlay items in `research/SYNTHESIS.md` P1.

**Honest on expected-null.** Nothing here is a return edge for a single long-only coin. The
cross-sectional gains (`ml-trading[1][98]`) are structurally unavailable; the costed on-domain tests
(`ml-trading[38][89]`) lose to B&H; ensembling is theorem-capped (`ml-trading[88]`); the
distributional output is an *honesty* win, not an alpha win. The strand's job is to make our null
**more trustworthy** and to **stop us chasing complexity that the literature already retired**.

---

## 4. Advantages for the project

- **A citable evidence base for the narration-only-ML posture.** When the operator (or a future
  contributor) asks "why not bolt on XGBoost / an ensemble / a deep net?", this corpus is the
  one-line answer: pure ML loses to simple baselines (`ml-trading[91]`), single-coin gives up the
  cross-section (`ml-trading[1]`), ensembling can't beat B&H-in-library (`ml-trading[88]`), and the
  costed crypto test loses to B&H (`ml-trading[89]`).
- **The strong-baseline framing sharpens our gate's verdict logic.** "Beat a *strong* baseline on a
  *utility-aligned* metric" (`ml-trading[100]`) is exactly B&H-net-of-costs over a path distribution
  — confirming our verdict metric and warning against ever swapping in accuracy/AUC/IC.
- **The distributional/risk output is Rust-trivial and additive.** HAR-RV is a 3-term linear
  regression; conformal/QRS intervals are point-forecast + empirical residual quantiles; quantile
  VaR/ES is order statistics. No `candle`/`tract` needed; renders as a passive band/number.
- **GARCH-conservative is the *safe* choice for a risk throttle** — `ml-trading[28]`'s finding that
  ML under-predicts vol while GARCH over-predicts means the interpretable, conservative model is
  also the *correct* one for not underestimating risk when extremes loom.
- **Clustered-MDA gives an honest feature story** on our co-moving technical features where naive
  MDA/MDI/SHAP lie (`ml-trading[94][49][58]`) — useful for any operator-facing explanation.
- **It pre-empts wasted effort.** Macro/social features hurt (`ml-trading[17][98]`); mass feature
  generation is the kitchen-sink trap (`ml-trading[63]`); deep TSC didn't beat InceptionTime
  (`ml-trading[54][55]`). Knowing this saves build cycles.

---

## 5. Problems and challenges (risks + HARD CONSTRAINTS bumped)

**Research-intrinsic risks:**

- **The distributional output can be mis-sold as a forecast.** A conformal/QRS interval is an
  *honesty* device; if surfaced carelessly it can read as "the model predicts X." The UX must frame
  a wide interval straddling zero as "we don't know — holding is fine" (`ml-trading[30][45]`), and
  coverage must be **validated empirically** because EnbPI/SPCI/Nixtla *fail* on dependent data
  (`ml-trading[45][46]`) and conformal assumes exchangeability crypto violates (`ml-trading[25]`).
- **Calibration breaks on small single-coin samples.** Use **Platt, not isotonic** when calibration
  data is scarce (`ml-trading[51]`), and always verify on held-out data (`ml-trading[36]`).
- **Regime detection lags and "detection ≠ profit."** All three branches are backward-looking
  (`ml-trading[83][11][12]`); the only persistent winner was on *equities* with a risk-premium
  tailwind crypto lacks (`ml-trading[6]`). Treat as a throttle candidate, gate against B&H net of
  costs, prefer a persistent (jump-model) labeler over flip-prone HMM.
- **Tree feature-importance is unreliable on both axes** — never trust default MDI/MDA on our
  correlated features (`ml-trading[49][58]`); and interpretability still ≠ edge (`ml-trading[94][8]`).
- **Any ensemble/learned component re-incurs the selection-bias bill** — the meta-learner adds a
  selection layer DSR/MinBTL must still cover (`ml-trading[88]`, and see the companion doc's gate
  items), and naive stacking on overlapping labels leaks (`ml-trading[52]`).
- **"Virtue of complexity" is a real but narrow exception** — it needs shrinkage and still dies
  under costs (`ml-trading[24]`); do not over-read it as license for deep nets.

**HARD CONSTRAINTS this strand must respect (name them in any work item):**

- **Paper-only; ML/LLM is narration-only in v1.** Any *learned predictor* (GBDT, ensemble, RFF-ridge,
  drift-tracking online learner) is out of v1 scope — opt-in research at most, following the retired
  TCN/PatchTST/GARCH/LLM-forecaster precedent (concluded not-beating-passive). The distributional/
  risk band is borderline: ship it as a **descriptive disclosure**, not a trading signal.
- **Gate/bands are FROZEN — additive only.** The strong-baseline framing *confirms* our verdict
  metric; it must not be used to alter the frozen FRAGILE/robustness bands or the weakest-link
  verdict. Any new diagnostic is additive in `crates/backtest/src/bakeoff/{robustness.rs, rank.rs}`.
- **Anchored report SHAs are byte-immutable (119/119).** A distributional band or vol-estimate added
  to a report goes in a *new* section/file or via the ADR-0038 re-emission protocol — never mutate an
  anchored body. Run `scripts/verify_anchors.sh` before and after any `spec/*/reports/` touch.
- **Decimal, not f64, for money.** The statistics (R², Sharpe, VaR-as-a-return, interval bounds in
  *return* space) can be f64; anything that becomes a **position size / budget number / sized bet**
  must be `Decimal`. A VaR/ES used to *throttle position size* crosses that seam — keep it clean.
- **`ui` must NOT depend on strategy/exec/llm/models.** A distributional band, vol estimate, or
  feature-importance story is data produced by `backtest`/a model crate; `ui` renders it as a passive
  struct. Do **not** let it pull a strategy/exec/llm/models type into the `ui` graph.
- **Overlays ship a day-1 baseline-equity-divergence e2e.** If a regime-flat / vol-throttle overlay
  (the risk-throttle candidate here) is built, it is a sizing-modifier → it MUST ship a day-1 e2e
  asserting its output equity diverges from the un-throttled baseline by ≥ epsilon when the regime/
  vol variable is non-trivial (the v3-vol-overlay-noop precedent). A risk band that only *displays* a
  number and never sizes is exempt — but the moment it touches sizing, the e2e is mandatory.
- **ML default crates: `candle` (prototyping) / `tract` (ONNX serving)** per architecture — but the
  *actionable* items here (HAR-RV, conformal/QRS bands, quantile VaR/ES, Clustered-MDA) need
  **neither**: closed-form stats + a linear regression + order statistics. Reach for `candle`/`tract`
  only if a GBDT/learned predictor graduates from the P2 research bucket.

---

## 6. Concrete next steps / candidate work items

Named, with codebase location and priority. Most of this strand is "do not build" — the few additive
items are deliberately modest.

- **[P0] `honest-baseline-and-verdict-guardrails`** — encode the strong-baseline + utility-metric
  discipline as engine assertions: B&H always present & exempt (already true — confirm + test), never
  accept accuracy/AUC/IC as a verdict, always show fee-sensitivity. Mostly *confirmation + standing
  tests*. Location: `crates/backtest/src/bakeoff/rank.rs` + tests. `ml-trading[100][13][91]`.
  (Overlaps the companion doc's gate work — sequence after the selection-bias gate items.)

- **[P1] `distributional-risk-band` (descriptive disclosure)** — add a calibrated **return interval**
  (MSCP `ml-trading[45]` or QRS `ml-trading[44]`) and/or **downside-risk number** (quantile VaR/ES
  `ml-trading[31]`) to the advisor output, with **empirical coverage validation** and **Platt
  calibration** for any probability. Surfaced as a **descriptive band**, not a signal; a wide
  zero-straddling interval is the honest "holding is fine" message. Location: a new producer in
  `backtest` (or a small stats module) → passive `ui` struct. **No sizing in v1** (keeps it out of
  narration-only-ML and avoids the divergence-e2e gate). `ml-trading[30][45][44][31][51][36]`.

- **[P1] `har-rv-vol-estimate`** — a HAR-RV (lagged daily+weekly+monthly RV) and/or GARCH-conservative
  realized-vol estimate as a **descriptive** risk number (and a *candidate* throttle input, gated
  separately). Rust-trivial linear regression. Location: a small vol module feeding the report.
  `ml-trading[48][44][28]`.

- **[P1] `staleness-monitor`** — a cheap concept-drift detector on the forward paper-trade stream as
  a **re-bake/alert trigger**, paired with a minimum-evidence period before any "beating B&H" claim.
  Guardrail, not alpha; expect detection lag. Location: forward-plan / paper-trade loop + report.
  `ml-trading[26][92][34]`.

- **[P2] `regime-throttle-spike`** — research spike: a **persistent (jump-model)** vol/regime
  classifier as a "stay-in-cash in unstable regimes" overlay, with hysteresis / explicit switch
  penalty, OOS-CV params, detection-lag model, gated vs B&H net of costs. **Day-1 baseline-divergence
  e2e mandatory** (it sizes). Prior: drawdown-only, no return win. Location: new gated overlay
  candidate. `ml-trading[6][83][84]`.

- **[P2] `classical-predictor-spike` (opt-in research only)** — *if ever* a predictor is wanted:
  regularized GBDT (`candle` or a tree crate) or RFF+ridge, tuned under CPCV, with avg-uniqueness
  weights, Clustered-MDA explanation, gated vs B&H net of costs. Opt-in, narration-out-of-scope,
  high prior of "no return win, dies under costs." Location: gated candidate behind a feature flag.
  `ml-trading[24][79][80][81][14][94]`.

---

## 7. Open questions for analyst & architect

1. **Is the distributional/risk band in-scope for v1** as a *descriptive disclosure* (no sizing), or
   does its model-derived nature put it on the narration-only-ML wrong side of the line? It is the
   one additive "measured honesty" idea in this strand — worth an explicit scope ruling.
   `ml-trading[45][44][31]`.
2. **Which interval method ships first** — MSCP (`ml-trading[45]`, benchmarked winner on
   coverage+efficiency) or the simpler Quantile Residual Simulation (`ml-trading[44]`)? Both avoid
   quantile-crossing; MSCP needs empirical coverage validation on our coins.
3. **Vol estimate: HAR-RV vs GARCH-conservative** for a risk number/throttle input — `ml-trading[48]`
   says HAR-RV beats GARCH on forecast error, but `ml-trading[28]` says GARCH's over-prediction is
   *safer* for a throttle. Which property do we want where (display vs sizing)?
4. **Do we want a regime throttle at all** given the jump-model win was equity-specific
   (`ml-trading[6]`) and the prior for crypto is drawdown-only/no-return-win? This overlaps the
   `research/SYNTHESIS.md` P1 vol/regime overlay items — coordinate so we don't build two.
5. **Staleness-monitor sensitivity:** does drift-detection lag (`ml-trading[11][12][26]`) make a
   re-bake trigger fire too late to help and just add churn? Needs a calibration study before it's
   worth wiring.
6. **Is there *any* appetite for a learned predictor** (the P2 spike), or is the corpus' verdict
   (`ml-trading[91][88][89]`) decisive enough to keep the door shut and spend the effort elsewhere?

---

## 8. What NOT to do / effort & blast radius

**Do NOT (this strand is largely a "do-not" list):**
- **Do NOT add a deep net / TSFM as the alpha engine** — simple beats fancy on a single coin
  (`research/SYNTHESIS.md` §3); pure ML loses to simple baselines (`ml-trading[91]`).
- **Do NOT expect ensembling/stacking to beat B&H** — the oracle inequality caps it at "as good as
  the best library member"; if B&H is in the library, that's the ceiling (`ml-trading[88]`).
- **Do NOT add macro/social-media features for return** — they degrade crypto prediction
  (`ml-trading[17][98]`).
- **Do NOT mass-generate features without an FDR gate** — it's the kitchen-sink trap and re-incurs
  selection bias (`ml-trading[63][17]`).
- **Do NOT trust default tree feature-importance** (MDI/MDA) on our correlated features — use
  Clustered-MDA/TreeSHAP with a bootstrap spread (`ml-trading[49][58][94][78]`).
- **Do NOT report R² on price levels, accuracy/AUC/IC as a verdict, or any metric without B&H +
  costs** (`ml-trading[16][13][100]`).
- **Do NOT treat a high-R² / high-accuracy crypto-ML result at face value** — astronomical compounded
  returns and survivorship-clean omissions are red flags (`ml-trading[43][38]`).
- **Do NOT let a risk band/vol estimate that touches sizing skip its day-1 divergence e2e**, or pull
  a model type into the `ui` graph.

**Effort & blast radius (rough):**
- *Honest-baseline guardrails (P0):* **low** — mostly confirmation + standing tests over existing
  `rank.rs`; sequence after the companion doc's selection-bias gate.
- *Distributional band + HAR-RV (P1, descriptive):* **low-medium** — closed-form stats + a passive
  `ui` struct; the only real work is empirical coverage validation. Additive; **no sizing** keeps it
  out of the overlay-divergence gate.
- *Staleness monitor (P1):* **low-medium** — a detector on the forward stream + a re-bake trigger;
  needs a lag-calibration study first.
- *Regime-throttle spike (P2):* **medium** — a sizing overlay → day-1 divergence e2e mandatory; high
  prior of drawdown-only.
- *Classical-predictor spike (P2):* **high, low expected value** — a full learned pipeline (labels,
  weights, CPCV, calibration, explanation, divergence e2e), opt-in, out of v1 narration-only scope.
  Do this only if the door is explicitly re-opened; the corpus says keep it shut.
