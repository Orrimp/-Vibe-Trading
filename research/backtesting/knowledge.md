# Knowledge — Backtesting & Test Data

_Synthesis of the `backtesting` ledger (100 papers; round-3 added [78–100]).
Payoff focus: concrete ways to harden our FROZEN robustness gate (1000-path
moving-block bootstrap vs buy-and-hold), our ranking, and our test-data pipeline —
and to avoid overfitting. Numbers in [brackets] reference papers.md entries._

> **Round-3 addition — the DSR/PBO gate add is now FULLY SPECIFIABLE; plus the
> bootstrap's provenance, calibration counter-weights, and forecast-test rigor
> [78–100].** This round closes the loop on the gate upgrade — see the dedicated
> **"DSR/PBO gate add — concrete spec"** section below. Five clusters:
> (1) **Bootstrap provenance & internals.** Künsch [78] is the literal parent of
> our moving-block bootstrap (consistency needs block length ℓ→∞, ℓ/n→0 — so a
> FIXED length is a heuristic; use the data-driven Politis–White selector [20]). The
> CIRCULAR block bootstrap [89] fixes our fixed-length MBB's edge bias (first/last
> bars under-sampled → understated tail risk) via wrap-around equal weighting — a
> cheap, self-contained gate fix. Bergmeir–Benítez [79] gives the formal license for
> multi-fold/CV evaluation over single hold-out, with the diagnostic "k-fold is valid
> iff residuals are white" (crypto residuals aren't → we MUST block + purge).
> (2) **Effective-N is solved.** López de Prado–Lewis [86] (False Strategy Theorem +
> ONC clustering) is the estimator for N: cluster the candidate return-correlation
> matrix, count clusters = effective independent trials. Novy-Marx [80] is the
> counter-pressure: COMPOSED multi-signal strategies inflate N toward (single)^k.
> Harvey–Liu "Evaluating Trading Strategies" [87] is the practitioner blueprint
> (haircut Sharpe via Bonferroni/Holm/BHY, real-time). Benjamini–Hochberg [85] is the
> FDR core (use BHY dependence variant for our correlated configs).
> (3) **Calibration counter-weights — don't over-deflate into nihilism.** Chen
> [84] (p-hacking can't manufacture t≫4) and Chen–Zimmermann [81] (publication bias
> explains only ~10–15%; FDR<10%) prove a high-deflated-t region survives → the gate
> should report the DEFLATED statistic and leave room for the rare real winner, not a
> blanket "assume noise." Kosowski–Timmermann–Wermers–White [100] is the capstone
> bootstrap-skill-vs-luck method (a small right-tail minority is genuinely skilled).
> Hsu–Hsu–Kuan [99] and Sermpinis et al. [88] find TA edges CAN survive snooping
> correction in LESS-efficient markets (crypto is arguably one) → the gate must be
> POWERFUL (SPA studentization), not just strict.
> (4) **Forecast-comparison rigor for the forward leg.** Our crown-vs-hold is a
> NESTED comparison → Clark–West [96] (active = hold + maybe-zero signal; naive OOS
> test biased AGAINST active by the noise of estimating extra params). Giacomini–
> White [94]: the honest question is CONDITIONAL ("which does better GOING FORWARD
> given state?"), misspecification-robust. Inoue–Kilian [97]: OOS tests have LOWER
> power than IS and are NOT automatically snoop-proof → a forward "loss" isn't proof
> of no-edge (low power) and the bootstrap recovers the power a single hold-out throws
> away. Hansen–Timmermann [98]: the WINDOW/split choice is itself data-mining → vary
> it, pre-commit it. Gelman–Loken [90]: even ONE analysis carries hidden multiplicity
> via data-contingent choices → pre-register the sweep grid (our FROZEN-gate ethos).
> Varma–Simon [91]: select and score on the SAME data = biased → nest (our in-sample
> crown vs untouched forward leg is the right architecture).
> (5) **Cost realism & decay, on-asset.** Frazzini–Israel–Moskowitz [82] and
> Patton–Weller [83]: costs are STRONGLY turnover-dependent and paper returns
> overstate realized (value low / momentum ~zero net) → report turnover; high-turnover
> crowns get extra skepticism; structurally favors hold. Donier–Bonart [95]: the
> √-impact law holds on BITCOIN specifically → fixed fee+spread is the right retail
> limit, impact only bites at size. Hansen–Lunde "does anything beat GARCH(1,1)?"
> [93]: the canonical proof RC LACKS POWER and SPA is the upgrade — strongest single
> citation for studentizing our gate; and a "nothing beats the simple baseline"
> analogue of our thesis.

> **Round-2 addition — tail-risk forecast validation (VaR/ES backtesting) [48–53].**
> A whole sub-literature on validating a DOWNSIDE forecast that we had not yet
> catalogued. Our bootstrap already emits a full loss distribution per strategy and
> for buy-and-hold, so these are cheap to bolt on as a forward-run calibration panel:
> **Kupiec POF** (right breach RATE, χ²₁) [48]; **Christoffersen LR_cc** (right rate
> AND breaches don't CLUSTER — vital for crypto vol-clustering) [49]; **duration /
> runs**-based tests for more power in small samples [54]; **Acerbi–Székely** ES
> tests (tail SEVERITY, not just frequency; ES IS backtestable despite non-
> elicitability) [50]; **Berkowitz PIT + censored tail test** (validate the whole
> predictive density / its tail with high-power Gaussian LR) [51]; **multinomial /
> multi-level VaR** test (implicitly backtests ES; N≥4 levels far more powerful at
> catching heavy-tail underestimation) [52]; and a **traffic-light green/amber/red**
> reporting pattern for the verdict [53]. Caveat carried throughout: all VaR/ES
> backtests are LOW-POWER in short samples — a "tail looks fine" pass on a short
> forward window is weak evidence, the MinBTL/MinTRL power discipline [3][4][32]
> restated for tail risk.

> **Round-2 addition — live-vs-backtest DECAY is quantified and predictable
> [57][58].** McLean–Pontiff [57]: published anomaly returns fall ~26% out-of-
> sample (data-mining illusion, upper bound) and ~58% post-publication (the extra
> ~32% = arbitrage as investors read the paper and trade it away). Falck–Rej–
> Thesmar [58]: across factors, OVERFITTING proxies (signal COMPLEXITY, OUTLIER-
> sensitivity) predict decay better than arbitrage-capital proxies — most decay is
> "never as real as the backtest said," not "competed away." For us: (a) EXPECT
> the forward paper-trade to underperform the in-sample crown by >=26% as the
> NORMAL case; (b) the textbook indicators we use (SMA/RSI/MACD) have been public
> for decades -> apply a heavy "well-known signal" decay discount; (c) measure and
> PENALIZE complexity + outlier-sensitivity in ranking — they predict which crown
> will rot.

> **Round-2 addition — market-impact MODELS beyond the sqrt-law [55][56].** The cost
> literature now has three layers: static sqrt-law (impact ~ Y*sigma*sqrt(Q/V))
> [13][41]; Almgren–Chriss permanent+temporary linear schedule [27]; and
> TRANSIENT-impact models — Obizhaeva–Wang LOB resilience [55] and the Bouchaud
> propagator (decay kernel reconciling persistent order flow with diffusive prices)
> [56]. All agree impact is DYNAMIC and DECAYING, and all vanish at EUR 200 retail
> scale (participation ~ 0). Three INDEPENDENT confirmations [13][41][55] that a
> fixed fee+spread is a defensible small-order limit. The only at-size import: if
> capacity ever enters scope, charge a decay KERNEL (transient), not a static
> per-trade or permanent impact — the former under-, the latter over-states cost.

> **Round-2 addition — point-in-time / data-revision METHODOLOGY [59][60]** (we own
> the evaluation discipline; the `data` topic owns the crypto data plumbing).
> Croushore–Stark [59]: evaluating on FINAL/revised data overstates real-time skill
> — out-of-sample comparisons must use the VINTAGE actually available at each date.
> Practitioner checklist [60]: lag fundamentals to ANNOUNCEMENT date, retain
> original-value + restated-value-with-date, use POINT-IN-TIME universe membership.
> For our spot-price advisor the reporting-lag trap is dormant, but the as-of-
> timestamp rule should be a TESTED INVARIANT of the data layer so future features
> (on-chain, funding, sentiment) inherit "what was knowable at bar t" for free;
> plus PIT listing/delisting gating ties to survivorship [14].

> **Round-2 addition — "the SET of best models," and selection-bias-corrected
> estimation we can actually run [61][62][63][64].** Beyond crowning one winner:
> the MODEL CONFIDENCE SET [61] returns the bootstrap-determined SET of strategies
> statistically indistinguishable from the best — and if BUY-AND-HOLD is in that
> set, the crown does NOT robustly beat holding (our thesis, made into a test);
> uninformative/short data correctly yields a LARGE set (built-in honesty).
> Cawley–Talbot [62] name "over-fitting in model SELECTION" (the bake-off score is
> selection-biased, never report it as the crown's performance) and prescribe
> NESTED CV (inner = sweep, outer = forward/never-used-for-selection). Tsamardinos
> BBC-CV [64] is the cheap win: bootstrap the POOLED per-config OOS returns
> (block-bootstrap to preserve autocorrelation), RE-SELECT the winner in each
> resample, measure its out-of-bag performance vs hold — a selection-bias-CORRECTED
> expected-performance estimate using machinery we ALREADY run (our per-config
> return matrix == its prediction matrix Π). Diebold–Mariano [63] = the arbitrary-
> loss, HAC-robust pairwise "is A better than B?" test (our block bootstrap is its
> resampling analogue), with the sharp caveat that significance on the SAME window
> the crown was selected on is contaminated — it must come from the OUTER/forward
> leg. Net: [64] is a strong candidate to BE our deflation engine; [61] a strong
> candidate for the ranking OUTPUT (report the set, flag if hold is in it).

> **Round-2 addition — timing-SKILL tests, attribution, permutation nulls, and a
> fresh illusory-profitability proof [65][66][67][68][69][70].** A new angle: test
> the directional CALL, not just the equity curve. Henriksson–Merton [65] non-
> parametric 2x2 test — tabulate the crown's in-market/flat signal vs next-bar
> market direction; skill requires P(correct|up)+P(correct|down) > 1; a crown can
> beat hold by luck yet FAIL this (cheap "timing-skill" badge). Brinson–Hood–
> Beebower [66]: active timing/selection add little on average and are often
> negative net of costs (classic prior for our thesis; mind the variance-vs-level
> misquote). Shapley attribution [67]: order-independent, fair decomposition of a
> COMPOSED crown's edge into components (anti-storytelling). Masters permutation
> tests [69]: a SHUFFLE-returns null (destroys time-structure, preserves margins) —
> complements our block bootstrap (which PRESERVES structure); run BOTH to bracket
> the claim; "permutation training" removes selection bias like RC/BBC-CV. BuildAlpha
> [68]: PARAMETER NEIGHBORHOOD-STABILITY (reject lone spikes in the parameter
> surface), beat-the-best-RANDOM second null, noise/shift perturbations, bagging-
> hurts-means-overfit. Kuang–Schröder–Wang [70]: 25,988 TA rules in emerging FX,
> "hundreds to thousands significant," best >30%/yr — almost ALL profit vanishes
> under data-snooping correction = "illusory." The single most vivid validation of
> our gate's skepticism in a market/rule-class close to ours.

> **Round-2 addition — stationarity guard, DL-era evaluation frameworks, online
> FDR, bootstrap-drawdown CIs, and fresh thesis confirmations [71][72][73][74][75]
> [76][77].** Bai–Perron / fluctuation tests [71]: detect+date structural breaks —
> a crypto window straddling a regime shift makes one in-sample crown a fit to a
> MIXTURE; a missing STATIONARITY pre-check for the gate (the bootstrap assumes
> approximate stationarity). AlphaEval [72]: score candidates on FIVE axes
> (predictive power, temporal STABILITY, ROBUSTNESS-to-noise, financial logic,
> DIVERSITY/redundancy), not one metric — composite beats single-metric selection;
> three axes map to our stability [12], perturbation [68], and effective-N [46]
> ideas. Online-FDR / alpha-investing [73]: control the false-"beats-hold" rate
> ACROSS our whole SEQUENCE of re-runs (decaying memory fits non-stationarity), not
> just within one run. Potter/PyBroker bootstrap [74]: a how-to for OUR exact gate —
> report MAX-DRAWDOWN & Calmar DISTRIBUTIONS (observed worst-case is one lucky path;
> 95th-pct can be 2x observed), use the STATIONARY bootstrap, and NEVER one block
> length (test several; if the verdict flips it isn't robust = spec-curve [44][20]).
> Two fresh thesis confirmations: a 2026 large-scale DL benchmark [75] (complex DL
> fails to beat hold/linear baselines risk-adjusted, multiple-seed, OOS) and a 2025
> BTC TA-vs-ML study [77] (3 of 4 active strategies LOSE to hold net of 0.1% fee;
> the lone "winner" was one uncorrected grid-searched config on one bull-year window
> — exactly what our DSR/PBO/bootstrap discounts). Timmermann–Granger [76]: the
> conceptual WHY — discovered edges self-destruct, inducing non-stationarity.

## DSR/PBO gate add — concrete spec (the headline deliverable)

After 100 papers the selection-bias correction we are adding to the gate is now
fully specified. The recipe below is buildable from artifacts our bake-off already
produces (a per-config bar-return / equity matrix). Honesty-first, ship-passive:
the goal is a CALIBRATED haircut that still leaves room to detect a rare genuine
winner — not maximal pessimism. Citations are load-bearing.

**Which estimator — Deflated Sharpe Ratio (DSR), the López de Prado / Bailey form.**
- Compute, for the crowned config, the **Probabilistic Sharpe Ratio against the
  selection-bias threshold**: `DSR = PSR(SR0) = Φ( (SR_obs − SR0)·√(T−1) /
  √(1 − γ3·SR_obs + ((γ4−1)/4)·SR_obs²) )`, where `SR_obs` is the crown's
  per-bar Sharpe, `T` the number of bars, `γ3,γ4` the skew/kurtosis of the crown's
  returns (crypto fat tails make this term BITE) [1][4].
- The threshold `SR0 = E[max SR]` is the false-strategy benchmark:
  `SR0 ≈ √V_SR · [ (1−γ)·Z⁻¹(1−1/N) + γ·Z⁻¹(1−1/(N·e)) ]`, with `γ≈0.5772`
  (Euler–Mascheroni), `V_SR` the variance of Sharpe across the N trials, and `Z⁻¹`
  the Gaussian inverse-CDF [1][3][86]. Upper bound for sanity: `E[max SR] ≤
  √(2 ln N)·σ_SR` [3].
- **Crown only if `DSR > 0.95`** (true SR exceeds the selection-bias threshold at
  95% confidence) AND the crown's net return clears buy-and-hold. Buy-and-hold is
  EXEMPT (it is the benchmark, not a searched trial). This makes the FRAGILE-can't-
  crown gate also a SELECTION-bias gate.

**How many trials to deflate by — `N` = EFFECTIVE, not raw config count.** This is
the single most important parameter and the one most often gotten wrong.
- Our sweep's configs are MASSIVELY correlated (SMA-50 ≈ SMA-51 produce near-
  identical equity), so the raw config count would WILDLY over-deflate (treat 200
  near-duplicates as 200 independent shots) [86][39].
- **Estimator [86]:** build the N×N correlation matrix of candidate return series
  (we already have the returns), convert to a distance `d = √(½(1−ρ))`, hierarchically
  cluster (ONC or any correlation-distance clustering), and set **`N_eff = number of
  clusters`**. Feed `N_eff` (not raw N) into `E[max SR]`.
- **Counter-pressure [80]:** for COMPOSED multi-signal strategies (our ComposedStrategy-
  from-TOML), the effective search space inflates toward `(single-signal count)^k`
  for a k-component blend — so composed crowns deserve a LARGER `N_eff`. The honest
  number is the cluster count of the FULL realized population (captures redundancy
  ↓ and combinatorial blends ↑ simultaneously). Practical rule: cluster everything
  that was actually evaluated, including every composed variant.
- Expectation for us: `N_eff` will be far smaller than the raw config count (most
  configs are redundant) — so this makes the gate FAIR, not punitive; but composed
  families pull it back up.

**Add PBO (CSCV) as the model-free companion [2].** From the same per-config return
matrix: split into S even time blocks (S=8–16 for our window lengths), form all
C(S, S/2) IS/OOS half-splits **with purge+embargo at the split boundary** [9] (our
indicators have lookback windows → embargo ~1–5% of bars), pick the IS-best config
in each split, record its OOS rank, map to logit λ; **`PBO = fraction of splits with
λ<0` (OOS-below-median)**. Report alongside DSR. Down-rank / refuse to crown when
`PBO > 0.5` [2][21][11].

**Report a haircut RANGE, not one number [87][29][81][84].** Show: (a) the raw
crown Sharpe; (b) the DSR / `E[max SR]`-haircut Sharpe (conservative, FWER-flavored);
(c) optionally a gentler empirical-Bayes / publication-bias shrink [81]. The nonlinear
haircut [29] will gut any crown that only MARGINALLY beats hold (the expected case);
a crown whose DEFLATED t-stat is still large (t≫3–4 after deflating by `N_eff`)
is the rare genuine winner [84] and should be surfaced as such, not hidden behind a
binary pass/fail.

**FWER vs FDR — default FWER for a "nothing beats hold" advisor [85][5][8][87].**
Use a Bonferroni/Holm/StepM (FWER) hurdle as the DEFAULT crown criterion (near-zero
tolerance for a false crown fits our skeptical prior); offer FDR (BHY — the
dependence variant, since our configs are correlated) as a secondary "here is the
SET that would pass at a 10% false-discovery tolerance" view [8][99]. Do NOT use
naive Bonferroni on raw N (over-penalizes correlated configs) — the `N_eff` clustering
[86] or a dependence-aware bootstrap (StepM/SPA) [7][8] is the correct handling.

**Power, not just strictness [93][99][7][8][97].** A White's-Reality-Check-style
gate can be so conservative it misses a real edge — empirically demonstrated in
[93]. STUDENTIZE the bootstrap statistic (divide each config's excess-return-vs-hold
by its bootstrap std) and use a sample-dependent null (SPA [7] / stepwise-SPA [99])
so the many obviously-bad configs we sweep don't bury a genuine winner. Crypto is
arguably inefficient [88][99], so the gate must be able to DETECT an edge if one
exists, while costs [82] and the B&H benchmark remain the deciding filters.

**Bootstrap internals to harden in the same pass [78][89][20][10].** (a) Set block
length from the correlogram via the corrected Politis–White selector [20] (grows ~n^(1/3)
[78]); (b) switch to CIRCULAR (wrap-around) block selection [89] to kill the fixed-
length edge bias (under-sampled boundary bars → understated tail risk); (c) optionally
the random-length STATIONARY bootstrap [10] for boundary-free stationarity; (d) make
block length a SENSITIVITY band — require the "beats hold?" verdict stable across a
small grid [74][20].

**Pre-registration / nesting discipline [90][91][98][62].** The sweep grid, window,
split rule, and cost model must be FIXED BEFORE looking (our FROZEN-gate ethos is
exactly this) — data-contingent choices are hidden multiplicity [90]. Select the crown
and ESTIMATE its performance on DIFFERENT data (in-sample crown vs untouched forward
leg) — never the same [91][62]. Treat the window/split as itself data-mined: vary it
and require stability [98].

**Minimum track-record / backtest length gate [1][3][4].** Refuse to crown / flag
low-confidence when `window_years < ~2·ln(N_eff)/target²` (MinBTL [3]); for the forward
window report MinTRL [4] — the bars needed for the observed Sharpe to be significant
at 95% given `N_eff`. Both are cheap closed-forms from stored returns.

## Key themes

1. **Backtest overfitting is the DEFAULT, not the exception.** Trying even a
   modest number of strategy configs on a fixed dataset produces a spuriously
   high in-sample Sharpe from pure noise [2][3]. Our bake-off sweep over
   SMA/MACD/RSI/Bollinger/composed families IS this setup, so an uncorrected
   crowned Sharpe is upward-biased by construction.
2. **Selection bias has closed-form and resampling corrections.** Expected MAX
   Sharpe over N noise trials ≈ √(2 ln N)·σ_SR; subtract it → Deflated Sharpe
   Ratio [1][3]. Probability of Backtest Overfitting via CSCV needs only the
   per-strategy return matrix [2]. Multiple-testing raises the t-stat hurdle
   from 2.0 to ~3.0–3.8 [5].
3. **Resampling backtests beat single-path walk-forward.** The path-distribution
   family (CPCV [9], bootstrap [6][10], our gate) is empirically the LEAST
   overfit; walk-forward (our forward run's shape) is the WORST at preventing
   false discoveries [11][19][21].
4. **Our gate is academically well-founded — but its internals need rigor.** It
   is a White-Reality-Check-style [6] block-bootstrap [10] test of "best beats
   benchmark," with buy-and-hold as the benchmark. The block LENGTH is a
   first-order, data-dependent parameter we should set from the correlogram, not
   by intuition [20].
5. **Costs and engine mechanics decide phantom alpha.** Transaction costs alone
   can offset ALL apparent technical-rule profit [24][17]; impact grows as
   √(size) [13]; and even the simulation ENGINE's intra-candle/cost choices move
   results materially for high-turnover strategies [22][23].
6. **Overfit strategies can LOSE, not just fail.** Under serial dependence /
   "compensation effects," overfitting yields strictly NEGATIVE out-of-sample
   returns [3] — a likely mechanism behind forward underperformance vs hold.
7. **You can't pick tomorrow's winner from yesterday's.** Persistence tests:
   even with the best data-snooping correction, the ex-ante-selected best rule
   does not stay best [24][14]. This is the core justification for ranking by
   robustness, not in-sample score.
8. **The multiple-testing haircut is NONLINEAR and method-dependent.** Discount a
   reported Sharpe via Bonferroni/Holm/BHY: a marginal Sharpe is gutted, a high
   Sharpe only lightly trimmed [29]. But the SIZE of the correct haircut depends
   on the null model — a worst-case Bonferroni vs an empirical-Bayes /
   publication-bias shrinkage can disagree a lot [29][31], so report a RANGE.
9. **Performance metrics can be GAMED; use a manipulation-proof score.** Sharpe,
   Sortino, alpha etc. are inflatable by selling-insurance / negatively-skewed
   payoffs [28]. The MPPM (power-utility certainty-equivalent growth) is
   gaming-resistant by construction (concave) and directly comparable to
   buy-and-hold [28].
10. **The real question is a DIFFERENCE of Sharpes (strategy vs hold), and it has
   proper tests.** Closed-form PSR/MinTRL for the difference (Opdyke/BLdP,
   correlation-aware) [34] and the Ledoit–Wolf studentized block-bootstrap CI
   [35] both answer "does the crown beat hold's Sharpe significantly?" — and high
   strategy↔hold correlation HELPS detect the difference with less data [34].
11. **Report STATISTICAL POWER, not just a verdict.** "Does not beat hold" can
   mean real-no-edge OR too-little-power; honest work reports the power / minimum
   sample for the observed effect [32], and publishing a rigorous NULL (p=0.34)
   is the correct output, not a failure [32].
12. **The replication debate is genuinely two-sided.** HXZ [26] / HLZ [5] say
   most anomalies fail; Chen–Zimmermann [31] reproduce ~88% and find a small
   (~12%) publication-bias haircut, arguing return DISPERSION is too large to be
   pure data-mining. Stay skeptical but non-dogmatic; technical rules
   specifically still fail net of costs [24].
13. **Both error types matter — strictness is a loss-function choice.** Raising
   the hurdle cuts false discoveries (Type I) but adds MISSED discoveries (Type
   II) [40]. Our gate's deliberate Type-I caution should be DOCUMENTED as an
   explicit loss-function choice, and made efficient (dependence-aware) so the
   strictness buys real protection, not blunt power loss [39][40].
14. **Correlated configs ⇒ use the EFFECTIVE number of tests, not raw N.**
   Bonferroni/Holm assume independence and grossly over-correct correlated
   configs; dependence-aware bootstrap (RC/StepM/SMS) recovers power and is what
   we already own [39][8][6]. Penalize DSR/haircut by m_eff ≪ N [39].
15. **Method/design choices are an uncertainty source bigger than sampling
   noise.** "Non-standard errors" exceed standard errors [44]; results flip on
   weighting/block-length/cost choices [22][26]. Report a SPECIFICATION CURVE —
   the distribution of "beats hold?" verdicts across defensible knob settings —
   not one falsely-precise verdict.
16. **Drawdown has a null too.** E[max drawdown] of a drift-σ Brownian scales
   log/√T/linear by drift sign [43]; judge an observed drawdown against it, just
   as DSR judges Sharpe against E[max Sharpe]. Calmar is ~monotonic in Sharpe, so
   it adds little independent info — prefer MPPM [28][43].
17. **Monte Carlo > resampling > walk-forward (LdP), but a wrong DGP lies.** An
   explicit DGP lets you know WHEN to decommission a model [36]; our
   non-parametric block bootstrap is a defensible middle [36][19]. Prefer
   "tactical" (regime-specific) over "all-weather" claims — but regime-
   conditioning multiplies trials, so DSR/PBO-penalize it [36].

## Methods / findings that hold up (and which don't)

**Hold up (adopt-worthy):**
- **Deflated Sharpe Ratio (DSR)** [1] — closed-form haircut for # trials +
  skew/kurtosis. Cheap; needs N and the winner's moments.
- **PBO via CSCV** [2], operationalized as a SELECTION FILTER in crypto [21] —
  model-free; needs only per-strategy returns; reject/down-rank high-PBO picks.
- **Probabilistic Sharpe Ratio / MinTRL / MinBTL** [1][3][4] — single-strategy
  confidence + required history length, higher-moment-aware.
- **Block / stationary bootstrap** [6][10] with **data-driven block length**
  [20] — our gate's backbone; the random-length stationary variant removes
  block-boundary artifacts.
- **CPCV with purging + embargo** [9] — gold-standard purged path generator;
  φ[N,k]=(k/N)·C(N,k) paths; select by a low percentile (≈ our weakest-link).
- **SPA studentization [7] / Romano-Wolf StepM [8]** — more powerful, less
  conservative than raw RC/Bonferroni; StepM finds the full set of gate-passers
  while controlling FWE and handling config correlation.
- **Lucky-Factors sequential orthogonalize-then-reselect [25]** — bootstrap-based,
  FWER-controlling ranking of CORRELATED candidates; exposes that most "good"
  configs are redundant copies of the crown, not independent discoveries.
- **Sharpe-haircut (Bonferroni/Holm/BHY → discounted Sharpe) [29]** — directly
  implementable from (crowned SR, T, N); nonlinear penalty; ships as R quantstrat
  `SharpeRatio.haircut`.
- **MPPM — manipulation-proof power-utility score [28]** — concave, gaming-proof
  ranking metric; certainty-equivalent growth rate comparable to hold's.
- **PSR/MinTRL & Ledoit–Wolf test for the DIFFERENCE of Sharpes [34][35]** — the
  right test for "crown vs hold"; closed-form (correlation-aware) and
  studentized-block-bootstrap variants; decision = CI excludes zero.
- **FDR mixture model of the whole candidate field [30]** — estimate the
  proportion of configs with genuine positive/zero/negative edge vs hold.
- **Power analysis + honest null reporting [32]** — report detectable effect size
  / power; a rigorous p=0.34 null is a valid result.
- **One-bar-delay robustness check + time-series CV [33]** — cheap look-ahead
  guard; leakage-aware tuning.
- **Effective-number-of-tests (m_eff) via config-correlation [39]** — the right N
  for DSR/haircut when configs are correlated; dependence-aware bootstrap is the
  authority for finance.
- **Specification curve / non-standard-error reporting [44][22][26]** — report the
  distribution of verdicts across defensible evaluation knobs, not one number.
- **Expected-max-drawdown Brownian null [43]** — analytic benchmark for whether a
  drawdown is impressive vs expected.
- **Up-front trial-count CAP + IS-WFA-OOS consistency gate [38][42]** — bound N
  before searching; require minimal IS→OOS degradation.
- **Execution-centric objective (cost in the score, not post-hoc) [42]** — fold
  realistic fees/funding into the ranking metric directly.
- **Seven-sins audit [17] + causal graph before backtest [19]** — cheap
  discipline checklists.
- **(round-3) Effective-N via ONC clustering of the candidate return matrix [86]** —
  the estimator for `N` in DSR/`E[max SR]`; cluster correlated configs, count
  clusters. THE fix that makes DSR usable on our redundant sweep. Pair with the
  composed-strategy `(single)^k` inflation [80].
- **(round-3) Circular (wrap-around) block bootstrap [89]** — removes the fixed-
  length MBB's edge bias (boundary bars under-sampled → understated tail risk);
  one-change hardening of our gate. Parent method = Künsch MBB [78].
- **(round-3) SPA / stepwise-SPA studentization [93][99][7]** — the canonical power
  upgrade; [93] is the empirical proof RC is too conservative. Studentize the
  bootstrap excess-return-vs-hold so bad configs don't bury a real winner.
- **(round-3) Clark–West nested-model OOS adjustment [96]** — our crown-vs-hold is
  nested (active = hold + maybe-zero signal); the CW-adjusted statistic removes the
  estimation-noise bias against the active model when framing a formal OOS test.
- **(round-3) Bootstrap skill-vs-luck of the cross-section [100]** — the classic
  template: bootstrap the candidate field under a zero-edge null, report the crown's
  performance as a PERCENTILE of the luck-only distribution. Non-normality is the
  explicit reason to bootstrap not t-test.
- **(round-3) Vary the window/split + pre-register the grid [98][90]** — the
  split point is itself data-mined; require stability across windows and fix the
  sweep grid before looking (FROZEN-gate ethos).

**Don't / weaker:**
- **Ranking by a gameable ratio (Sharpe/Sortino alone)** [28] — an
  insurance-seller / negatively-skewed strategy can win; pair with MPPM.
- **Eyeballing "strategy SR > hold SR"** [34][35] — needs a difference-of-Sharpes
  test (correlation- and higher-moment-aware), not a raw comparison.
- **A single multiple-testing haircut number** [29][31] — the correct discount is
  null-model-dependent; report a range (worst-case vs empirical-Bayes).
- **A bare "does not beat hold" verdict** [32] — uninterpretable without a power /
  detectable-effect-size statement.
- **Hold-out / single train-test split** [2] — leaks future knowledge, high
  variance, ignores # trials.
- **Single-path walk-forward as the robustness verdict** [11][19][21] — weakest
  scheme; keep ours as a confidence check only.
- **Bonferroni on correlated configs** [5][8] — over-penalizes; neighboring
  configs are highly correlated → use dependence-aware bootstrap (StepM).
- **Raw max-Sharpe selection** [1][3] and **naive √q Sharpe annualization** [16]
  — both upward-biased; never report without correction/CI.
- **Trusting the engine** [22][23] — verify cost model + intra-candle fills;
  prefer conservative assumptions.
- **(round-3) Deflating by RAW config count** [86][80] — over-corrects correlated
  configs; use `N_eff` = cluster count (with `(single)^k` for composed blends).
- **(round-3) Over-deflating into nihilism** [84][81][100] — a high-deflated-t
  region survives even brutal correction; report the deflated statistic and leave
  room for the rare real winner, don't assume everything is noise.
- **(round-3) Treating the forward "loss" as definitive proof of no-edge** [97] —
  OOS tests have LOW power and aren't snoop-proof; the bootstrap recovers the power a
  single hold-out throws away. Forward run = confidence check, not the verdict.
- **(round-3) Naive (unadjusted) OOS test for our crown-vs-hold** [96] — the
  comparison is nested; estimating extra params adds noise that biases the naive test
  AGAINST the active model. Use the CW adjustment if framing a formal OOS test.
- **(round-3) Cherry-picking the backtest window / split** [98] — the split is
  itself data-mining; vary it and pre-commit it.

## Actionable takeaways for our advisor (ranked by leverage)

1. **Surface N and add a Deflated-Sharpe / PBO scorecard to the ranking — now FULLY
   SPECIFIED (see "DSR/PBO gate add — concrete spec" above).** Estimator = DSR =
   PSR(E[max SR]) with the crown's skew/kurtosis [1][4]; threshold `E[max SR]` from
   [1][3][86]; **crown only if DSR>0.95 AND beats hold**. Trial count `N` = EFFECTIVE
   independent trials = number of CLUSTERS of the candidate return-correlation matrix
   (ONC) [86][39], inflated toward `(single)^k` for composed blends [80] — NEVER the
   raw config count (over-deflates correlated configs). Add CSCV-PBO from the same
   return matrix with purge+embargo [2][9]; down-rank `PBO>0.5`. Report a haircut
   RANGE [87][29][81] and the DEFLATED t-stat (don't over-deflate — a high-deflated-t
   crown is the rare real winner [84][100]). Prerequisite: RECORD every config's
   return series, not just the winner's, so `N_eff`/PBO/FDR [30] are computable. →
   Stops us from crowning in-sample noise; highest-value import.
2. **Set the bootstrap block length from the data, not a constant** [20]. Use the
   Politis–White (corrected) selector on each (coin,window)'s correlogram; log
   the chosen length in the report. Too-short blocks make strategies look more
   robust than they are — the dangerous direction. → Makes the FROZEN gate's
   internals principled and reproducible.
3. **Confirm the gate is RC-style (best-vs-benchmark, all configs) and add
   power** [6][7][8]. If it currently tests a single strategy, it under-corrects
   the bake-off's selection bias. Consider SPA studentization and StepM to
   report the SET of strategies that beat buy-and-hold (not just top-1) with FWE
   control.
4. **Run a Seven-Sins self-audit per ranking report** [17], especially:
   look-ahead (shift-by-one-bar test on every indicator [15]), survivorship
   (include dead/delisted coins in any universe claim [14]), costs/turnover
   (audited cost spec; churny crowns get extra skepticism [13][22][24]),
   outliers (drop-top-k-days stress test), storytelling (require a hypothesis
   before crowning [19]).
5. **Report per-strategy uncertainty and forward decay** [16][15]. Attach a
   confidence interval to the crowned Sharpe (Lo SE, non-normality-adjusted),
   prefer native-frequency Sharpe over spun annualization, and report
   OOS/IS Sharpe (Walk-Forward Efficiency [12]) + alpha decay on the forward run.
6. **Cross-validate the engine** [22][23]: a second reference engine must match
   to the penny at ZERO cost and within a documented tolerance with costs; add
   "model candle" tests pinning a conservative intra-candle fill order.
7. **Enforce a MinBTL gate** [3]: flag low-confidence / refuse to crown when
   window_years < ~2·ln(N)/target².
8. **Add a Monte-Carlo / synthetic-ground-truth check** [11][19]: validate the
   gate against synthetic no-alpha series (must return "nothing beats hold") and
   require the crown to survive a parametric DGP (GARCH/OU) too — agreement
   across backtest TYPES is the real robustness signal.
9. **Test the DIFFERENCE of Sharpes (crown vs hold) properly** [34][35]. Inside
   our existing block bootstrap, compute the Ledoit–Wolf studentized
   Sharpe-difference and report whether its CI excludes zero, plus the closed-form
   PSR_strategy(SR_hold). Exploit that long/flat strategies are highly correlated
   with hold (smaller SE ⇒ easier to detect a real difference, AND correctly
   shows "mostly-hold" strategies are indistinguishable). Cheapest high-rigor fix
   tailored to our exact null.
10. **Rank with a manipulation-proof score, not just Sharpe** [28]. Add MPPM Θ
   (power-utility certainty-equivalent growth) alongside Sharpe/DSR and compare
   the crown's Θ to buy-and-hold's Θ — blocks crowning a negatively-skewed
   "insurance-seller." Just a power-mean over stored returns.
11. **Report the Sharpe haircut as a RANGE** [29][31]. Show both the conservative
   DSR/Bonferroni-haircut [1][29] and a gentler empirical-Bayes/publication-bias
   shrink [31]; the nonlinear haircut [29] will gut any crown that only marginally
   beats hold — which is the expected case.
12. **Add a power / detectable-effect-size line to every verdict** [32]. Distinguish
   "real no-edge" from "insufficient data/power"; pair with MinBTL [3]/MinTRL [34].
13. **Add a one-bar-delay robustness check** [33]: re-run the crown with signal-at-t
   / trade-at-t+1; an edge that vanishes is a look-ahead artifact ([15][17]).
14. **(round-2) Use BBC-CV as the deflation engine on machinery we already run**
   [64]. Treat the per-config bar-return matrix as the OOS-prediction matrix Π;
   BLOCK-bootstrap the bars [10][20], RE-SELECT the bake-off winner each resample,
   record its out-of-bag performance vs hold. The center is a selection-bias-
   CORRECTED expected performance; the spread is honest uncertainty. → A direct,
   resampling alternative to closed-form DSR that captures our exact multiple-
   testing optimism. Strong candidate to BE the deflation step.
15. **(round-2) Output the Model Confidence Set, and flag when hold is in it** [61].
   Report the bootstrap-determined SET of strategies statistically tied for best; if
   buy-and-hold is in the MCS, the crown does NOT robustly beat holding — our thesis
   as a test. Short/noisy windows correctly yield a LARGE set (built-in honesty).
16. **(round-2) Add a forward-run TAIL-CALIBRATION panel** [48][49][50][52][74].
   Our bootstrap already emits a loss distribution; check the crown's (and hold's)
   predicted downside on the forward run with Kupiec POF (rate) + Christoffersen
   LR_cc (no CLUSTERING — vital for crypto), ideally an ES/multinomial severity
   test, and report MAX-DRAWDOWN/Calmar DISTRIBUTIONS not point values. Collapse to
   a green/amber/red badge [53]. Caveat the LOW power on short windows [48][54].
17. **(round-2) Make block-length a SENSITIVITY band, not a single value** [74][20]
   [44]. Re-run the gate over a small grid of block lengths; require the "beats
   hold?" verdict to be STABLE. A verdict that flips on block length isn't robust —
   a concrete, cheap instance of the specification-curve discipline.
18. **(round-2) Add a parameter-NEIGHBORHOOD-STABILITY check + a timing-SKILL test**
   [68][72][65]. Require the crown's ±1-2-step parameter neighbors to also beat hold
   (reject lone spikes in the parameter surface), and for timing strategies run the
   Henriksson–Merton 2×2 test (signal vs next-bar direction; skill ⇔ p1+p2>1) as a
   luck-vs-skill badge — a crown can beat hold by luck yet fail this.
19. **(round-2) Apply a heavier decay / "well-known signal" discount, and run a
   stationarity pre-check** [57][58][76][71]. Expect the forward run to underperform
   the crown by ≥26% as NORMAL [57]; discount textbook indicators (decades-public,
   likely self-destructed [76]) extra; penalize complexity + outlier-sensitivity
   [58]; and Bai–Perron/CUSUM-check the window for structural breaks before crowning
   on a regime-straddling window [71].
20. **(round-3) Use the EFFECTIVE-N ONC clustering as THE trial count for DSR** [86]
   [80]. Cluster the candidate return-correlation matrix, count clusters = `N_eff`;
   inflate toward `(single-signal count)^k` for composed strategies. This is the
   single decision that makes the DSR haircut correct rather than punitive — implement
   it first within the gate add.
21. **(round-3) Switch the bootstrap to CIRCULAR (wrap-around) blocks and set length
   from the correlogram** [89][78][20]. Removes the fixed-length edge bias (boundary
   bars under-sampled → understated tail risk) and makes block length principled;
   keep a sensitivity band [74]. Low-risk, self-contained hardening of the FROZEN gate.
22. **(round-3) STUDENTIZE the gate statistic (SPA/stepwise-SPA) for power** [93][99]
   [7][8]. [93] is the empirical proof an RC-style gate can be too conservative to
   detect a real edge; divide each config's excess-return-vs-hold by its bootstrap std
   and use a sample-dependent null, so the many bad configs we sweep don't bury a
   genuine winner. Crypto inefficiency [88][99] means the gate must be able to detect.
23. **(round-3) Don't over-deflate; report the DEFLATED statistic, not a binary** [84]
   [81][100]. A high-deflated-t crown (t≫3–4 after deflating by `N_eff`) is the rare
   genuine winner; show the operator the deflated number so it is distinguishable from
   the common marginal crown the gate correctly rejects. Calibrated skepticism, not
   nihilism.
24. **(round-3) Frame the forward leg honestly: low-power, nested, split-dependent**
   [97][96][98][91]. A forward "loss" may be low power (the bootstrap recovers it);
   the crown-vs-hold test is nested (use CW adjustment [96] if formal); vary the
   window/split and pre-commit it [98]; select-and-score on different data [91]. Treat
   the forward run as a confidence check, the bootstrap as the verdict.
25. **(round-3) Report turnover and lean on cost-turnover asymmetry** [82][83][95].
   Costs are strongly turnover-dependent and paper returns overstate realized; surface
   each crown's turnover next to its net edge, give high-turnover crowns extra
   skepticism. For a €200 BTC order, fixed fee+spread is the right model (√-impact ≈ 0
   on Bitcoin [95]); impact only matters at size. Structurally favors hold.

## Open questions / things worth testing in our app

- Is our 1000-path gate framed RC-style (best-vs-benchmark over ALL configs) or
  as a single-strategy test? (verify in `crates/backtest`) — determines whether
  it corrects the bake-off's selection bias at all. [6]
- What block length does our moving-block bootstrap use, and is it ≥ the
  data-driven optimum for crypto's autocorrelation? [10][20]
- Do our IS/OOS / bootstrap blocks purge+embargo overlapping-label leakage at
  boundaries? Indicators with lookback windows (SMA-200, Bollinger) create
  feature overlap. [9]
- (RESOLVED-in-method) The EFFECTIVE number of independent trials = number of
  CLUSTERS of config return series (angular-distance clustering), not raw N
  [46][39]. Open: implement it — cluster our sweep's return matrix and feed m_eff
  to DSR. Likely m_eff ≪ raw config count, making the gate fair not punitive.
- Does our cost spec widen the spread in high-volatility regimes (2–3×), or is it
  a flat constant that understates costs exactly where churny crowns trade? [47]
- Do we RECORD every config's return series in the bake-off output (needed for
  m_eff, PBO, FDR), or only the winner's? [46][2][30]
- Does the forward paper-trade show the NEGATIVE-OOS signature [3] predicts under
  mean reversion, vs merely zero excess return?
- Are any crowned strategies' edges concentrated in 1–2 outlier days? [17]
- Are dead/delisted coins absent from our universe, biasing cross-coin claims? [14]
- (round-2) Do we report a tail-CALIBRATION check on the forward run — at minimum
  Kupiec POF + Christoffersen CLUSTERING (LR_cc), ideally an ES/multinomial test —
  for the crown's AND hold's predicted downside band? [48][49][50][52]
- (round-2) Does the gate run a STATIONARITY / structural-break pre-check on the
  (coin,window) so we don't crown on a window straddling a regime shift? [71][76]
- (round-2) Do we test the crown's PARAMETER NEIGHBORHOOD (do ±1-2-step neighbors
  also beat hold) to reject lone-spike overfits? [68][72]
- (round-2) Does the gate vary the BLOCK LENGTH and require the "beats hold?"
  verdict to be stable (spec-curve), rather than fixing one length? [74][44][20]
- (round-2) Could we replace/augment the deflation with BBC-CV on our per-config
  return matrix (bootstrap, re-select winner, measure out-of-bag vs hold)? [64]
- (round-2) Should ranking output the MODEL CONFIDENCE SET and flag when
  buy-and-hold is inside it (= crown not robustly better)? [61]
- (round-2) For timing strategies, do we run the Henriksson–Merton 2×2 skill test
  (signal vs next-bar direction; p1+p2>1) as a luck-vs-skill badge? [65]
- (round-2) If the advisor becomes a standing service, do we control online-FDR
  across the SEQUENCE of bake-off re-runs (decaying memory)? [73]
- (round-2) Do we apply a heavier decay/"well-known signal" prior given that our
  indicators (SMA/RSI/MACD) are decades-public and likely self-destructed? [57][76]
- (round-3, SPEC-READY) Implement DSR = PSR(E[max SR]) with `N_eff` = ONC cluster
  count of the candidate return matrix (composed → `(single)^k`); crown iff DSR>0.95
  AND beats hold. Everything needed is in the per-config return matrix. [1][86][80]
- (round-3) Is our gate statistic STUDENTIZED (excess-return-vs-hold ÷ bootstrap std)
  and is its null sample-dependent (SPA-style), or raw-RC-style and possibly too
  conservative to detect a real crypto edge? [93][7][99]
- (round-3) Does the bootstrap use CIRCULAR (wrap-around) blocks to avoid the fixed-
  length edge bias, and is the block length from the correlogram (not a constant)?
  [89][78][20]
- (round-3) When we report forward performance, do we account for its LOW power (don't
  read a forward "loss" as definitive) and is the crown-vs-hold OOS test nested-model-
  adjusted (Clark–West) if framed formally? [97][96]
- (round-3) Is the bake-off WINDOW / forward-split rule pre-committed and varied for
  stability, rather than one cherry-pickable cutoff? [98][90]
- (round-3) Do we surface each crown's TURNOVER next to its net edge so high-turnover
  crowns (where costs bite [82]) get extra skepticism? [82][83]

## Paper map (claim → supporting [N])

- Overfitting is trivially easy / the default → [2][3][21]
- Expected max Sharpe ≈ √(2 ln N); deflate it → [1][3]
- Multiple testing raises the t-stat / significance bar → [3][5]
- FWER vs FDR (Bonferroni/Holm vs BHY); FDR selects more rules → [5][24]
- Model-free overfitting probability (PBO/CSCV) → [2][21]
- PBO as a selection filter improves crypto OOS/crash returns → [21]
- Single-strategy confidence (PSR) + required track length (MinTRL/MinBTL) → [1][3][4]
- Sharpe is a statistic with error; naive √q annualization overstates it → [16]
- Bootstrap data-snooping test best-vs-benchmark (≈ OUR gate) → [6][10]
- More powerful / less conservative data-snooping tests → [7][8]
- Block length is data-dependent and first-order → [10][20]
- Purging + embargo + combinatorial paths (CPCV) → [9]
- Resampling backtests beat single-path walk-forward → [11][19][21]
- Three backtest types; agreement across them = robustness → [19]
- Hold-out / walk-forward are unreliable → [2][11][12][19]
- Overfitting → negative OOS under serial dependence → [3]
- Can't pick future-best rule ex ante (persistence fails) → [24][14]
- Transaction costs offset apparent technical-rule profit → [24][17]
- Market impact ≈ Y·σ·√(Q/V), square-root law → [13]
- Survivorship inflates returns + fakes predictability → [14]
- Look-ahead inflates returns; point-in-time discipline; agents fail honestly → [15]
- Seven sins taxonomy / engineering discipline → [17][19]
- Derive rules from a model to avoid backtest search-overfit → [18]
- Engine implementation / intra-candle correctness moves results → [22][23]
- Sequential orthogonalize-then-reselect for correlated factors (FWER) → [25][8]
- Most published anomalies fail a uniform conservative re-test → [26][5]
- ...but reproduction can be high; publication-bias haircut may be small → [31]
- Sharpe haircut is nonlinear (marginal gutted, high lightly trimmed) → [29]
- Performance metrics are gameable; manipulation-proof MPPM exists → [28]
- The core test is a DIFFERENCE of Sharpes (crown vs hold) → [34][35]
- Correlation between strategy and hold REDUCES the SE / required sample → [34]
- Studentized block-bootstrap CI for Sharpe difference (≈ our gate) → [35]
- FDR mixture: estimate proportion of configs with real edge vs hold → [30]
- Report statistical power / detectable effect; null is a valid result → [32]
- Optimal execution = E[impact] vs Var[timing risk]; efficient frontier → [27]
- One-bar trading-delay robustness check; time-series CV for tuning → [33]
- Regime-stratified verdicts (signals work only in some regimes) → [32][33]
- Type I vs Type II; strictness is a loss-function choice → [40]
- Correlated tests ⇒ use effective N (m_eff ≪ N); bootstrap not Bonferroni → [39]
- Non-standard errors (design choices) > sampling error; spec curve → [44][22][26]
- Expected max drawdown of Brownian (log/√T/linear by drift) → [43]
- Calmar ~monotonic in Sharpe (adds little independent info) → [43]
- Monte-Carlo backtest + DGP enables decommissioning; tactical>all-weather → [36]
- Portfolio-weight optimization is also a multiple-testing engine → [37]
- Up-front trial-count cap + IS→OOS consistency as a gate → [38][42]
- Square-root impact is a consequence of price diffusivity → [41]
- Informed trades ⇒ impact crosses √→LINEAR (worse cost at scale) → [41]
- Crypto perp costs (funding) + DSR double-screen in auto-tuning → [42]
- Best-of-large-rule-universe RC bootstrap; in-sample pass decays OOS → [45]
- Effective N = number of clusters of config return series (not raw N) → [46][39]
- Crypto spreads tiny but blow out 2–3× in stress; model state-aware costs → [47]
- VaR backtest: right breach RATE (Kupiec POF, χ²₁) → [48]
- VaR backtest: right rate AND no clustering (Christoffersen LR_cc; Markov) → [49]
- Duration/runs-based VaR backtest: more power in small samples → [54]
- ES IS backtestable despite non-elicitability (tail SEVERITY) → [50]
- PIT + censored-tail density test (whole predictive density) → [51]
- Multinomial multi-level VaR test implicitly backtests ES (N≥4 powerful) → [52]
- Traffic-light green/amber/red reporting for tail calibration → [53]
- All VaR/ES backtests are LOW-POWER in short samples → [48][54]
- Live-vs-backtest decay: ~26% OOS, ~58% post-publication → [57]
- Overfitting (complexity, outlier-sensitivity) predicts decay > arbitrage → [58]
- Expect forward run to underperform crown by ≥26% as the NORMAL case → [57][58]
- Transient/resilient market impact (LOB replenishment) → [55]
- Propagator model: decay kernel reconciles persistent flow + diffusive prices → [56]
- Point-in-time / vintage data: final-data eval overstates real-time skill → [59]
- Reporting-lag / restatement / PIT-universe look-ahead traps + fixes → [60]
- Model Confidence Set: bootstrap SET of best models (is hold IN it?) → [61]
- Over-fitting in model SELECTION; nested CV (inner=sweep, outer=forward) → [62]
- BBC-CV: bootstrap pooled OOS returns, re-select, get bias-corrected estimate → [64]
- Diebold–Mariano: arbitrary-loss HAC pairwise accuracy test; forecasts≠models → [63]
- Market-timing SKILL test (Henriksson–Merton 2×2; p1+p2>1) → [65]
- Allocation dominates; active timing/selection add little net of costs → [66]
- Shapley value: fair order-independent attribution of a composed crown → [67]
- Parameter neighborhood-stability; beat best-RANDOM; bagging-hurts=overfit → [68]
- Permutation null (shuffle returns) complements block bootstrap → [69]
- 25,988 TA rules; profit "illusory" after data-snooping correction → [70]
- Structural-break (Bai–Perron/CUSUM) stationarity guard for the window → [71]
- Multi-axis alpha evaluation (power/stability/robustness/diversity) > 1 metric → [72]
- Online FDR / alpha-investing controls false-"beats-hold" across re-runs → [73]
- Bootstrap drawdown/Calmar CIs; stationary bootstrap; never one block length → [74]
- DL fails to beat hold/linear baselines risk-adjusted, multiple-seed, OOS → [75]
- Discovered edges self-destruct → non-stationarity (EMH+forecasting) → [76]
- BTC TA/ML vs hold: 3/4 lose net of cost; lone "winner" uncorrected/one-window → [77]
- Moving-block bootstrap = our gate's parent; block length ℓ→∞, ℓ/n→0 → [78]
- CV/multi-fold valid over hold-out; k-fold OK iff residuals white → block+purge → [79]
- Composed multi-signal strategies inflate effective N toward (single)^k → [80]
- Publication bias explains only ~10–15%; high-t signals are largely real → [81]
- Costs are strongly turnover-dependent; high-turnover strategies crushed → [82]
- Paper returns overstate realized (value low / momentum ~zero net of cost) → [83]
- p-hacking CANNOT manufacture t≫4 → a real high-deflated-t region survives → [84]
- False Discovery Rate definition + BH step-up; BHY for dependence → [85]
- Effective N = clusters of candidate return matrix (ONC); False Strategy Thm → [86]
- Haircut Sharpe via Bonferroni/Holm/BHY, real-time strategy evaluation → [87]
- Discrete-FDR; TA edges CAN survive snooping in less-efficient markets → [88]
- Circular (wrap-around) block bootstrap removes MBB edge/boundary bias → [89]
- "Garden of forking paths": one analysis carries hidden multiplicity → [90]
- Select + score on SAME data = biased; nested CV gives unbiased estimate → [91]
- Robustness-aware composite objective (optimize-for-generalization) → [92]
- RC LACKS POWER, SPA is the upgrade; "nothing beats GARCH(1,1)" → [93]
- Conditional (not unconditional) predictive ability; misspecification-robust → [94]
- √-impact law holds on BITCOIN; fixed fee+spread right at retail scale → [95]
- Nested-model OOS: estimating zero-value params biases naive test (CW adj) → [96]
- OOS tests have LOWER power than IS and aren't snoop-proof → [97]
- Sample-split choice is itself data-mining; vary it / pre-commit → [98]
- Stepwise-SPA: more powerful than StepM; full SET of gate-passers → [99]
- Bootstrap skill-vs-luck of the cross-section; small right-tail is skilled → [100]
