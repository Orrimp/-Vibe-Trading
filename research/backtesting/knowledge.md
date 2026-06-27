# Knowledge — Backtesting & Test Data

_Synthesis of the `backtesting` ledger (47 papers). Payoff focus: concrete ways
to harden our FROZEN robustness gate (1000-path moving-block bootstrap vs
buy-and-hold), our ranking, and our test-data pipeline — and to avoid
overfitting. Numbers in [brackets] reference papers.md entries._

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

## Actionable takeaways for our advisor (ranked by leverage)

1. **Surface N and add a Deflated-Sharpe / PBO scorecard to the ranking.** Use the
   EFFECTIVE number of independent trials m_eff — cluster config return series by
   angular distance and count clusters [46][39], NOT the raw config count (configs
   are correlated; raw N over-deflates). Report the crowned Sharpe AND its
   DSR / E[max SR] haircut [1][3][46] and a CSCV-PBO [2]. Prerequisite discipline:
   RECORD every config's return series (not just the winner's) so m_eff, PBO, FDR
   [30] are all computable. Optionally DISQUALIFY high-PBO crowns [21]. → Stops us
   from crowning in-sample noise; highest-value import.
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
