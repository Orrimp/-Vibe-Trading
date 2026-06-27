# Knowledge — Machine Learning for Trading (classical ML, feature eng., forecasting)

_Synthesized from `papers.md` (source of truth)._
_Progress: 46 papers logged ([1]–[46]). Round 2 added [25]–[46]: conformal prediction &
calibration, GARCH & quantile/probabilistic risk forecasting, feature selection & sample
reweighting, concept drift / online learning, costed-backtest negative results, crypto-ML
overfitting, gradient-boosting volatility, and gate/objective-design papers._

## Key themes

1. **Predictability is real but tiny, and the money is in the cross-section.**
   Gu–Kelly–Xiu [1] — the field's benchmark — find ML predicts equity returns with
   monthly OOS R² of only ~0.4% at the stock level even with 900+ features; the
   economic gains come from a *huge long–short cross-section*, not from any single
   asset. A single-coin long-only paper advisor cannot harvest that structure.

2. **The dominant risk is fooling yourself, not weak models.** Recurring failure
   modes across the literature, each with a concrete guardrail:
   - Fitting transforms (smoothing/scaling/frac-diff d) on the *whole* series before
     splitting → look-ahead leakage [5]. Guardrail: fit on train only, apply forward.
   - Reporting statistical metrics with **no buy-and-hold/random-walk benchmark and
     no costed backtest** [4][16][17]. Guardrail: B&H always-on; costs first-class.
   - **R² on price *levels*** (≈1.0 trivially) instead of on returns/changes [16].
   - Accuracy that *grows with horizon* — a leakage tell [5].
   - Under-powered evaluation (few test scenarios) manufacturing false winners [13].
   - Testing only in a bull / low-vol window [3][21].
   - **Searching many rules/strategies and reporting the best without penalty** —
     data snooping [19][7][20].

3. **A disciplined financial-ML pipeline is the antidote, and it's implementable.**
   López de Prado [2]: fractional differentiation (stationarity *with* memory),
   triple-barrier labeling, meta-labeling, sample-uniqueness weighting, **purged
   k-fold CV + embargo**, MDA importance, **deflated Sharpe**. The
   selection-overfitting toolkit is concrete: **White's Reality Check / Hansen's SPA**
   [19], **PBO via CSCV** [7], the **Deflated Sharpe Ratio** [20], and — empirically —
   **CPCV beats k-fold/walk-forward** in synthetic ground-truth tests [14]. All are
   linear/recursive/bootstrap — Rust-friendly, no deep net.

4. **Classical/shallow ML is the right tool; complexity is double-edged.** Tree
   ensembles beat linear/ARIMA on tabular data [1][4][17]; feature engineering +
   classical models can beat deep learning on financial tasks [23]. BUT there is a
   real, peer-reviewed counter-current: **the "virtue of complexity"** [24] shows
   that *ridge-regularized* overparameterized (random-feature) models predict returns
   *better* out-of-sample — with two caveats that protect our thesis: it **requires
   shrinkage**, and **gains erode under transaction costs**. Net: stay classical by
   default; complexity is only safe with heavy regularization and still loses to
   costs.

5. **"Beats a linear model" ≠ "beats holding net of costs."** A no-intelligence
   baseline routinely beats published AI traders under honest, high-volume testing
   [13]; technical-rule profits vanish after data-snooping correction [19]; ML
   asset-pricing profits concentrate in hard-to-arbitrage names and **deteriorate
   under realistic turnover/costs** [21]. Even the favorable result [22] takes a ~57%
   haircut from costs+decay and survives only on a broad multi-anomaly,
   high-turnover, (deep-learning) configuration we cannot use.

6. **Regime detection helps risk more than return, and persistence is everything.**
   A *persistent* regime-timer (statistical jump model) beat equity B&H on return AND
   drawdown net of costs [6]; a *plain HMM* did NOT beat B&H on return (too many
   flips → cost/whipsaw) [6]. Online change-point detection (BOCPD [11], AR-BOCPD
   [12]) is light and Rust-friendly but has structural **detection lag** — signals
   arrive after the move, so it's a risk throttle / descriptive overlay, not timing
   alpha.

7. **Feature parsimony beats feature dumping — but regularize, don't just hard-select.**
   Adding macro + social-media features *degraded* crypto direction accuracy;
   technical indicators on price/volume were the workhorse [17]. Most "new" factors
   are redundant given the zoo [10]; ~65% of published anomalies don't replicate
   conservatively [9]. Yet "Shrinking the Cross-Section" [18] warns the right frame is
   **shrinkage toward stable/high-variance directions**, not naive sparsity. Concrete
   selection methods: **permutation/shuffling importance** [32] (≈ MDA [2]), **FSA**
   [33], **double-selection LASSO** [10] — and the *same multiple-testing penalty*
   [19][20] applies to "best feature subset" as to "best strategy."

8. **Forecast *distributions*, not point estimates — predictability hides in the tails.**
   The center of the return distribution is near-unpredictable, but there is genuine
   *distributional* structure: penalized quantile regression finds predictors whose
   sign **reverses from the lower to the upper quantile** [30], and distribution-free
   quantile methods beat GARCH for **VaR/Expected-Shortfall** tail forecasting [31].
   This reframes the tiny mean-R² of [1][4]: a model can be useless for the mean yet
   informative about *risk*. The honest output for a single coin is a **return
   interval** (conformal [25]) or a **downside-risk number** (VaR/ES [31]), not a
   point prediction — and a wide interval straddling zero is itself evidence of no edge.

9. **Uncertainty must be *calibrated*, and finance breaks the easy assumptions.**
   Conformal prediction gives distribution-free coverage — but only under
   **exchangeability**, which temporal dependence and regime change violate [25];
   adaptive/online conformal (state-aware, reweighted calibration) is required for
   non-stationary series. Calibration is a first-class concern for any probability we
   show or size on.

10. **Concept drift / non-stationarity is the operational risk of the forward phase.**
    A strategy ranked best on a past window goes stale when the input→target relation
    shifts ("concept drift" [26], the ML name for regime change). Cheap drift detectors
    [26] and online/regret-bounded learners [35] can *track* the shift — but detection
    lags the move [11][12][26] and a regret bound is only *relative* ("close to the best
    fixed rule"), so if no fixed rule beats B&H, low regret still means you don't beat
    B&H [35]. Drift tooling is best as a **staleness monitor / re-bake trigger**, not
    timing alpha.

11. **For volatility, GARCH is still a strong, conservative baseline.** ML edges GARCH
    on average vol-forecast error but by tiny, asset-specific margins, and **ML
    systematically *under*-predicts vol while GARCH *over*-predicts** [28] — so for a
    *risk* throttle GARCH's conservatism is safer (don't underestimate risk when
    extremes loom). GARCH encodes real stylized facts (clustering, leverage) worth using
    as an interpretable feature [27][28]. GARCH-X is a sensible Rust-implementable vol
    estimator.

12. **Independently-converged gate designs validate ours — and add concrete rules.**
    Two 2026 frameworks reinvent our bake-off gate from scratch: AlgoXpert's **IS-WFA-OOS**
    protocol [40] (stable parameter *plateaus* + cliff-sensitivity veto, walk-forward with
    purge gap, **majority-pass 2/3 folds + catastrophic veto**, OOS parameter-lock with
    pre-committed thresholds) and GT-Score [41] (rank on a **robustness-aware multiplicative
    composite** μ·ln(z)·R²/σ_d, not raw return → +98% OOS-retention). Both are weakest-link
    in spirit, like ours. The new actionable rules: (a) crown a *plateau*, not the single
    peak; (b) rank on consistency + downside + significance, not raw return — but with a
    **fat-tail-correct** significance term [20], since both papers' Z-scores assume
    normal i.i.d. returns, which crypto violates.

13. **Costed, B&H-benchmarked ML studies confirm the prior — directional accuracy is an
    OOS mirage; famous crypto "wins" are survivorship artifacts.** The cleanest replication
    [38]: 13 RF models on SPY minute data, train directional accuracy 80–87% → **test 48–50%
    (worse than a coin flip)**, train R² ~0.78 → **test R² negative**, and **every model lost
    money vs B&H +2.29%**. Price features beat technical indicators at that frequency. And the
    most-cited crypto-ML "beats the market" paper [43] (1,681 coins, astronomical returns)
    dissolves on inspection: **survivorship bias** (dead coins excluded), zero-market-impact /
    unlimited-supply assumptions, unstable parameters, no walk-forward, no significance test —
    the crypto analogue of the equity microcap anomaly [9][21]. Astronomical compounded
    returns are a *red flag*, like R²≈1.0 on price levels [16]. Joins [13][19][34] as the
    honest-testing set; reinforces single-coin, liquid, cost-aware, survivorship-clean.

14. **Sample weighting for non-IID labels is mandatory if we ever train.** Financial
    labels from overlapping windows are non-IID — far fewer *unique* observations than
    rows — so naive training overfits [42][2]. Two complementary reweighting schemes:
    **average-uniqueness weighting + bagging `max_samples≈avg uniqueness`** [42][2]
    (weight by overlap) and **learning-trajectory reweighting** [32] (weight by
    learnability). Both reject the IID assumption that standard ML brings to finance.

## Methods / findings that hold up (and which don't)

**Hold up:**
- **CPCV** (purged + embargoed + combinatorial) and walk-forward for honest OOS
  evaluation [1][4][6][14].
- Selection-overfitting gates: **Reality Check / SPA** [19], **PBO/CSCV** [7],
  **Deflated/Probabilistic Sharpe** [20], deflated Sharpe [2].
- **Triple-barrier labeling** [2][3]; **fractional differentiation** fit on train
  only [2][3].
- **Meta-labeling** as a low-risk *filter to trade less* [2][15].
- **Tree ensembles > linear** on tabular features [1][4][17]; **feature engineering +
  classical ≥ deep learning** for financial tabular tasks [23].
- **Persistent regime timing (jump model)** can beat equity B&H net of costs [6].
- **Shrinkage/regularization** as the source of OOS robustness [18][24].
- **Parsimonious technical-indicator features**; causal/selective beats kitchen-sink
  [17][10].

**Do NOT hold up / red flags:**
- Direction-accuracy from full-series smoothing + long-horizon sign labels [5].
- Any metric with no B&H/random-walk baseline and no costed backtest [4][16][17].
- **R² on price levels** as success [16].
- Plain **HMM** regime timing for *return* enhancement (flip-prone) [6].
- Single-bull/low-vol-window evaluation [3][21]; under-powered evaluation [13].
- Bolting on macro/social-media features expecting gains [17].
- Technical trading rules as alpha once data-snooping is corrected [19].
- **Unregularized** complexity [24]; assuming "smarter execution" rescues a marginal
  edge [22].

## Actionable takeaways for our advisor

1. **Upgrade the bake-off gate with multiple-testing corrections — we have the inputs
   for free.** Our gate already knows **N** (strategies/params tried) and each
   strategy's per-period returns. Add, alongside the 1000-path moving-block bootstrap:
   (a) a **Deflated Sharpe Ratio** + Probabilistic-Sharpe for the crowned strategy
   [20] (corrects N, sample length, skew, kurtosis — apt for crypto fat tails); (b)
   optionally a **Hansen SPA p-value** [19] or **PBO via CSCV** [7]. Treat
   DSR<benchmark / high PBO / weak SPA as a veto. This closes the *selection*-
   robustness axis the bootstrap alone doesn't.

2. **Hard test-data rules (codify in the engine).** (a) Fit every transform on the
   in-sample window ONLY, apply forward [5]. (b) Report R²/error on **returns, never
   price levels** [16]. (c) Always show a naive baseline (B&H / predict-last)
   [4][13][16]. (d) Make **transaction costs + turnover penalty first-class**; a
   strategy must beat B&H *net of realistic costs* [21][22]. (e) Stress across
   regimes, not just calm/bull windows [3][21].

3. **Meta-labeling is the top ML experiment — as a "do less" filter.** Keep our
   simple strategy as the *side*; add a small interpretable classifier (tree/logit) to
   decide *whether to act*, triple-barrier-labeled [2], CPCV-validated [14], gated vs
   B&H net of costs [15]. Most plausible win: fewer, higher-confidence trades → less
   cost drag. Cannot underperform "always act" if gated.

4. **A persistent vol/regime feature is worth a gated experiment — as a risk
   throttle.** Prefer a jump-model-style persistent classifier (interpretable,
   Rust-friendly) over a flip-prone HMM [6]; expect lag [11][12]. Use it to "trade
   less / stay in cash in unstable regimes," never as a timing-alpha claim without
   clearing the gate.

5. **Stay classical; if you must add a predictor, use random-features + ridge.** For
   tabular financial prediction, classical models + good features are competitive with
   or beat deep nets [23] at a fraction of the cost. If a forecasting overlay is ever
   wanted, the principled high-complexity option is **random Fourier features + ridge**
   [24] (still linear-in-features, Rust-friendly) — but expect costs + the single-asset
   constraint to neutralize it.

6. **Keep the feature set small and technical; regularize when combining.** Don't add
   macro/social-media data — it tends to hurt and inflates overfitting surface
   [17][10]. When blending signals, prefer shrinkage over lucky sparse selection [18].
   Diagnose feature value with permutation/MDA + SHAP, mindful of the correlated-
   feature substitution caveat [2][8].

7. **Default to skepticism; report risk-adjusted vs absolute separately.** Many
   "winners" beat B&H only on Sharpe/vol, not total money [3][6-HMM]; a no-intelligence
   baseline beats most clever strategies under honest testing [13]; technical rules are
   a data-snooping artifact [19]. Our prior — **holding wins for a single coin net of
   costs** — survives the literature; hold the door only *slightly* open ([24][22]
   show edge is possible but in configurations we can't use).

8. **Prefer distributional / risk outputs over point forecasts (an honesty win, even
   without alpha) — and here are the concrete recipes.** Since the mean is near-unpredictable
   but the tails carry structure [30][31], the most defensible thing our advisor can *add* is
   not a return prediction but an **honest uncertainty band** and a **downside-risk number**.
   Concrete, classical, Rust-friendly methods that came up: (a) for an interval, **Multi-step
   Split Conformal Prediction (MSCP)** is the benchmarked winner on coverage+efficiency [45],
   or the equally simple **Quantile Residual Simulation** (point forecast + empirical
   error-quantile distribution) [44] — both avoid the **quantile-crossing** failure of naive
   pinball-loss quantile regression [30][44]; (b) for downside risk, **quantile-based VaR/ES**
   beats GARCH on the tails [31]. Always **validate coverage empirically** — several popular
   conformal wrappers (EnbPI/SPCI/Nixtla) *fail* coverage on dependent data [45], and no UQ
   method is universally best [46]. A wide interval straddling zero is the truthful "we don't
   know — holding is fine" signal — a disclosure/decision-quality win independent of alpha;
   the VaR/ES estimate is a natural sizing/throttle input to gate (lean GARCH-conservative
   for the throttle [28]).

9. **Add a staleness monitor to the forward phase, and a minimum-evidence period.**
   A cheap concept-drift detector on the live stream [26] can trigger "re-run the
   bake-off / re-evaluate the crowned pick" instead of letting a stale winner trade —
   more honest than assuming the past window still holds. Pair with a **minimum
   track-record / test-period requirement** [34][20] before any "it's beating B&H"
   claim from forward paper-trading. Both are guardrails, not alpha; expect drift
   detection to lag [11][12].

10. **Harden the bake-off with plateau-selection + a robustness-aware ranking objective.**
    Two concrete, implementable upgrades validated by independent 2026 frameworks: (a)
    **crown a stable parameter *plateau*, not the single highest-Sharpe peak**, and apply a
    **cliff-sensitivity veto** that rejects fragile peaks whose neighbors collapse [40] —
    a precise formalization of robustness our bake-off currently lacks; (b) **rank
    strategies on a robustness-aware composite** (return × significance × equity-curve
    consistency ÷ downside deviation), not raw return/Sharpe [41], which demonstrably
    doubles OOS retention — but swap their normal-i.i.d. Z-score for the skew/kurtosis-aware
    **Deflated/Probabilistic Sharpe** [20] given crypto's fat tails. Combined with a
    **majority-pass-across-folds + catastrophic-veto** verdict [40] (already our weakest-link
    spirit), this fixes *selection* at ranking time, not just via post-hoc veto.

11. **If we ever train a learned component, weight samples for non-IID labels.** Overlapping
    triple-barrier labels are non-IID [42][2]: down-weight by **average uniqueness** and set
    bagging `max_samples ≈ avg uniqueness`; optionally add **learning-trajectory reweighting**
    [32]. Choose its inputs with **permutation/shuffling importance** [32] (≈ MDA [2]) or FSA
    [33], and **calibrate** its output probability (reliability diagram + ECE + Brier →
    isotonic/Platt) [36] before mapping confidence to bet size [15].

## Open questions / things worth testing in our app

- Does a meta-labeling "whether-to-act" filter on an existing strategy beat single-coin
  crypto B&H net of costs through the FROZEN gate + CPCV + DSR? (Most promising
  experiment; prior: maybe a small drawdown/cost win, unlikely a return win.) [15][2]
- Does a persistent (jump-model) vol-regime overlay beat single-coin crypto B&H net of
  costs — or does crypto's lack of an equity-risk-premium tailwind kill the [6]
  result? (Prior: no robust edge.)
- Can we validate our own gate on *synthetic* coins with known-zero edge
  (Heston/Merton/regime sims [14]) and confirm it refuses to crown a winner, and that
  DSR/PBO flag overfit picks?
- Would a random-features + ridge timing overlay [24] show any gated, cost-net edge on
  our coins, or does it collapse under costs as the paper warns? (Prior: collapses.)
- Would fractionally-differentiated price features change which bake-off strategy wins,
  or just add overfitting surface? [2][3]
- Is out-of-sample directional accuracy on our coins ever materially >55% once leakage
  is removed and measured on returns (not levels)? (Prior: no.) [4][5][16][38]
- Would a **conformal/QRS prediction interval** on next-window return (MSCP [45] or QRS
  [44]) be a net honesty improvement for the operator — i.e. does its empirically-verified
  coverage hold on our coins, and does a "lower-bound-acceptable → act, else hold" rule
  (HR-LR [39]) ever clear the gate vs B&H net of costs? (Prior: intervals are honest and
  useful; the act-rule rarely beats holding.)
- Would adopting **plateau-selection + cliff-veto** [40] and a **robustness-aware ranking
  composite** (consistency + downside + fat-tail-correct significance [41][20]) change
  which strategy our bake-off crowns, and improve its forward (OOS) retention vs ranking on
  raw return/Sharpe? (Prior: yes — fewer overfit crowns, more honest winners.)
- Does a **GARCH(1,1)/GARCH-X or QRS-LGBM volatility estimate** [28][44] as a sizing/throttle
  input beat single-coin B&H net of costs, or only reduce drawdown without adding return?
  (Prior: drawdown-only, like [3][6-HMM].)
- Is a **concept-drift detector** [26] on the forward stream a useful re-bake trigger, or
  does its lag make it fire too late to help (and just add churn)? [11][12]

## Paper map (claim -> supporting [N])

- ML return predictability is tiny; gains are cross-sectional → [1]
- Canonical financial-ML pipeline (frac-diff, triple-barrier, meta-label, purged CV,
  deflated Sharpe) → [2]
- Frac-diff + triple-barrier works on crypto but wins only on risk-adjusted terms, in a
  bull window → [3]
- Classical ML beats linear/ARIMA statistically but skips the B&H + cost benchmark →
  [4][16][17]
- Full-series smoothing + long-horizon labels = leakage = fake 90%+ accuracy → [5]
- Persistent regime timing (jump model) can beat equity B&H net of costs; plain HMM
  cannot (on return) → [6]
- PBO/CSCV quantifies selection overfitting; rises with #trials → [7]
- CPCV empirically beats k-fold/walk-forward at controlling overfitting → [14]
- SHAP = principled interpretability; correlated-feature caveat in finance → [8][2]
- ~65% of published anomalies fail to replicate conservatively → [9]
- Double-selection LASSO: most new factors are redundant given the zoo → [10]
- Online change-point detection works but lags the move → [11][12]
- A no-intelligence baseline beats published AI traders under honest testing → [13]
- Meta-labeling (side + secondary filter) improves risk-adjusted metrics; best as a "do
  less" filter; small-sample caveat → [15]
- R²≈1.0 on price *levels* is an illusion; report on returns → [16]
- Feature parsimony beats feature dumping; technical indicators are the workhorse;
  macro/social hurt → [17]
- Robustness comes from shrinkage, not naive sparsity → [18][24]
- Technical-rule profits are a data-snooping artifact (Reality Check / SPA) → [19]
- Deflated/Probabilistic Sharpe = implementable selection-bias correction (needs N,
  skew, kurtosis, T) → [20]
- ML asset-pricing profits concentrate in hard-to-arbitrage names; die under
  turnover/costs → [21]
- Some ML strategies survive costs (~57% haircut) but only broad/high-turnover/deep
  configs → [22]
- Feature engineering + classical ML ≥ deep learning on financial tabular tasks → [23]
- "Virtue of complexity": ridge-regularized overparameterized models predict returns
  better OOS, but need shrinkage and die under costs → [24]
- Conformal prediction gives distribution-free intervals but needs exchangeability;
  non-stationary/regime data requires adaptive/online conformal → [25]
- Concept drift = regime change in ML terms; cheap detectors work but detecting ≠
  profitably reacting (lag) → [26]
- GARCH encodes useful vol stylized facts; a GARCH(1,1)/GARCH-X vol estimate is a strong
  interpretable feature → [27][28]
- GARCH vs ML for vol: ML edges on avg error but ML under-predicts / GARCH over-predicts;
  GARCH safer for a risk throttle → [28]
- Triple-barrier param grid for balanced labels; deep (LSTM) ≈ classical (XGBoost) on
  F1, both barely above base rate → [29]
- Predictability is tail-specific: quantile predictors reverse sign across the
  distribution; forecast distributions not means → [30]
- Distribution-free quantile VaR/ES beats GARCH on tails; good for crypto's fat tails →
  [31]
- Permutation/shuffling feature selection + learning-trajectory sample reweighting for
  low-SNR financial data → [32]
- Feature Selection with Annealing prunes 1,000→few; author caveat "accuracy ≠
  reliability" → [33]
- "Backtest looks great, live fails"; minimal test-period to tell edge from luck (time
  analogue of MinTRL) → [34]
- Online learning for stat-arb: distribution-free, regret-bounded — but regret is
  relative to best fixed rule, not to B&H → [35]
- Calibration (ECE/Brier/reliability diagrams; isotonic/Platt) is mandatory for any
  probability we size on → [36][15]
- Best-designed pro-deep result (futures, cost-breakeven, turnover-efficiency) still
  needs a multi-asset cross-section we can't use → [37]
- Costed RF on SPY minute: train acc 80%→test 48%, test R²<0, all models lose to B&H
  +2.29% → [38]
- Conformal-to-decision bridge: HR-LR rule (high lower-bound = act, else hold); beats
  simple baselines but no costs/3 stocks → [39]
- Independent gate design = ours: IS plateau + cliff veto + WFA majority-pass +
  catastrophic veto + OOS lock → [40]
- Rank on robustness composite (return × sig × consistency ÷ downside), not raw return;
  +98% OOS retention → [41]
- López de Prado's 10 failure modes; non-IID overlapping labels → weight by average
  uniqueness → [42][2]
- Most-cited crypto-ML "beats market" result is survivorship-biased + zero-impact
  assumptions; astronomical returns = red flag → [43]
- GBM (LightGBM) beats HAR/RF for BTC vol; QRS gives calibrated intervals (avoids quantile
  crossing); volume change + lagged RV dominate → [44]
- Conformal method horse-race: MSCP wins coverage+efficiency; EnbPI/SPCI/Nixtla fail
  coverage on dependent data → [45]
- UQ-toolbox review: validate coverage, no method universally best, conformal recalibrates
  any base model post-hoc; GAMLSS models scale/shape → [46]
