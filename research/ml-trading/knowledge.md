# Knowledge — Machine Learning for Trading (classical ML, feature eng., forecasting)

_Synthesized from `papers.md` (source of truth). Final update for this run._
_Progress: 24 papers logged ([1]–[24])._

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
   **shrinkage toward stable/high-variance directions**, not naive sparsity.

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
  is removed and measured on returns (not levels)? (Prior: no.) [4][5][16]

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
