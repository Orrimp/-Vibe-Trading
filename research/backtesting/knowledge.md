# Knowledge — Backtesting & Test Data

_Synthesis of the `backtesting` ledger (24 papers). Payoff focus: concrete ways
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
- **Seven-sins audit [17] + causal graph before backtest [19]** — cheap
  discipline checklists.

**Don't / weaker:**
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

1. **Surface N and add a Deflated-Sharpe / PBO scorecard to the ranking.** N =
   every config swept (including dominated ones). Report the crowned Sharpe AND
   its DSR / E[max SR] haircut [1][3] and a CSCV-PBO [2]. Both reuse data we
   already produce. Optionally DISQUALIFY high-PBO crowns [21]. → Stops us from
   crowning in-sample noise; this is the highest-value import.
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

## Open questions / things worth testing in our app

- Is our 1000-path gate framed RC-style (best-vs-benchmark over ALL configs) or
  as a single-strategy test? (verify in `crates/backtest`) — determines whether
  it corrects the bake-off's selection bias at all. [6]
- What block length does our moving-block bootstrap use, and is it ≥ the
  data-driven optimum for crypto's autocorrelation? [10][20]
- Do our IS/OOS / bootstrap blocks purge+embargo overlapping-label leakage at
  boundaries? Indicators with lookback windows (SMA-200, Bollinger) create
  feature overlap. [9]
- What is the EFFECTIVE number of independent trials (configs are correlated)?
  Sets the right N for DSR/Bonferroni — likely ≪ raw config count. [8]
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
