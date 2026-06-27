# Knowledge — Machine Learning for Trading (classical ML, feature eng., forecasting)

_Synthesized from `papers.md` (source of truth)._
_Progress: **100 papers logged — topic target reached.** Round 2 added [25]–[46]: conformal
prediction & calibration, GARCH & quantile/probabilistic risk forecasting, feature selection &
sample reweighting, concept drift / online learning, costed-backtest negative results, crypto-ML
overfitting, gradient-boosting volatility, and gate/objective-design papers._
_Round 3 added [47]–[78]: thin-seam fills — time-series classification (ROCKET/MiniROCKET
[47][54], InceptionTime [55]); the HAR-RV volatility family [48]; the MDA-vs-MDI feature-
importance debate corrected (Sobol-MDA [49]); crypto information-driven bars + triple-barrier
labeling [50]; isotonic-vs-Platt calibration choice [51]; ensemble stacking with a leakage
warning [52]; imbalanced rare-event handling — skip-SMOTE [53]; plus the catch22/FRESH feature
seams, MCS/forecast-combination [69][76], and the interpretation-stability capstone [78]._
_Round 3-final added [79]–[100]: the **boosting/ensemble foundations** (XGBoost [79], LightGBM
[80], CatBoost [81], Friedman GBM [82], Breiman Random Forests [85], Super Learner/stacking
[88], SVM [86]) — the named classical learners, anchored from first principles; **regime
detection breadth** (Wasserstein clustering [83], HMM factor-switching [84]); the **selection-
bias root** (Pseudo-Mathematics + MinBTL [95]) and **clustered feature importance** [94] (the
LdP fix for correlated-feature unreliability); **on-domain costed crypto negatives** (Bysik–
Ślepaczuk hourly-BTC walk-forward, Holm-corrected → no strategy beats B&H [89]; 41-model
Bitcoin survey with forward-test collapse [90]); **classical-baseline anchors** (M4 — pure ML
loses to simple statistical methods [91]; Stronger-Baselines capstone [100]); **crypto
efficiency** (daily=random-walk, weekly=weak mean-reversion [96]); **trend/factor reality**
(Time-Series Momentum [97], crypto high-dim factor ML [98], crypto Lasso factor model [99]);
and a financial concept-drift template [92]._

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

15. **Data leakage is the field-wide engine of false results — and it has a checklist.**
    Kapoor & Narayanan [56] find leakage across 17 scientific fields / ~329 papers, with an
    **eight-type taxonomy** (no clean split; pre-processing/feature-selection on combined
    sets; illegitimate/future-encoding features; temporal leakage; non-independence) and a
    **"model info sheet"** remedy. Several types ARE the financial-ML sins already in this
    ledger: full-series smoothing/normalization [5], frac-diff-d/selector fit on all data
    [2][5], look-ahead [4][5], overlapping-label non-independence [2][42]. Their marquee
    result — in civil-war prediction, **every "complex ML beats logistic regression" claim
    failed once leakage was fixed** — is our thesis in another field [13][38]. Bake an
    eight-point leakage-audit into the engine for any learned component.

16. **For time-series classification, random-convolution (ROCKET) is the pragmatic
    sweet spot — and even the TSC field admits complexity ≠ progress.** The authoritative
    TSC benchmark [54] ranks Hydra-MultiROCKET / HIVE-COTE v2 top, with **MiniROCKET
    near-top at >10× the speed** and a linear-classifier head (Rust-friendly, overfitting-
    resistant — random features + ridge, the sequence analogue of [24]). Its authors warn
    the deep-TSC literature routinely does **model selection on test data** and that a flood
    of new deep methods **didn't beat InceptionTime** [55] — a field-internal echo of our
    skepticism [13][19]. Hard caveat: all this accuracy is on UCR datasets with *real* class
    structure; financial direction/triple-barrier labels are near-noise [29][38], so TSC
    accuracy says nothing about a tradable edge. If sequence features are ever wanted →
    **MiniROCKET, gated**; shapelets [60][61] are heavier and background vs ROCKET.

17. **Tree feature-importance is unreliable on BOTH axes — use retrain-without / TreeSHAP.**
    The MDA-vs-MDI debate resolves *against trusting either default*: **MDI (gain) is
    inconsistent** (can *lower* a feature's importance when its true impact rises) and biased
    by collinearity/cardinality [58]; **MDA (permutation) is also unreliable under correlated
    features** — it estimates a confounded quantity whose spurious term grows with dependence
    [49]. Since financial features heavily co-move [17], the honest tools are the consistent
    **TreeSHAP** (+ **SHAP interaction values** for regime-conditional vol×trend structure)
    [58][8] and the expensive-but-correct **drop-column / Sobol-MDA** [49]. Interpretability
    still ≠ edge [8][58]; feature interactions in markets are **regime-dependent** [59][30].

18. **For rare-event / imbalanced labels, skip SMOTE; pick a robust calibrated model.**
    Take-profit events are a rare imbalanced class. The benchmark [53] finds **explicit
    rebalancing (SMOTE/over/under-sampling) is often unnecessary and can hurt** — synthetic
    minority data distorts the distribution (acute in low-SNR finance), undersampling
    discards signal. Better: a **robust probabilistic classifier (gradient-boosted trees),
    threshold-tuning, PR-curve/G-mean metrics (not accuracy), then calibration**. And the
    calibrator choice: **Platt scaling beats isotonic when calibration data is scarce**
    [51] (our single-coin regime) — isotonic overfits small sets. Always verify calibration
    on held-out data [36][51], the same "validate, don't assume" rule as conformal coverage [45].

19. **The HAR-RV family is the simple, strong realized-volatility baseline.** HAR-RV
    (regress RV on lagged **daily + weekly + monthly** RV) is a trivially-Rust-implementable
    linear model that **beats GARCH/ARFIMA** at vol forecasting [48] — arguably a better
    default vol feature than GARCH(1,1), or complementary (HAR=multi-scale persistence,
    GARCH=clustering/leverage [27][28], the GBM study [44] also benchmarked HAR). Path-
    dependent / semivariance extensions add marginal statistical gains [48] but carry the
    field-standard **no-costed-trade / no-B&H** caveat — a better vol forecast still must
    prove it beats holding net of costs (prior: drawdown-only [3][6-HMM]).

20. **Feature search needs a multiple-testing gate too — the feature analogue of DSR.**
    Mass-generating features (tsfresh/FRESH [63], ROCKET's 20k features [47]) creates the
    same selection-bias as baking off many strategies: some look predictive by chance. The
    honest control is an **FDR filter (Benjamini-Yekutieli)** on feature relevance [63] —
    the feature-side twin of deflated-Sharpe/PBO/SPA on strategies [19][20][7]. But univariate
    relevance tests miss interactions and keep correlated redundancy [49][58], and mass-
    generation is itself the kitchen-sink trap parsimony warns against [17][23]: prefer a
    small curated technical set first; auto-extraction + FDR is a disciplined fallback, not
    a default.

21. **Boosting/ensemble are *tools*; their regularizers — not their pedigree — decide
    generalization, and even leak-hygiene only buys a more trustworthy (usually null) result.**
    The named classical learners are now anchored from first principles: gradient boosting is
    **steepest descent in function space** (Friedman [82]), so XGBoost's second-order objective
    + L1/L2/γ leaf penalties [79], LightGBM's leaf-wise growth + GOSS/EFB [80], and CatBoost's
    **ordered boosting** (which fixes a *target-leakage / prediction-shift* inside the boosting
    loop [81]) are all refinements whose **regularizers (shrinkage ν, subsampling, num_leaves,
    min_data_in_leaf, depth) are the real defense against memorizing single-coin noise** — to be
    tuned under CPCV [14], never on test. Random forests generalize via Breiman's
    **strength-vs-correlation bound** [85], but OOB error assumes IID and is *no substitute*
    for purged CV on overlapping labels [2][42]. All three GBDTs natively accept **per-sample
    weights** → the average-uniqueness non-IID fix [2][42] is free. Hard through-line: these
    libraries dominate on data with *real* signal; on near-noise crypto direction labels [29][38]
    they overfit as readily as anything, so the **gate, not the learner, governs** — and CatBoost's
    leak-hygiene just makes the (likely "no edge") verdict more trustworthy.

22. **Ensembling cannot beat a benchmark already in its library — a theorem-shaped echo of our
    thesis.** Stacking / Super Learner [88] combines a model *library* via cross-validated
    out-of-fold predictions and a meta-learner, with an **oracle inequality**: asymptotically it
    performs **as well as the best single candidate — no better**. So if **buy-and-hold is in the
    library** and no active strategy robustly beats it [13][19][89], the optimal stack *cannot*
    beat B&H either — ensembling is a defense against picking the wrong model, **not** a source of
    new alpha. The out-of-fold construction must be **purged/embargoed** [2][14] (naive stacking on
    overlapping labels leaks [42]), and the meta-learner adds a selection layer the deflated-Sharpe
    penalty must still cover [20]. The M4 competition [91] empirically agrees from the other side:
    **pure ML lost to simple statistical methods on 100k series; only *combinations/hybrids* won**,
    and only modestly — ensembling done right helps, but it doesn't manufacture edge.

23. **Regime detection has three branches (HMM, change-point, clustering) that all share two
    flaws: lag and "detection ≠ profit."** Beyond the HMM [6][67][84] and change-point [11][12]
    branches, **distributional clustering** (Wasserstein/MMD k-means [83]) is the third — it
    compares whole return distributions (mean/var/skew/tail) and is conceptually the right way to
    label "what regime are we in," but [83] is explicit that it does **only identification, no
    backtest**, and like all regime methods it is **backward-looking** (a window's distribution is
    known only after observing it) → forward use inherits detection lag [11][12]. The HMM
    factor-switching upside [84] (switch *which* strategy is active by regime) is a real design
    idea, but its benchmark was other factor models (not B&H), its OOS window was short and
    crash-dominated, and plain Gaussian HMMs are flip-prone [6]. Net for us: regime is a **risk
    throttle / strategy-selector candidate**, prefer a *persistent* labeler (jump model [6]) over
    flip-prone HMM/heavy Wasserstein, and gate every regime claim against B&H net of costs.

24. **On-domain, cost-and-multiple-testing-correct crypto studies now directly validate the
    product thesis.** The cleanest is Bysik–Ślepaczuk [89]: hourly BTC, **27-fold walk-forward**,
    **10 bp/turn costs**, XGBoost/LSTM/iTransformer — frictionless looks great but **collapses net
    of costs** (sign-based XGBoost **+73.5% → −64%/yr**); a **cost-aware filter** (act only when
    expected move > cost hurdle) recovers it to ~65%/yr @ Sharpe ~1.09, but **after Holm correction
    no cost-aware strategy significantly beats buy-and-hold**. This is our thesis on our asset with
    our discipline. The 41-model Bitcoin survey [90] shows the failure mode by omission: PNL-tuned
    over 41 models × many windows (textbook data-snooping [19][95]), it admits backtests **don't
    translate forward**, and reports **no costs, no B&H**. Joins [13][38][91][100] as the honest-
    testing set — and hands us a ready-made **cost-aware-filter** baseline overlay.

## Methods / findings that hold up (and which don't)

**Hold up:**
- **CPCV** (purged + embargoed + combinatorial) and walk-forward for honest OOS
  evaluation [1][4][6][14][89]; **MinBTL** as a crown pre-condition [95].
- Selection-overfitting gates: **Reality Check / SPA** [19], **PBO/CSCV** [7],
  **Deflated/Probabilistic Sharpe** [20], deflated Sharpe [2]; **Holm/multiple-testing
  correction across the strategy set** confirmed on-domain [89]; **double-selection LASSO**
  for "is this signal marginally useful?" [10][99].
- **Triple-barrier labeling** [2][3][50]; **fractional differentiation** fit on train
  only [2][3].
- **Meta-labeling** as a low-risk *filter to trade less* [2][15]; the **cost-aware filter**
  (act only when expected move > cost hurdle) is the implementable form [89].
- **Tree ensembles > linear** on tabular features [1][4][17]; **feature engineering +
  classical ≥ deep learning** for financial tabular tasks [23]; **pure ML < simple
  statistical methods; combinations/hybrids win** at scale [91].
- Boosting **regularizers** (shrinkage ν, subsampling, leaf/depth limits) and the RF
  **strength-vs-correlation** intuition as the source of generalization [79][80][82][85];
  GBDT **per-sample weights** for non-IID labels [2][42][79][80].
- **Stacking / Super Learner** as the principled aggregator — but only as good as its best
  candidate (oracle inequality) [88].
- **Persistent regime timing (jump model)** can beat equity B&H net of costs [6];
  **distributional (Wasserstein) clustering** is the right regime *labeler* concept [83].
- **Shrinkage/regularization** as the source of OOS robustness [18][24].
- **Parsimonious technical-indicator features**; causal/selective beats kitchen-sink
  [17][10]; **clustered feature importance / Clustered-MDA** is the substitution-robust,
  leak-safe importance+selection tool [94].
- **Strong, utility-aligned baselines** (a tuned baseline + a money-vs-B&H metric, not
  accuracy/AUC) as the test that separates real ML value from illusion [100][13][91].

**Do NOT hold up / red flags:**
- Direction-accuracy from full-series smoothing + long-horizon sign labels [5].
- Any metric with no B&H/random-walk baseline and no costed backtest [4][16][17][90];
  a **weak/strawman baseline** is as misleading as none [100].
- **R² on price levels** as success [16].
- Plain **HMM** regime timing for *return* enhancement (flip-prone) [6]; regime
  *identification* presented as a tradable signal (it's backward-looking + lagged) [83].
- Single-bull/low-vol-window evaluation [3][21]; under-powered evaluation [13];
  short, single-crash-dominated OOS windows for regime timing [84].
- Bolting on macro/social-media features expecting gains [17][98].
- Technical trading rules as alpha once data-snooping is corrected [19].
- **Tuning the reported metric (PNL) across a large model/param grid without a
  multiple-testing penalty** — manufactures an in-sample winner that fails forward [90][95][7].
- **Frictionless** crypto strategy results — costs at sign-flip turnover can flip
  +73% to −64% [89].
- **Pure ML** expected to beat simple statistical baselines on its own [91].
- Trusting OOB error / naive k-fold on overlapping financial labels [85][2][14].
- **Unregularized** complexity [24]; assuming "smarter execution" rescues a marginal
  edge [22].

## Actionable takeaways for our advisor

1. **Upgrade the bake-off gate with multiple-testing corrections — we have the inputs
   for free.** Our gate already knows **N** (strategies/params tried) and each
   strategy's per-period returns. Add, alongside the 1000-path moving-block bootstrap:
   (a) a **Deflated Sharpe Ratio** + Probabilistic-Sharpe for the crowned strategy
   [20] (corrects N, sample length, skew, kurtosis — apt for crypto fat tails); (b)
   optionally a **Hansen SPA p-value** [19] or **PBO via CSCV** [7]; (c) a **MinBTL
   pre-condition** [95] — given N, compute the minimum window length below which the
   crowned Sharpe is expected to be a pure overfitting artifact, and **refuse to crown
   if our window is shorter** (cheap, new, honest veto we lack). Treat DSR<benchmark /
   high PBO / weak SPA / window<MinBTL as a veto. An independent on-domain study [89]
   confirms the payoff: after **Holm correction across the strategy set**, no cost-aware
   hourly-BTC strategy significantly beat B&H — so report N and a multiple-testing-
   corrected significance, not a point estimate. This closes the *selection*-robustness
   axis the bootstrap alone doesn't. (Memory/serial-correlation makes overfit crypto
   picks not just zero- but *negative*-expected OOS [95] — another reason the veto matters.)

2. **Hard test-data rules (codify in the engine).** (a) Fit every transform on the
   in-sample window ONLY, apply forward [5]. (b) Report R²/error on **returns, never
   price levels** [16]. (c) Always show a naive baseline (B&H / predict-last)
   [4][13][16]. (d) Make **transaction costs + turnover penalty first-class**; a
   strategy must beat B&H *net of realistic costs* [21][22]. (e) Stress across
   regimes, not just calm/bull windows [3][21].

3. **Meta-labeling is the top ML experiment — as a "do less" filter; start with the
   cost-aware filter as the baseline.** Keep our simple strategy as the *side*; add a
   small interpretable classifier (tree/logit) to decide *whether to act*, triple-
   barrier-labeled [2], CPCV-validated [14], gated vs B&H net of costs [15]. The simplest
   instance — proven to matter on hourly BTC — is the **cost-aware execution filter: act
   only when the expected move exceeds the transaction-cost hurdle** [89] (the difference
   between +73.5% and −64%/yr there). Most plausible win: fewer, higher-confidence trades
   → less cost drag. Cannot underperform "always act" if gated. If we ship a GBDT meta-
   labeler, use **CatBoost-style leak-safe encoding** for any categorical regime input
   [81], **per-sample average-uniqueness weights** [2][42], and **Clustered-MDA** [94] for
   the operator-facing "why it acted."

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

6. **Keep the feature set small and technical; regularize when combining; diagnose with
   substitution-robust importance.** Don't add macro/social-media data — it tends to hurt
   and inflates overfitting surface [17][10][98]. When blending signals, prefer shrinkage
   over lucky sparse selection [18], or a **double-selection LASSO** to keep only marginally-
   useful signals [10][99]. Diagnose feature value with **Clustered Feature Importance /
   Clustered-MDA** [94] (group co-moving features, shuffle the group) rather than naive
   MDA/MDI/SHAP, which lie under the correlated features that dominate finance [49][58][8][78];
   pair with a bootstrap **interpretation-stability check** [78] and report a spread, not a
   point. CFI doubles as **cluster-based selection** (one representative per cluster) [94].

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
  does its lag make it fire too late to help (and just add churn)? [11][12][92]
- Does adding a **MinBTL pre-condition** [95] to the gate (refuse to crown when the window is
  shorter than the trial-count-implied minimum) ever change a crowning on our coins/windows —
  i.e. are we sometimes crowning on too-short a history given how many strategies we bake off?
  (Prior: yes, occasionally — and the memory-effect negativity [95] makes those crowns
  actively harmful, not just lucky.)
- Does a **cost-aware execution filter** (act only when the expected move exceeds the
  transaction-cost hurdle) [89] — the simplest "do less" overlay — beat single-coin B&H net of
  costs through the gate, or (as on hourly BTC [89]) merely recover viability without
  significantly beating holding after multiple-testing correction? (Prior: recovers viability,
  doesn't beat B&H — but it's a strong default overlay regardless.)
- If we ever ship a GBDT meta-labeler, does **Clustered-MDA** [94] give a *stable* (bootstrap-
  checked [78]) operator-facing "why it acted" story on our co-moving technical features, where
  naive MDA/SHAP [49][58] do not? (Prior: yes for stability of *groups*; individual-feature
  attributions stay unstable.)

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
- Boosting = steepest descent in function space; shrinkage/subsampling/robust losses are the
  foundation → [82]
- XGBoost = regularized GBDT (L1/L2/γ leaf penalty, sparsity-aware, weighted quantile sketch);
  regularizers are the overfitting defense → [79]
- LightGBM = fast GBDT (GOSS/EFB, leaf-wise growth); num_leaves/min_data_in_leaf are the
  regularizers; speed buys more thorough validation → [80]
- CatBoost = ordered boosting fixes target-leakage/prediction-shift *inside* the boosting loop;
  ordered target stats for leak-safe categorical encoding → [81]
- Random Forests = de-correlated trees; strength-vs-correlation generalization bound; OOB ≠
  purged CV on overlapping labels → [85]
- SVM ~60% weekly-direction on NIKKEI, beats LDA/QDA/NN — but no costs, no B&H benchmark → [86]
- Stacking / Super Learner: oracle inequality → only as good as best candidate; can't beat B&H
  if B&H is in the library; out-of-fold must be purged → [88]
- Pure ML < simple statistical methods on 100k series (M4); combinations/hybrids win, modestly →
  [91]
- MinBTL: minimum window given N or the best in-sample Sharpe is spurious; memory effects →
  *negative* OOS; report N or it's "pseudo-mathematics" → [95]
- Clustered Feature Importance / Clustered-MDA: substitution-robust importance + selection (the
  fix for correlated-feature unreliability) → [94]
- Hourly-BTC walk-forward, 10bp costs: frictionless +73.5% → −64% net; cost-aware filter
  recovers; Holm-corrected, no strategy beats B&H → [89]
- 41-model Bitcoin survey: PNL-tuned across big grid (data-snooping), backtests don't translate
  forward; no costs, no B&H → [90]
- Crypto efficiency: daily returns ≈ random walk (efficient/unpredictable), weekly ≈ weak
  mean-reversion (structural-break-robust tests) → [96]
- Time-series momentum is real but a diversified cross-asset long/short vol-scaled effect; n=1
  long-only can't harvest it; ~1yr then reversal → [97]
- Crypto cross-sectional ML: OOS R²~4.9% (> equities), but cross-sectional + on-chain-
  fundamentals-driven + long/short — doesn't transfer to one long-only coin → [98]
- Most crypto anomalies don't survive OOS (size effect vanishes); double-selection LASSO → DS3;
  left-tail-risk effect appears → [99]
- Stronger baselines often match/beat complex ML; weak/absent baseline + accuracy-not-utility
  metric manufactures false ML superiority (cross-domain capstone) → [100][13][91]
- Distributional (Wasserstein/MMD) clustering = the right regime-*labeler* concept, but
  identification-only, backward-looking, lagged → [83]
- HMM factor-switching upside (switch which strategy by regime); but benched vs factors not B&H,
  short crash-dominated OOS, flip-prone → [84]
- Concept drift in financial series: detect-drift → re-fit (OS-ELM + explicit detector);
  template for our re-bake trigger, but detection lags → [92]
