# Knowledge — Evolution (Evolutionary / Genetic Methods & Automated Alpha-Search)

Synthesized findings for OUR app — a Rust single-coin crypto **advisor** (paper/sim
only) that bakes off strategies, ranks them under a FROZEN 1000-path moving-block
bootstrap gate (weakest-link verdict; buy-and-hold always the benchmark), then
paper-trades the winner. Validated thesis: **no active strategy robustly beats
holding, net of costs.**

Skeptical prior for THIS topic: evolutionary alpha-search is, by construction, a
multiple-testing machine. The central question is whether any of it survives an
honest, cost-aware, out-of-sample, regime-spanning test — or whether it is just
industrialized overfitting our gate exists to reject.

## Key themes

1. **The data-snooping core.** GP / GA / RL / MCTS / symbolic-regression alpha
   miners all *search a vast space against backtest feedback*. That IS multiple
   hypothesis testing at scale; absent correction it manufactures in-sample winners
   that die out-of-sample. The fancier the search (LLM-as-mutation [9], MCTS
   [8][12], tree-structured thoughts [11]), the *faster* it finds spurious patterns
   — the statistics that doom it are unchanged. Even the careful miners admit it:
   TreEvo [11] states outright "increasing the number of evaluations may make the
   methods more prone to overfitting." This is not folklore — Bailey et al. [29]
   *prove* that backtesting even a *small* number of configurations yields a high
   in-sample Sharpe by chance, and that the in-sample winner is *negatively*
   correlated with out-of-sample return.
2. **Honest GP says our thesis.** The foundational GP-trading paper (Allen &
   Karjalainen 1999, [1]) — with a proper train/select/test split + costs — found
   evolved rules do **not** robustly beat buy-and-hold on the S&P out-of-sample.
   25 years later the careful crypto walk-forward / double-OOS study [5] reaches the
   same verdict and shows optimized params beat *random* params only 8–13% of the
   time; the BTC ML-under-costs study [32] — using a B&H benchmark, 10 bp costs,
   block-bootstrap *and* Holm multiple-testing correction — finds even a >65%-
   annualized strategy does **not** significantly beat holding.
3. **Negative results when benchmarked honestly.** NEAT neuroevolution [7] *loses*
   to B&H by ~9 pts over 22 years (before costs). NSGA-II multi-objective [27] and
   GP+directional-changes [28] deliver *lower risk*, not alpha. The pattern: active/
   evolved strategies, when actually compared to holding net of costs, deliver
   *risk reduction at best, not alpha*.
4. **The alpha-factor paradigm is cross-sectional & institutional.** 101 Formulaic
   Alphas [3]: industrial alphas are individually weak, short-horizon, low-
   correlation, useful only *combined across hundreds* with heavy turnover. That
   regime is structurally inapplicable to a single-coin retail advisor.
5. **Metric substitution hides the truth.** Alpha miners report **IC / predictive
   accuracy**, not **PnL net of costs vs B&H**. IC ~0.04–0.09 ([2][6]) is a weak
   correlation that need not monetize once turnover + fees bite.
6. **"Optimization solver" ≠ "trading edge."** A large slice of "evolutionary +
   portfolio" work (ARO [16], swarm/LLSO [17], NSGA-II variants) just finds *better
   points on the Markowitz constrained frontier* — a solver-quality contribution
   that assumes the model and never claims OOS outperformance. For a single coin the
   allocation/cardinality problem is moot, so these don't apply; always check
   whether a paper claims *solver quality* (common, irrelevant to us) or *realized
   OOS edge vs B&H* (rare).
7. **The evaluator is everything.** FunSearch [18] proves LLM-driven evolutionary
   *program search* can find genuinely new, *provably* better algorithms — but only
   because it has a sound, cheap, *correct* evaluator (a math verifier). Ported to
   trading, the evaluator becomes a noisy, finite, gameable backtest; the same
   machinery then finds *spurious* winners. The robustness gate is our (imperfect)
   stand-in for a verifier; markets have no ground-truth one.
8. **Optimizer choice interacts with (strategy, asset).** Bayesian (TPE) vs
   evolutionary (DE) on crypto [13]: no globally best optimizer — parameter-space
   "viable fraction" varies 8–18× across strategy×coin pairs. Supports our
   per-(coin,window) re-bake; warns that tiny-viable-region configs are the most
   fragile/overfit.
9. **GP overfits by its nature.** A general GP-methodology study [21] establishes
   that GP overfitting "cannot be solved or suppressed as easily as in more
   traditional approaches" — model selection only abates it. Every GP/SR alpha miner
   here ([1][9][10][11][12][19][20]) inherits this baseline tendency *before*
   financial-noise + multiple-testing hazards.
10. **"Risk-seeking" objectives are a tell.** Deep Symbolic Regression [24]
    introduced the top-ε risk-seeking policy gradient (great when a ground-truth
    equation exists); RiskMiner [12] ported it to alphas. On noisy data with no
    ground truth, optimizing best-case = chasing lucky tails = overfit. Our gate
    should reward robust central tendency vs B&H, never best-case.
11. **Forecasting accuracy ≠ tradeable edge — restated for evolution.** GA used for
    hyperparameter optimization of forecasters ([22] AUROC ~0.60, [25] MAPE) lowers
    a *prediction-error* loss but never crosses to cost-net PnL vs B&H. Low MAPE on a
    trending index ≈ "tomorrow = today + drift," which B&H already captures free.
12. **Beat a RANDOM null, not just B&H.** Chen & Navet [30][31] propose pretests:
    if a GP-evolved strategy can't beat **zero-intelligence / lottery (random)
    trading**, the "failure" is the algorithm/efficiency, not a tuning gap to chase
    — and the apparent edge is noise. The crypto walk-forward bootstrap-vs-random
    [5] is the same idea quantified. A random-strategy null is a cheap, powerful
    sub-test our gate currently lacks.

## Methods / findings that hold up (and which don't)

- **Holds up:** the *discipline* — train/select/OOS test + transaction costs +
  bootstrap-vs-random as an overfitting detector ([1][5]). Double-out-of-sample
  (optimize on global-train via walk-forward, evaluate ONCE on a never-touched
  unseen period) [5] is a directly reusable protocol.
- **Holds up (a reusable overfitting diagnostic):** MadEvolve [14] compares observed
  IS→OOS degradation against **multiple-testing theory** (Bailey et al. 2014) — if
  degradation stays *below* the theoretical p-hacking baseline, the gain is more
  likely real. Also: **cost as a market-impact penalty inside the fitness function**
  (not an afterthought) and **scale-invariant metrics** (Sharpe/Calmar can't come
  from mere position-sizing) to rule out sizing artifacts.
- **Holds up (as a risk idea, not alpha):** "B&H + strategy" portfolios reduce
  drawdown even when the strategy alone doesn't beat B&H ([5]); dynamic re-weighting
  to combat **factor decay** ([6]) flags that edges are non-stationary.
- **Does NOT hold up:** headline PnL from cost-blind, single-window, no-benchmark,
  re-optimized-on-the-same-stream optimizers ([4], +550% scalping). The "agent /
  LLM" wrapper around a GA changes nothing statistically. Risk-*seeking* objectives
  that chase best-case tails ([12]) overfit by construction (degrade past α>0.85).
- **Recurrent blind spots even in "good" papers:** missing **buy-and-hold**
  benchmark ([2][8][11][12][14]) and missing/implicit **transaction costs**
  ([2][8][11][12]); IC/accuracy reported instead of cost-net PnL. Even MadEvolve
  [14] — the most self-aware — measures gains vs its *own pre-evolution baseline*,
  not vs holding the coin.
- **Open:** whether *any* evolutionary search clears a cost-aware, regime-spanning,
  B&H-benchmarked gate on a single liquid coin. No paper seen so far does.

## Actionable takeaways for our advisor

1. **Treat our bake-off as a search that must be significance-charged.** Sweeping
   many strategies/params on `(coin, window)` IS a multiple-testing exercise. Borrow
   deflated-Sharpe / PBO ideas; the more configs we try, the higher the bar a winner
   must clear vs B&H. Bailey [29] proves even a *few* configs inflate in-sample
   Sharpe, and [32] shows **Holm correction** flips a >65%-return BTC strategy to
   "not significant vs B&H" — exactly the posture our gate should keep.
2. **Add a RANDOM-strategy null as a gate sub-test (highest-value new idea).**
   Beyond "beat B&H," require a tuned/evolved pick to beat a **matched-activity
   random-trading / random-parameter null** ([5][30][31]). An optimized config that
   beats <~50% of random sets is overfit, not skilled — cheap and catches edges that
   look good vs B&H purely from lucky timing.
3. **Double-out-of-sample is the gold standard** ([5]): if we add param tuning,
   reserve a final unseen window evaluated exactly once; beware "OOS" data reuse.
4. **A cost-aware execution filter is the one thing that helped** ([32]): only trade
   when the expected move exceeds a cost-tied threshold. It cut turnover enough to
   make an ML strategy gross-viable (though still not B&H-significant) — worth
   testing as a turnover-reducing strategy modifier, not as an alpha claim.
5. **Factor/strategy decay is real** ([6]): consider a periodic re-bake cadence for
   a crowned pick; combine with the [13] finding that the best optimizer/strategy is
   per-(coin,window), so re-baking should re-select, not just re-fit.
6. **Don't import the cross-sectional alpha-factor paradigm** ([3]): single-coin
   retail can't diversify across weak short-horizon alphas; it just pays turnover.
7. **If we ever run a search, copy MadEvolve's honesty kit** ([14]): cost-in-fitness,
   scale-invariant metrics, and compare IS→OOS degradation against multiple-testing
   theory — but keep **buy-and-hold** as the benchmark it omitted.

## Open questions / things worth testing in our app

- **Random-null sub-test:** add "beat matched-activity random trading" alongside
  "beat B&H" in the gate. Does any crowned pick survive both? ([5][30][31])
- **Cost-aware execution filter** as a strategy modifier: does threshold-gated
  trading reduce turnover enough to change any verdict? ([32])
- Is there a re-bake cadence that genuinely helps vs. just churns costs? ([6][13])
- Does adding evolutionary search to our bake-off ever produce a config that clears
  the FROZEN gate — or only ever in-sample winners? (Hypothesis: only in-sample;
  [29] predicts the winner is *negatively* predictive OOS.)

## Paper map (claim → supporting [N])

- Honest GP/GA fails to beat B&H OOS net of costs → [1][5][7][32]
- Evolutionary/alpha search is industrialized multiple-testing → [2][4][6][8][9][11][12]
- THEORY: even few configs inflate in-sample Sharpe; IS↔OOS negatively correlated → [29]
- Search admits its own overfitting as eval budget grows → [11][14]
- Optimized params barely beat random params → [5]
- Beat a random / zero-intelligence null, not just B&H (pretests) → [5][30][31]
- Holm/multiple-testing-corrected bootstrap-vs-B&H on BTC → null result → [32]
- Cost-aware execution filter (trade only above cost threshold) → [32]
- Evolved strategies give lower risk, not alpha (multi-objective) → [7][27][28]
- Alpha factors are weak/short/cross-sectional, not single-coin tools → [2][3][6]
- IC / accuracy ≠ tradeable edge net of costs → [2][6][8][11][12]
- Recurrent missing B&H benchmark → [2][8][11][12][14]
- Recurrent missing/implicit transaction costs → [2][8][11][12]
- Risk reduction (lower drawdown / vol), not alpha → [5][7]
- Factor decay → dynamic re-weighting / re-bake → [6]
- Double-OOS + bootstrap-vs-random as overfitting detectors → [1][5]
- IS→OOS degradation vs multiple-testing theory as a diagnostic → [14]
- Cost as market-impact penalty inside fitness; scale-invariant metrics → [14][17]
- "Optimization solver" ≠ "trading edge" (Markowitz frontier solvers) → [16][17]
- Optimizer choice interacts with (strategy, asset); per-pair tuning → [13]
- Evolutionary program search needs a sound evaluator (FunSearch) → [18]
- Vectorial / strongly-typed GP as better rule representation → [10]
- GP overfits by nature; model selection only abates it → [21]
- "Risk-seeking"/best-case objectives overfit on noisy data → [12][24]
- Forecast-error/AUROC win ≠ cost-net edge vs B&H → [22][25]
- Overfitting-probability test as a candidate filter (PBO-like) → [14][23]
- "GP/ML beats B&H on BTC" claims that omit costs/regimes → [19][20]
