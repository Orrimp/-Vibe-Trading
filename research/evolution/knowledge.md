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

> **Status (2026-06-27): 100/100 papers reviewed.** After a full sweep of the GP/GA/
> symbolic-regression/neuroevolution/swarm/MOEA/LCS/code-evolution literature, the
> bottom line is unchanged and *strengthened*: **no paper exhibits a single-coin,
> long-only, cost-net, regime-spanning, B&H-benchmarked, multiple-testing-corrected
> edge.** Every "evolution beats the market" result, on inspection, rides a structural
> advantage our product is scoped out of — **HFT latency / order-flow front-running**
> ([93][94]), **cross-sectional breadth / long-short** ([49][63][88]), **carry/funding**
> ([49]), **leverage** ([83]), or **index-reconstitution flows** ([88]) — or omits costs/
> B&H/OOS/correction ([4][20][44][96]). The honest, careful papers repeatedly land on
> our exact verdict (great in-sample → fails net of costs OOS: [1][5][48][86][95]; or
> risk-reduction-not-alpha: [7][27][72][85]). The most *portable* outputs are
> defensive: the **Deflated-Sharpe formula** [98] and **PBO/Reality-Check** [50][53]
> for our planned gate addition, a **random-strategy null** [30][31], **event-driven
> ("update-when-required") re-baking** [97], and anti-overfitting **fitness/training-set**
> techniques ([68][92][99]).
>
> **Deep-read pass (2026-06-28): 10 high-value entries upgraded to first-hand full reads**
> — [98] DSR, [29] MinBTL, [50] STW Reality Check, [30]/[31] Chen-Navet random nulls,
> [68] GT-Score, [92] dynamic-subset GP, [10] vectorial GP, [48] Neely-Weller. **The two
> defensive formulas we are adding to the gate are now captured exactly** (DSR Eq.2 +
> expected-max-SR Eq.1 from the [98] primary; MinBTL closed form + the years-vs-trials
> table from [29]). Three things sharpened by full text: (a) **DSR's worked example proves
> fat tails *shrink* the survivable trial budget** (Normal returns: 88 trials OK; skew−3/
> kurt10: only 46) — heavy-tailed crypto should make us *more* suspicious of big sweeps,
> not less; (b) the random-null must be **matched-activity** (same trade frequency *and*
> time-in-market) to be cost-fair [31], and the search-vs-search comparison must be
> **equal-intensity** (draw ~N random configs); (c) **even snooping-corrected, in-sample-
> significant rules fail OOS** — STW's best DJIA rule survived the data-snooping correction
> *in-sample 1897-1986* yet was insignificant OOS 1987-1996 (Reality-Check p≈0.12) and
> earned nothing on S&P futures [50]. One **correction**: [68] GT-Score *does* use a B&H
> benchmark (inside its significance Z-score) and *does* include a cost-sensitivity check —
> my earlier "omits both" was wrong. Net effect on the thesis: **unchanged and better-
> armed** — the honest full reads converge on "no robust single-coin edge," and we now hold
> the exact closed forms to enforce it.

## Key themes

1. **The data-snooping core.** GP / GA / RL / MCTS / symbolic-regression alpha
   miners all *search a vast space against backtest feedback*. That IS multiple
   hypothesis testing at scale; absent correction it manufactures in-sample winners
   that die out-of-sample. The fancier the search (LLM-as-mutation [9], MCTS
   [8][12], tree-structured thoughts [11]), the *faster* it finds spurious patterns
   — the statistics that doom it are unchanged. Even the careful miners admit it:
   TreEvo [11] states outright "increasing the number of evaluations may make the
   methods more prone to overfitting." This is not folklore — Bailey et al. [29]
   *prove* (via Extreme Value Theory) that the expected *maximum* in-sample Sharpe
   across N trials with **zero true skill** grows with N, and that the in-sample
   winner is *negatively* correlated with out-of-sample return. **The quantitative
   handle (now read first-hand) is the Minimum Backtest Length** [29]: MinBTL ≈
   [(1−γ)·Z⁻¹(1−1/N) + γ·Z⁻¹(1−1/(N·e))]² / E[max SR]² (γ≈0.5772; Z⁻¹ = inverse
   normal CDF). For a target annualized E[max SR]=1 it gives **N=10 trials → ~0.5 yr
   needed, N=50 → ~1.5–2 yr, N=100 → ~2.5–3 yr, N=1000 → ~5–6 yr** — i.e. trying
   more configs *demands* a proportionally longer backtest, and below MinBTL a
   Sharpe-1 winner is achievable with no real edge. A short window + a big sweep is
   *guaranteed* to surface a spurious champion. STW [50] is the century-of-DJIA
   empirical proof of the same: even a rule that *survives the data-snooping
   correction in-sample* (1897-1986) was insignificant OOS (1987-1996, p≈0.12).
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
13. **"Faster/fancier search" = MORE multiple testing, never less.** A consistent
    sub-theme across the optimizer/infra papers: every efficiency gain — Bayesian/TPE
    [13], CMA-ME quality-diversity [33], surrogate-assisted MOEA [34][66], 500×-faster
    TensorNEAT [38], warm-start GP [56], CMA-ES [59], grammar-/graph-/GFlowNet-guided
    alpha search [57][58][62], distributional-RL alpha search [65] — lets you evaluate
    *more configurations per unit compute*. On a noisy backtest fitness that means
    reaching the *over-fit optimum faster* and drawing *more* spurious winners ([29]),
    not finding real edge. Quality-diversity's "diversity" is over a *behavior
    descriptor* (turnover, exposure), NOT over robustness — illuminating a behavior
    space faster just industrializes diverse overfitting ([33][62][69]). Corollary: if
    we ever adopt a faster search, the significance correction must scale *with* the
    search budget. The LLM-driven code-evolution wave (FunSearch [18] → AlphaEvolve
    [71] → CodeEvolve [70] → trading: MadEvolve [14], QuantEvolve [69]) is the apex of
    this: it makes "evolve a whole strategy as code" cheap and accessible — which only
    *raises* the importance of a hard cost-aware OOS-vs-B&H gate, because the evaluator
    (a backtest) is the one part that does NOT improve with the search.
14. **Coevolution / Red Queen explains WHY edges decay.** FinEvo [36] and the Red
    Queen's Trap [35] frame markets as adaptive ecosystems where other strategies
    *erode* any edge (Adaptive Markets Hypothesis). This is a deeper cause of "factor
    decay" [6] than overfitting alone: even a *genuine* edge is competed away. Strong
    argument that B&H is the durable benchmark and any crowned active pick is
    temporary — supports periodic re-baking, supports distrust of one-time backtests.
15. **The breakeven-win-rate barrier (a quantitative kill-switch).** The Red Queen
    autopsy [35] gives a clean closed form: for active high-frequency trading,
    breakeven win rate W_BE = (1 + C_ratio)/(1 + R) where C_ratio is round-trip cost /
    target-profit and R is reward-to-risk; at ~0.1% costs and 1% target, W_BE ≈ 55%.
    A strategy whose *implied* win rate can't clear W_BE under our cost model is
    structurally doomed — a cheap pre-screen *before* the bootstrap even runs. This is
    the mechanism behind "cost-blind hallucination": optimizing directional accuracy
    while ignoring magnitude harvests "Fool's Gold" (churning, volume not value).
16. **Honest validation looks like reporting the null.** Multiple careful papers now
    *deliberately* report insignificant aggregate results as the contribution:
    Interpretable Hypothesis-Driven [39] (RL, 100 equities, 34 OOS periods, costs →
    Sharpe 0.33, **p=0.34, not significant**) and the BTC ML-under-costs study [32]
    (**Holm-corrected, does not reject vs B&H**). This is exactly our project's
    posture — the deliverable is the disciplined protocol + the truthful verdict, not
    a flashy curve.
17. **The data-snooping trilogy (the topic's intellectual spine).** Brock-Lakonishok-
    LeBaron 1992 [52] showed simple MA/range-break rules beat random-walk/GARCH nulls
    on the Dow (1897-1986) — but with **no costs and a tiny pre-chosen rule set**.
    Sullivan-Timmermann-White 1999 [50] applied **White's Reality Check** to the
    ~7,846-rule *universe* (filter / MA / support-resistance / channel-breakout /
    on-balance-volume families). **First-hand nuance now captured:** STW found certain
    rules **DID survive the data-snooping correction *in-sample* 1897-1986** — but the
    best rule was **insignificant out-of-sample 1987-1996 (Reality-Check p≈0.12)** and
    earned **nothing on S&P 500 index futures** (the one market where costs + shorting
    are clean). So the lesson is two-fold: (a) **significance must be conditioned on
    the size of the search** ("charge the search budget" [29]), AND (b) **even an
    in-sample-significant, snooping-corrected rule can be worthless OOS** — the verdict
    must rest on held-out / resampled / cost-net performance, never the in-sample fit.
    This is the century-of-data empirical proof of the Bailey [29] IS→OOS-degradation
    theorem, and *why our gate keeps both the search-size correction (DSR/PBO/Reality-
    Check) AND the regime bootstrap* — neither alone suffices.
18. **MARKET-MATURITY NUANCE (the honest counter-weight).** Hsu-Kuan [51]:
    Reality-Check-corrected, cost-net, OOS — technical rules **survive in YOUNG/less-
    efficient markets (NASDAQ, Russell 2000) but NOT in mature ones (DJIA, S&P 500).**
    Crypto is plausibly young/less-efficient, so we should NOT be dogmatic that the
    null always holds — the disciplined posture is **"test honestly, don't assume."**
    BUT: it's cross-sectional indices not single coins; the rules that add the most are
    compound (bigger search → only safe *because* corrected); efficiency rises over
    time. So this *raises the value of our gate*, not a license to trust active edges.

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
- **Holds up (a portable anti-overfitting *search* technique — confirmed by two
  first-hand reads):** **difficulty-weighted training-subset rotation.** [92] (GP for
  IV) rotates the training subset every g generations and *up-weights the subsets the
  search currently fits worst* (Adaptive-Random "ARSS" wins on OOS MSE); [10]
  (vectorial GP for trading) independently uses the same idea (random-buffer sampling
  + segment the training set into 3 parts, evaluate on one per generation, all three
  during "super generations"). The principle — never let the optimizer score a
  candidate on one fixed slice; rotate across sub-windows and concentrate scrutiny on
  the *hardest* ones — is a direct cousin of our **weakest-link** moving-block
  bootstrap. Concrete import: pre-regularize any bake-off optimizer by rotating
  candidates across our regime blocks and weighting toward the blocks where each does
  worst. Caveat from [10]: even *with* this regularization, evolved strategies showed
  **frequently negative OOS fitness despite positive training fitness** — it abates,
  doesn't cure ([21]).
- **Holds up (as a risk idea, not alpha):** "B&H + strategy" portfolios reduce
  drawdown even when the strategy alone doesn't beat B&H ([5]); dynamic re-weighting
  to combat **factor decay** ([6]) flags that edges are non-stationary.
- **Holds up (new overfitting diagnostics worth importing):** (a) **PBO / CSCV**
  (Probability of Backtest Overfitting via Combinatorially-Symmetric Cross-Validation)
  — AutoQuant [53] applies it to *crypto perps* and finds **substantial residual
  overfitting even after careful tuning**, framing its system as "validation
  infrastructure, not proof of persistent alpha" (the closest external mirror of our
  mission); [23][29] motivate the same. (b) **White's Reality Check** [50] — a
  bootstrap that tests the best strategy against the *whole searched universe*. (c)
  **Synthetic / resampled-path testing** [55] (GAN-generated paths) — same principle
  as our moving-block bootstrap; we prefer the *model-free* bootstrap because a GAN
  can hallucinate dynamics or miss tails. (d) **Parameter-plateau check** [49] —
  prefer configs sitting on a *broad* performance plateau over sharp optima (a cheap
  fragility screen). (e) **Multi-cost-scenario double screening** [53] — evaluate a
  candidate under several cost assumptions, not one; fee-only crypto-perp backtests
  *materially over-state* returns vs fully-costed (funding+slippage).
- **Boundary case (a well-controlled crypto "beats B&H" that ISN'T our case):**
  AdaptiveTrend [49] beats BTC B&H (Sharpe 2.41 vs 0.17, −12.7% vs −64.1% DD) WITH
  costs + circular-block bootstrap + significance — but via a **150-asset cross-
  sectional long-short + funding carry**, none of which single-coin long-only retail
  can do. Confirms: edge is reachable through *diversification/relative-value/carry*,
  not single-coin timing; for our scope it still says "hold."
- **Boundary cases (honest "beats B&H net of costs" that ISN'T our case):** eTrend
  [89] (XCS-evolved trend rules beat B&H with high *Sortino* after costs) — but on a
  *risk-adjusted* metric (likely downside protection, [77] says EC rules win in
  downtrends/lose in uptrends), cross-sectional stocks, no regime-bootstrap; XCS-MSCI
  [88] beats B&H+random but via *index-reconstitution flows*. Both are interpretable +
  cost-aware, neither clears *our* single-coin total-equity-vs-B&H + regime-bootstrap bar.
- **Holds up (legitimate evolution success OUTSIDE our scope):** GP fitting an
  *implied-volatility surface* beats Black-Scholes on hedging error ([91]) — because the
  target has near-ground-truth (option prices), unlike a noisy backtest. Sharpens the
  rule: evolution is trustworthy when *fitting a model to a well-defined target*,
  untrustworthy when *mining rules against a gameable backtest*. (We are spot single-coin
  → no IV surface to fit.)
- **Does NOT hold up:** headline PnL from cost-blind, single-window, no-benchmark,
  re-optimized-on-the-same-stream optimizers ([4], +550% scalping; [96], 320%/yr on
  5-min FX; [44], Sharpe-2.8 from ~2000 randomized alphas). The "agent / LLM" wrapper
  around a GA changes nothing statistically. Risk-*seeking* objectives that chase
  best-case tails ([12]) overfit by construction (degrade past α>0.85). The cleanest
  *self-reported* failure: GA FX systems superb in-sample, **unprofitable OOS net of
  costs** ([86][95]) — the authors themselves conclude "markets could be efficient."
- **Recurrent blind spots even in "good" papers:** missing **buy-and-hold**
  benchmark ([2][8][11][12][14]) and missing/implicit **transaction costs**
  ([2][8][11][12]); IC/accuracy reported instead of cost-net PnL. Even MadEvolve
  [14] — the most self-aware — measures gains vs its *own pre-evolution baseline*,
  not vs holding the coin.
- **Resolved across 100 papers:** *no* evolutionary search in the reviewed literature
  clears a cost-aware, regime-spanning, B&H-benchmarked, multiple-testing-corrected gate
  on a **single liquid coin, long-only, unleveraged**. Every apparent counterexample
  relies on a structural lever outside our scope (HFT speed [93][94], cross-sectional/
  long-short breadth [49][63][88], carry [49], leverage [83], index-flow events [88]) or
  drops a control (costs/B&H/OOS/correction). The careful papers converge on our thesis.

## Actionable takeaways for our advisor

1. **Treat our bake-off as a search that must be significance-charged — DSR + MinBTL
   are now the exact closed forms to enforce it.** Sweeping many strategies/params on
   `(coin, window)` IS a multiple-testing exercise. **The Deflated Sharpe Ratio [98]
   is the formula we add to the gate, captured first-hand:**
   **DSR = Z[ (ŜR − SR₀)·√(T−1) / √(1 − γ̂₃·ŜR + ((γ̂₄−1)/4)·ŜR²) ]**, where the
   rejection threshold is the *expected maximum* Sharpe under the null (Eq.1):
   **SR₀ = √V[{ŜRₙ}] · ( (1−γ)·Z⁻¹[1−1/N] + γ·Z⁻¹[1−(1/N)·e⁻¹] )**, γ≈0.5772.
   Direct implementation mapping for our pipeline (all five inputs are things we
   already have): **N ← bake-off config count**; **V[{ŜRₙ}] ← variance of the Sharpe
   ratios across all baked-off configs**; **T ← window length in periods**; **γ̂₃, γ̂₄
   ← realized skew/kurtosis of the crowned strategy's returns**. Crown only if
   **DSR ≥ 0.95**. **The worked example is the load-bearing crypto lesson:** a ŜR=2.5
   over 5 years with N=100, skew−3, kurt10 → DSR≈0.90 < 0.95 (reject); with **Normal
   returns the same ŜR clears at N=88 trials, but skew−3/kurt10 drops the survivable
   trial count to 46** — so **fat-tailed crypto returns *shrink* the number of configs
   a strategy can survive**, and our gate should be *more* suspicious of large sweeps
   on heavy-tailed coins, the opposite of the naive intuition. Pair DSR with: (a) a
   **MinBTL pre-flight check** [29] — assert window length T ≥ MinBTL(N) *before*
   crowning (a one-line guard; the years-vs-trials table above is the lookup); (b) a
   **PBO / CSCV** number [53] (AutoQuant applies it to crypto perps and still finds
   substantial residual overfitting after careful tuning, concluding "validation
   infrastructure, not proof of persistent alpha" — our mission re-derived); (c)
   **White's Reality Check** [50] / **Holm** [32] over the N tried (Holm flipped a
   >65%-return BTC strategy to "not significant vs B&H"). The three corrections are
   complementary: DSR = closed-form parametric Sharpe deflation, PBO = non-parametric
   CSCV, Reality Check = bootstrap-over-the-universe. Also adopt the **optimal-stopping
   discipline** [98] to *bound* the sweep up front — sample ~1/e (37%) of justified
   configs, then take the first that beats them — because every extra trial
   irreversibly raises SR₀.
2. **Add a RANDOM-strategy null as a gate sub-test (highest-value new idea) — and
   construct it correctly.** Beyond "beat B&H," require a tuned/evolved pick to beat a
   random-trading null ([5][30][31]). The first-hand read of Chen-Navet [31] pins down
   the *fair* construction: the random null must be **matched-activity — same trade
   frequency AND same time-in-market (intensity)** as the candidate — otherwise the
   comparison is contaminated by differing transaction-cost exposure (a strategy
   could "win" merely by trading less). And the *search-vs-search* comparison must be
   **equal-intensity**: pit our bake-off (N configs) against a random search that
   draws ~N random configs, the same "charge the search budget" logic as DSR/MinBTL.
   The clean diagnostic [31]: if random search beats a lottery-null but our optimized
   pick does *not*, the optimizer is **overfitting**, not finding edge. An optimized
   config that beats <~50% of matched random sets is overfit, not skilled — cheap, and
   catches edges that look good vs B&H purely from lucky timing.
3. **Double-out-of-sample is the gold standard** ([5]): if we add param tuning,
   reserve a final unseen window evaluated exactly once; beware "OOS" data reuse.
4. **A cost-aware execution filter is the one thing that helped** ([32]): only trade
   when the expected move exceeds a cost-tied threshold. It cut turnover enough to
   make an ML strategy gross-viable (though still not B&H-significant) — worth
   testing as a turnover-reducing strategy modifier, not as an alpha claim.
4b. **The trinary "no-trade" signal + CVaR-based fitness are honest turnover/risk knobs**
   ([99]): an evolved rule with an explicit *no-trade* action ([99] trinary buy/sell/hold)
   abstains when the signal is weak — a cousin of the cost-aware execution filter [32] and
   the breakeven-win-rate screen [35] — and a **coherent CVaR / conditional-Sharpe fitness**
   ([85][99]) is more honest than mean-variance for fat-tailed crypto. Worth testing as
   turnover-control + tail-aware ranking, not as an alpha claim.
5. **Factor/strategy decay is real** ([6]): consider a periodic re-bake cadence for
   a crowned pick; combine with the [13] finding that the best optimizer/strategy is
   per-(coin,window), so re-baking should re-select, not just re-fit. Make the re-bake
   **event-driven, not calendar-driven** — the "**update when required**" principle [97]:
   trigger a re-bake only when a monitored signal (realized-vs-expected divergence,
   regime-change flag, edge-decay statistic) crosses a threshold. Calendar re-baking
   churns costs *and* draws fresh overfit winners each cycle ([29]); coevolution [35][36]
   says edges decay because *competitors erode them*, so a decay/divergence trigger
   captures the right moment to re-evaluate (or to conclude "hold instead").
6. **Don't import the cross-sectional alpha-factor paradigm** ([3]): single-coin
   retail can't diversify across weak short-horizon alphas; it just pays turnover.
7. **If we ever run a search, copy MadEvolve's honesty kit** ([14]): cost-in-fitness,
   scale-invariant metrics, and compare IS→OOS degradation against multiple-testing
   theory — but keep **buy-and-hold** as the benchmark it omitted.
8. **Add a breakeven-win-rate pre-screen** ([35]): before bootstrapping a tuned/active
   pick, reject any whose implied win rate can't clear W_BE = (1+cost/target)/(1+R)
   at our cost model. Cheap, closed-form, kills high-turnover "Fool's Gold" candidates
   before they consume the gate's budget. Complements (does not replace) the B&H
   benchmark and the random-null sub-test.
9. **Scale the significance correction WITH the search budget** ([29][33][34][38]): a
   faster/bigger bake-off (more strategies, finer grids, Bayesian/surrogate search) is
   *more* multiple testing — the deflated-Sharpe/PBO bar must tighten as the config
   count grows, or efficiency gains just buy faster overfitting.
10. **Quality-diversity is a presentation tool, not an overfitting cure** ([33][62][69]):
    an archive of behaviorally-diverse strategies (by turnover/exposure/risk-profile)
    could be a nice way to *show a user a menu* — QuantEvolve [69] realizes exactly this
    MAP-Elites-feature-map-of-investor-preferences idea for trading — but every archived
    elite must individually clear the FROZEN gate; "diverse" ≠ "robust."
11. **Bake the overfitting penalty INTO the fitness/objective, not just the post-hoc
    test, and report a generalization ratio** ([68]): GT-Score [68] = **(μ · ln(z) ·
    r²) / σ_d** (μ=mean return, z=excess-return-over-B&H significance Z-score, r²=path
    consistency, σ_d=downside deviation), implemented piecewise to *penalize* configs
    that don't clear B&H beyond sampling noise. **Correction from the full read:** it
    *does* use a buy-and-hold benchmark (μ_m, inside the Z-score) and *does* include a
    cost-sensitivity check (0–10 bps/side) — my earlier "omits both" was wrong. Real
    numbers: walk-forward generalization ratio **0.365 (GT-Score) vs 0.185 baseline =
    the "98%"**, but GT-Score's *raw* OOS return is **lower** (43.6% vs 46–50%) — it
    explicitly trades return for retention, and even the best objective retains only
    **~37% of training return OOS** (a sobering anchor for how illusory in-sample
    performance is). Three portable imports: (a) a **B&H-relative significance gate
    baked into ranking** (penalize sub-B&H configs — close to our weakest-link
    verdict); (b) the **generalization ratio (validation÷training return) as a
    per-candidate overfitting metric** — any bake-off pick far below 1 is overfit; (c)
    Monte-Carlo-over-seeds stability ([43]). Decisive caveat: effect sizes are small
    (Cohen's d<0.2) and the **parametric Z-score breaks under fat tails** — so for
    crypto the *non-parametric* version of the same idea (our bootstrap + DSR [98]) is
    the honest implementation; a better objective *abates* overfitting ([21]), never
    cures it.
12. **Prefer parsimonious (fewer-indicator) crowned picks** ([74]): MOEA/D selected
    *fewer* indicators with better interpretability than AGE-MOEA; simpler strategies
    overfit less ([21][29]) and are easier to narrate to a retail user. A complexity
    penalty (the deflated-Sharpe spirit) should bias our bake-off toward the simplest
    config that clears the gate.

## Open questions / things worth testing in our app

- **Random-null sub-test:** add "beat matched-activity random trading" alongside
  "beat B&H" in the gate. Does any crowned pick survive both? ([5][30][31])
- **Cost-aware execution filter** as a strategy modifier: does threshold-gated
  trading reduce turnover enough to change any verdict? ([32])
- Is there a re-bake cadence that genuinely helps vs. just churns costs? ([6][13])
- Does adding evolutionary search to our bake-off ever produce a config that clears
  the FROZEN gate — or only ever in-sample winners? (Hypothesis: only in-sample;
  [29] predicts the winner is *negatively* predictive OOS.)
- **Breakeven-win-rate pre-screen** ([35]): would adding W_BE=(1+cost/target)/(1+R)
  as a cheap reject-before-bootstrap filter prune obviously-doomed high-turnover
  candidates without discarding any pick the full gate would have crowned?
- **Re-bake cadence under coevolution** ([35][36]): if edges decay because *other
  strategies compete them away* (not just overfitting), is there a re-evaluation
  cadence that captures decay early — or does the decay just confirm "hold instead"?
- **Quality-diversity menu** ([33]): if we returned an archive of behaviorally-diverse
  strategies (binned by turnover/exposure) instead of a single crowned pick, would
  *any* cell contain a strategy that beats B&H net of costs under the gate?

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
- "Optimization solver" ≠ "trading edge" (Markowitz frontier solvers) → [16][17][46][47]
- Industrialized overfitting (generate ~2000 alphas, flashy Sharpe, no costs/B&H/correction) → [44]
- IS→WFA→OOS cascade as the overfitting-mitigation standard → [5][45]
- AutoML/NAS on financial data: seed-variance dominates, no convergence (AUC ~0.55) → [43]
- "IC alone insufficient — also need stability/robustness/turnover" (meta-evaluation) → [42]
- Honest-null reporting as the contribution (insignificant aggregate result) → [32][39]
- Optimizer choice interacts with (strategy, asset); per-pair tuning → [13]
- Evolutionary program search needs a sound evaluator (FunSearch) → [18]
- Vectorial / strongly-typed GP as better rule representation → [10]
- GP overfits by nature; model selection only abates it → [21]
- "Risk-seeking"/best-case objectives overfit on noisy data → [12][24]
- Forecast-error/AUROC win ≠ cost-net edge vs B&H → [22][25]
- Overfitting-probability test as a candidate filter (PBO-like) → [14][23]
- "GP/ML beats B&H on BTC" claims that omit costs/regimes → [19][20][40]
- Faster/fancier search = more multiple testing, not more edge → [13][29][33][34][38]
- Quality-diversity (CMA-ME) industrializes *diverse* overfitting; diverse ≠ robust → [33]
- Surrogate-assisted search speeds the route to over-fit Pareto corners → [34]
- Coevolution / Red Queen / AMH: other strategies erode any edge → [35][36]
- Breakeven-win-rate barrier W_BE=(1+cost/target)/(1+R) as a kill-switch → [35]
- Cost-blind hallucination / churning ("Fool's Gold," optimize accuracy not magnitude) → [35]
- Maximalist DL+evolution crypto system: +300% validation → −70% live (IS→OOS collapse) → [35]
- Honest validation = deliberately reporting an insignificant null → [32][39]
- GA/MOEA delivers risk reduction (lower drawdown), not alpha vs B&H → [5][7][27][28][37]
- Data-snooping trilogy: rules beat nulls → corrected for universe → mostly artifact → [52]→[50]→[51]
- White's Reality Check: significance must condition on the searched universe size → [50]
- MARKET-MATURITY nuance: edge survives (corrected, cost-net, OOS) in YOUNG markets, not mature → [51]
- PBO / CSCV finds substantial residual overfitting even after careful tuning (crypto perps) → [53]
- AutoQuant ≈ our mission: auditable cost-aware validation infra, "not proof of persistent alpha" → [53]
- Fee-only crypto-perp backtests materially over-state returns vs funding+slippage costed → [53]
- Synthetic/resampled-path testing as overfitting defense (we prefer model-free bootstrap) → [55]
- Parameter-plateau (broad optimum) preferred over sharp optima as a fragility screen → [49]
- Well-controlled crypto BEATS B&H — but via cross-sectional long-short + carry, not single-coin → [49]
- Foundational GP-FX: no excess returns net of costs → market efficiency → [48]
- Quality-diversity (CMA-ME) as a strategy-menu presentation tool, not an overfitting cure → [33]
- Coevolution as the *cause* of decay (other strategies compete edge away) → [35][36]
- Code-evolution wave: FunSearch → AlphaEvolve → CodeEvolve → trading (MadEvolve/QuantEvolve) → [18][71][70][14][69]
- Anti-overfitting penalty baked INTO the fitness/objective; generalization-ratio diagnostic → [68]
- GFlowNets / grammar / graph-guided alpha search = fancier search, same gate gaps (no costs/B&H) → [57][58][62]
- Distributional-RL / quantile alpha mining (model the return distribution to drive search) → [65]
- Quality-diversity MAP-Elites of investor-preference behaviors as a strategy MENU → [69]
- Parsimony: fewer-indicator strategies overfit less + more interpretable (MOEA/D vs AGE-MOEA) → [74]
- Neuroevolution "beats index" only via long-short / cross-sectional breadth (not single-coin) → [63]
- GA classification accuracy on emerging tokens ≠ cost-net edge (illiquid = huge effective costs) → [73]
- EA improves a portfolio by risk-shaping via ALLOCATION (moot for single coin) → [67][75]
- Better-disciplined GA (B&H + walk-forward + drawdown-fitness) still lands on risk-reduction, omits costs → [72]
- Deep neuroevolution / CMA-ES: capability to evolve big policies exists; the evaluator decides edge → [59][64]
- Surveys (EC+RL in finance; evolution+deepRL policy search; EC rule-discovery review; Darwinian-in-finance) — method-rich, validation-poor on B&H → [76][77][79][84]
- Genetic Network Programming (graph individuals) + CVaR: regime-conditional RISK-shaping (max-Sharpe in bull, min-risk in bear), not single-coin alpha → [85]
- Strongly-typed / gene-expression GP: better/parsimonious rule representations; "beats classical optimization" ≠ "beats B&H net of costs" → [82][83]
- Evolutionary feature/alpha construction optimizes feature quality (diversity/information), not cost-net PnL → [81]
- Learning Classifier Systems (XCS/eTrend): interpretable evolved rules; best honest "beats B&H net of costs" cases but Sortino-based / cross-sectional → [88][89][90]
- GA-tuned params beat DEFAULT params, but realistic spreads+commissions ⇒ "markets could be efficient" → [86]
- GA-for-LSTM-hyperparameters: price-level R²=0.87 is trivial-persistence, not edge → [87]
- eTrend/eTrendRev: same evolutionary machinery evolves momentum OR reversion ⇒ it fits the regime it's shown → [89][90]
- XCS "beats B&H+random" but via MSCI index-reconstitution flow (event-driven/cross-sectional, not single-coin) → [88]
- DEFLATED SHARPE RATIO closed form (DSR=Z[(ŜR−SR₀)√(T−1)/√(1−γ̂₃ŜR+((γ̂₄−1)/4)ŜR²)], threshold SR₀=expected-max-SR via EVT) = our DSR gate addition; inputs N/V/T/skew/kurt all already in our bake-off → [98]
- DSR worked example: fat tails SHRINK the survivable trial budget (Normal 88 trials vs skew−3/kurt10 only 46) ⇒ be MORE suspicious of big sweeps on heavy-tailed crypto → [98]
- MINIMUM BACKTEST LENGTH closed form (MinBTL≈[(1−γ)Z⁻¹(1−1/N)+γZ⁻¹(1−1/(Ne))]²/E[maxSR]²; table N=10→0.5y,100→2.5–3y,1000→5–6y) = one-line pre-flight gate check → [29]
- Matched-activity random null (same trade frequency AND time-in-market) + equal-search-intensity = the cost-fair way to build the random-null sub-test → [31]
- STW: rule SURVIVES snooping-correction in-sample yet FAILS OOS (DJIA p≈0.12) + earns nothing on S&P futures ⇒ need both search-size correction AND regime bootstrap → [50]
- Difficulty-weighted training-subset rotation (up-weight the hardest sub-windows during search) = portable anti-overfitting technique, cousin of weakest-link bootstrap → [92][10]
- Vectorial/typed GP with serious regularization STILL shows negative OOS fitness despite positive training fitness ⇒ representation/regularization abates not cures → [10]
- GT-Score=(μ·ln(z)·r²)/σ_d with B&H-relative significance z baked into the objective + cost-sensitivity check; generalization ratio as per-candidate overfitting metric → [68]
- GA trading system: superb in-sample, unprofitable OOS once costs imposed (canonical) → [95]
- GA-tuned params beat defaults, but realistic spreads ⇒ "markets could be efficient" → [86]
- Trinary buy/sell/NO-TRADE rule + CVaR/conditional-Sharpe fitness = honest turnover/tail knobs → [85][99]
- "Update when required" (event-driven, not calendar-driven re-optimization) for re-bake cadence → [97]
- Adaptive Markets Hypothesis via STGP: edges transient + speed-dependent (HFT) → decay + keep-testing → [93][94]
- "Evolution beats the market" only via HFT front-running / order-flow (latency edge, not retail timing) → [93]
- GP-for-options/volatility (IV surface fit) = legitimate evolution success WITH near-ground-truth target, out of our spot scope → [91]
- Dynamic/rotating training-subset selection during search = portable anti-overfitting technique → [92]
- Anti-pattern: GA 320%/yr on 5-min FX, no costs/B&H/OOS → industrialized overfitting → [96]
- Capstone note: even the freshest 2025 GP (STGP-SATA) benchmarks vs ML models, not cost-net B&H → [100]
- META-FINDING: EC-evolved rules work in DOWNTRENDS, poorly in UPTRENDS ⇒ B&H wins over a full cycle → [77]
- Swarm/ACO trading = in-sample optimal-sequence fitting (hindsight artifact), not OOS edge → [80]
- Directional-changes (event/intrinsic-time) representation + multi-threshold as a candidate feature family → [28][78]
