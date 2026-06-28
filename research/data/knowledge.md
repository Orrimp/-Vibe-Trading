# Knowledge — Data (the DATA itself)

*Synthesized from `papers.md`. Updated every ~5 papers. Lens: how each paper
informs OUR data discipline + our moving-block-bootstrap robustness gate +
synthetic-test-data ideas.*

## Key themes

1. **Dependent-data resampling is a design space, not one method.** The block-
   bootstrap family (moving / circular / stationary / non-overlapping / tapered)
   plus model-based variants (sieve, residual, Markov) all aim to preserve serial
   dependence i.i.d. bootstrap destroys. Block length is the load-bearing tuning
   knob. [1]
2. **Leakage is bigger than the train/test split.** Chronological splits are
   necessary but not sufficient; *decision-time* information availability and
   *execution alignment* dominate. Centered/look-ahead rolling features and
   same-bar execution are the largest silent Sharpe-inflators. [2]
3. **Naive cross-validation manufactures illusory skill on financial data.**
   Purging + embargo (and CPCV) are required; quantified, the gap is enormous
   (Bitcoin returns: naive R² +0.85 vs purged −1.2). [3]
4. **GAN/synthetic market data is seductive but fragile.** GANs can learn easy
   distributional shape and vol clustering, but reliably miss jumps, fat tails,
   mean-reversion speed, and cross-asset dependence; training is unstable. [4][5]
5. **Synthetic-data validation is itself a hard, subjective problem.** Hand-
   picked stylized-fact checks introduce bias; learned summaries are less
   gameable but require their own machinery. [6]
6. **Survivorship/universe bias is a selection problem.** Quantified at ~5pp
   annual return inflation in an emerging small-cap index; for us it reappears
   as *which coins we choose* and *how we handle delistings*. [7]
7. **Multiple testing manufactures winners.** Trial count vs minimum backtest
   length is a hard mathematical relationship; not disclosing N trials makes a
   backtest meaningless. Our bake-off IS a multiple-testing engine. [8]
8. **Independent corroboration of "holding wins net of costs."** Classic
   technical rules, tested with the *same* bootstrap-null machinery we use, beat
   buy-and-hold gross but die net of transaction costs. [9]
9. **Path-aware labeling (triple-barrier) > fixed-horizon returns**, but even
   good labels + deep models yield thin edge (F1≈0.43, 3-class). [10]
10. **Synthetic-data SOTA is moving from GANs → diffusion** (more stable, better
    coverage), but no generator yet reproduces ALL stylized facts; heavy for a
    bar-level single-coin advisor. [11]
11. **Generic time-series augmentation is dangerous for finance.** Jitter/
    rotation/magnitude-warp inject physically meaningless artifacts; only
    dependence-preserving block/permutation resampling is defensible. [12]
12. **The dependence structure comes from the TEMPORAL POSITIONS of large moves,
    not just fat tails** — the exact thing i.i.d. bootstrap destroys and block
    bootstrap preserves; loss clusters are more severe than gain clusters. [13]
13. **Overfitting-aware objectives beat Sharpe-maximization** for out-of-sample
    generalization; bake a benchmark-significance gate into the objective. [14]
14. **AI/ML adoption causes alpha decay.** Crowding + reflexivity collapse signal
    half-lives (5–7yr → ~18mo); commoditized signals (SMA/RSI/MACD) decay
    fastest — any crowned edge is time-decaying, not permanent. [15]
15. **Effective sample size ≪ raw row count** in finance (autocorrelation);
    non-stationarity + low signal-to-noise are the core ML-in-finance obstacles —
    the deep reason a single backtest is unreliable. [16]
16. **Too-good backtests usually hide missing controls** (no costs, no purge, no
    multiple-testing adjustment, short window) — read them skeptically. [17][9]
17. **Block length should be chosen from the data's correlation decay** (Politis-
    White spectral plug-in), not ad-hoc; stronger persistence ⇒ longer blocks. [18]
18. **Crypto data quality is a first-order risk:** >70% of unregulated-exchange
    volume is wash-traded — volume signals are suspect; source provenance matters;
    Benford/size-rounding are cheap data-quality gates. [19]
19. **Look-ahead bias is measurable, not theoretical** (PIT framing); use values
    stamped with when they became known; data revisions/backfills are a subtle
    leak; the same MA/momentum/mean-reversion family inflates under it. [20]
20. **CPCV is the modern standard of evidence** in applied financial ML — a
    *distribution* of OOS performances over many backtest paths, yielding PBO +
    deflated Sharpe; complementary lens to our return-block bootstrap. [21][3]
21. **Augmentation only helps when there's signal** to begin with; you fix a
    generator's blind spots by encoding domain structure (priors), not more
    data. [22]
22. **THE data-snooping result:** 7,846 technical rules, in-sample winner looks
    significant but FAILS out-of-sample once snooping-corrected (White's Reality
    Check). Closest academic analogue to our bake-off + thesis. [23]
23. **Robustness = worst-adverse-subperiod, not average** (window-robust /
    distributionally-robust objective) — external validation of our weakest-link
    verdict; ensure crisis windows are represented. [24]
24. **The Deflated Sharpe Ratio is the implementable multiple-testing fix:** PSR
    + DSR + Minimum Track Record Length deflate a Sharpe by trial count, sample
    length, skew, and kurtosis — crypto's fat tails make this especially needed. [25]
25. **Selecting the best of many noisy trials = rewarding luck** (the "illusion
    of success"); fair selection is *prospective* (does it hold forward?), not
    historical. [26]
26. **Overfitting can be a rejectable hypothesis test**, applied as a pre-deploy
    FILTER — less-overfit crypto agents beat the benchmark over a crash window;
    surviving honest scrutiny is rare and fragile. [27]
27. **Cost-model implementation is THE silent source of backtest divergence** —
    same strategy, 5 engines, disagreement driven almost entirely by transaction-
    cost handling (incl. a "÷100 commission" bug); high-turnover hit hardest. [28]
28. **IS / WFA / OOS must be strictly non-overlapping;** a true holdout never
    touched during tuning is the discipline; walk-forward catches one-window
    flukes. [29]
29. **Even a careful 2025 GAN underestimates tails & misses structural breaks**
    on volatile/EM-like assets — i.e. exactly where crypto lives; synthetic test
    data would understate crash risk. [30]
30. **LOB/tick data is out of scope for a bar-level advisor**, but it teaches a
    portable lesson: a model can score well on a statistical metric (accuracy/F1,
    R²/Sharpe) yet produce no tradable net edge — judge by net P&L vs B&H. [31]
31. **Synthetic-data design space has two branches:** *learned* (GAN/diffusion/
    **copula**/VAE — [4][5][11][32][33]) and *mechanistic* (**agent-based
    simulators** ABIDES/ZI/Chiarella — [6][34]). Both need stylized-fact
    validation and both are heavier + less reproducible than block-resampling
    real returns. Copulas cleanly separate marginals (tails) from dependence. [32][33][34]
32. **Validate any generator on known-ground-truth synthetic data BEFORE real
    data**, scoring rolling-window moments, not just the unconditional
    distribution. [32][4]
33. **Missing-data imputation is a leakage surface.** Never impute with future
    bars; an imputed bar is not a real observation (bootstrapping over fills
    understates uncertainty). Crypto's 24/7 trading makes gaps rarer but outages/
    delistings/feed glitches still occur — prefer flag/exclude or causal fill. [35]
34. **The "Seven Sins" checklist** (survivorship, look-ahead, storytelling, data-
    mining, costs, outliers, asymmetric-shorting) maps ~1:1 onto our gates; the
    *outlier* sin is new emphasis — a config that wins only on one or two extreme
    moves is fragile, and the bootstrap should expose it. [36]
35. **Two opposite kinds of "outlier"**: data-error prints (clean/remove) vs real
    extreme events (KEEP — they're the tail risk the gate exists to stress). Never
    silently winsorize away real crash bars. [37][36]
36. **The sampling unit is a data decision: time bars are statistically inferior**
    (oversample noise, undersample information) vs information-driven (tick/volume/
    dollar) bars. We use daily time bars — defensible for an operator-legible
    retail advisor, and *safer* in crypto because volume/dollar bars inherit wash-
    trade contamination [19]; but it explains why our daily returns are non-normal +
    serially correlated (⇒ block bootstrap + non-normal metrics are right). [38]
37. **Feature-store discipline names our consistency gates:** *point-in-time
    correct retrieval* = our PIT/look-ahead rule [2][20]; *training–serving skew* =
    our F5 forward-fidelity gap (bake-off ranks one impl, forward runs an SMA
    proxy) — fix is "one strategy definition, used everywhere." [39]
38. **Overlapping labels are even less independent than bars** (concurrency);
    correct with uniqueness sample-weights + sequential bootstrap + purge/embargo —
    ONLY if we add horizon labels; reinforces effective-N ≪ raw-N. [40][16]
39. **Stationarity-vs-memory tradeoff:** integer differencing (returns) kills the
    level memory; fractional differentiation keeps it at minimum d for
    stationarity. Crypto's trends live in the non-stationary level — explains why
    trend rules exist *and* why they're fragile (level ≈ random walk). [41]
40. **Stylized facts are ASSET-SPECIFIC, not universal** (8/11 of Cont's hold in
    modern equities, non-uniform across stocks). Our block bootstrap inherits
    *this coin's* actual facts rather than imposing a generic template — a strength
    vs any equity-tuned generator. [42]
41. **Bitcoin's stylized facts are strong AND drifting:** q-Gaussian heavy tails,
    power-law absolute-return ACF, multifractality; but **Hurst rose 0.42→0.49**
    (efficiency increasing toward random walk) — independent support that
    exploitable autocorrelation is *shrinking over time* (alpha decay). [43]
42. **Crypto volatility differs from equities in 3 ways:** inverse leverage effect
    (positive returns raise vol — sign-flipped vs equities), lower vol persistence,
    and jumps that matter more. Jumps dominating is the strongest case AGAINST
    synthetic generators (they under-model jumps) and FOR real-data block
    bootstrap. [44]
43. **Online (causal) change-point detection (BOCPD)** is the leakage-free way to
    ask "are we in a new regime?" — but as a *diagnostic/monitor*, NOT a trading
    rule (regime-switching rules are another overfit trap). The AR-with-time-
    varying-variance variant fits financial data better. [45][46]
44. **The stationary bootstrap (random/geometric block length)** is the canonical
    alternative to our fixed-length moving-block — it hedges across block scales
    and yields a strictly stationary resample; the natural block-scheme
    sensitivity check for our gate. [47]
45. **"Most ML funds fail for DATA reasons, not model reasons"** — de Prado's 10
    pitfalls (sample uniqueness, stationarity/memory, overfitting, multiple
    testing, chronology, CV, survivorship) are a super-checklist over this whole
    folder. [48]
46. **TSTR ("train on synthetic, test on real") is the honest acceptance test for
    synthetic data** — behavioral/task-level, not distributional similarity; a
    generator that lets a strategy look good but fail on real data fabricated
    structure. [49]
47. **Synthetic-data-for-RISK is especially dangerous:** even the best generator
    (TimeGAN) is single-index + training-unstable, and VAEs *smooth away extreme
    moves*, degrading risk/VaR estimation — the exact failure a crypto gate cannot
    afford. Evaluate fidelity + utility + robustness, not just fidelity. [50]
48. **Adversarial/stress robustness is REGIME-CONDITIONAL:** a model equally good
    in calm and stress can be ~2× more fragile under stress; average-case
    evaluation hides crisis fragility — judge by adverse-regime behavior. [51][24]
49. **Leakage (not model choice) is the dominant reason ML fails to reproduce:**
    8-type taxonomy, 329 papers across 17 fields; in conflict prediction, complex
    models' "advantage" over logistic regression vanished once the leak was fixed —
    a direct parallel to "no active strategy beats B&H once tested honestly." The
    "model info sheet" = adaptable leakage-audit disclosure for our runs. [52]
50. **Labeling is a data decision with a precision/recall split:** fixed-horizon
    is volatility/path-blind; triple-barrier is path-aware; **meta-labeling**
    (primary = direction/recall, secondary = act-or-not/precision) is an attractive
    *architecture lens* even without ML — our robustness gate is a "should we act?"
    filter in that spirit. [53][10]
51. **PBO via CSCV is computable on our exact bake-off output** (strategies × time
    matrix): "the in-sample winner lands below median OOS X% of the time" — a
    single honest robustness number + an overfit-rejection gate + a realistic
    performance-degradation haircut. Top implementable pick alongside DSR. [54]
52. **CEX price data is higher-quality than DEX** (efficiency <5bps vs 10–50bps;
    gas distorts DEX); small retail trades are cheapest on CEX. Source a reputable
    CEX/CEX-aggregate feed, stamp provenance, and use CEX-realistic costs. [55][19]
53. **Honest data cleaning = separate noise from signal with a NULL MODEL** (RMT/
    Marčenko-Pastur band for covariance; our bootstrap's null for an equity curve).
    Covariance denoising itself is out of scope (single coin). [56]
54. **Even rigorous, interpretable, cost-aware daily-bar signals net ~0%** (Sharpe
    0.33, p=0.34, honestly reported with ~12% power) and are strongly regime-
    dependent — a model of honest-null reporting and direct thesis corroboration.
    Many disjoint walk-forward folds catch the one-window fluke. [57]
55. **Select parameters by a STABLE PLATEAU, not the single peak** (peaks are
    overfit); double-OOS + smoothed grid + shuffled-block + bootstrap generalize
    far better. **On CRYPTO (BTC/BNB/ETH): in-sample beats B&H, OOS only MATCHES
    B&H (lower drawdown), and B&H+strategy BLENDS win** — a near-exact replica of
    our thesis + a product idea (surface a blend). ~0.4% break-even cost. [58]
56. **Augmentation is a small-data crutch** (400% gain on 30K samples → 40% on
    255K); only causal/dependence-preserving transforms (Reverse/sign-flip HURT);
    and it lifts financial metrics without lifting accuracy — for our large single-
    coin history + evaluation use-case, of little value. [59][12]
57. **GAN augmentation can help ML FORECASTING on scarce crypto data** (lower MSE,
    Bitcoin>equities) — but lower MSE ≠ tradable edge, and augmenting *training*
    data is a different job from our *evaluation* bootstrap (fabricating data is a
    liability in the gate). [60]
58. **The synthetic-data evaluation battery** = distributional + temporal +
    stylized-fact + downstream-utility + (privacy); judge on utility & stylized
    facts, not distributional fit alone. The **"trilemma"** (interpretability vs
    temporal realism vs feasibility) is exactly why block bootstrap wins — it
    sidesteps generation entirely. VAEs smooth extremes (again). [61][50]
59. **"Derive rule parameters from the process, don't only search"** (OU optimal-
    stopping for stop/take levels) avoids grid-search overfitting — but the OU/
    stationary-mean-reversion assumption misfits trend/jump-heavy crypto; the
    usable hybrid is vol-scaled (data-derived) thresholds à la triple-barrier. [62]
60. **DFDR (discrete false-discovery-rate) is a more powerful multiple-testing
    correction** than Reality Check; on MSCI indices some TA value survives
    correction BUT only conditionally on regime + with frequent rebalancing (=
    high turnover = cost-killed) — keeps us honest without overturning the thesis. [63]
61. **Field-standard data cleaning = robust (median/MAD) loose outlier filter**
    (>10 MAD from a rolling median) after deterministic checks (zero/neg prices,
    single source, merge same-timestamp) — loose by design so it removes errors,
    not real crash bars; make it trailing-only if it ever feeds a live decision. [64]
62. **Diffusion (esp. interpretable trend/seasonal decomposition) is the least-bad
    generator family** if forced off real data (more stable than GANs, steerable),
    but general-benchmark fidelity ≠ crypto jump/tail preservation; block bootstrap
    stays the default. [65][11]

## Round-3 additions (66–100), organized by sub-theme

*Lens throughout: **what protects our gate from crowning a lucky single path.**
The new papers deepen five sub-themes — they motivate our 1000-path moving-block
bootstrap and the planned Deflated-Sharpe / PBO add directly.*

### A. Splits, leakage & multiple testing (the core defense)
- **The MBB our gate runs has a name, a proof, AND a precise reason the block
  length matters.** Künsch (1989) [84] is the foundational moving-block bootstrap —
  resample blocks of l consecutive obs, with consistency requiring **l→∞ and
  l/n→0**. *(Full-text deepening:)* what the MBB variance estimate actually computes
  is the **spectral density of the returns at zero frequency = the long-run
  variance** (the *sum of all autocovariances*, not the lag-0 variance). That is
  exactly why the block length is load-bearing for *us*: volatility-clustered crypto
  returns have non-trivial autocovariance out to long lags, so a block **too short
  to span the coin's correlation length under-estimates the long-run variance →
  over-narrow, over-optimistic confidence bands** (the gate would look more decisive
  than the data warrant). This turns the abstract "l→∞, l/n→0" into a concrete
  failure mode to guard. Block-length selectors converge *slowly* (n^-1/6 to n^-1/3)
  [89] and the optimum differs for **quantile** targets (drawdown/CVaR) vs variance
  targets [88] — so the honest posture is: pick a defensible length (Politis-White
  [18]), document it, and **sensitivity-check the verdict** across nearby lengths
  and MBB↔stationary [47]. A stationary bootstrap gives "reasonable and stable
  estimation for *any* quantity from one single time series" [100] — cross-
  disciplinary (statistical-physics) confirmation that resampling one correlated
  path beats trusting a point estimate.
- **Selection/tuning leakage is the *worst* kind — now quantified, and it IS the
  data-side proof of why we need DSR/MinBTL.** *(Full-text deepening [80]:)* across
  2,047 datasets × 6 algorithms, preprocessing leakage (normalization/PCA) is
  **negligible (|ΔAUC| < 0.005)** while **selection leakage — peeking at the
  best-of-K — is ~40× larger (ΔAUC ≈ +0.040)**. The decisive mechanism: selection
  inflation **= σ·√(2·ln K) + genuine-diversity**, where K = number of trials, and
  this noise term **decays only as O(1/√n)** — and at small n (~1,900) **~90% of the
  inflation is pure noise exploitation, not skill.** Map to us: our bake-off is a
  best-of-K selection machine with **large K (configs × params) and small n (one
  coin's history)** — both factors push our crown toward maximal inflation. So the
  σ√(2 ln K)/√n result is the *empirical, data-side* twin of the Deflated-Sharpe /
  Minimum-Backtest-Length argument: deflate the crown by trial count, and expect the
  deflation to be *large* precisely because our sample is small. Discipline: the
  window used to crown a config must never be the window it's later judged on
  [29][58], and the crown must be treated as a multiple-testing event.
- **Multiple-testing deflation now has four interchangeable tools**, all consuming
  our existing bake-off output: Deflated/Probabilistic Sharpe + MTRL [25], **PBO
  via CSCV** [54], the **Harvey-Liu haircut** (non-linear in Sharpe magnitude;
  Bonferroni/Holm/BHY) [73][96], and a **complexity (covariance) penalty** [98].
  A controlled-environment comparison ranks **CPCV best, plain Walk-Forward worst**
  by PBO and DSR [79] (Walk-Forward's diagnosed weakness: a single chronological
  path has **weaker stationarity + higher temporal variability**) — independent
  support that *single-path* evaluation is the weakest option and our *multi-path*
  bootstrap is the right philosophy.
- **A single hold-out — i.e. our forward paper-trade alone — is NOT a sufficient
  overfitting check.** *(New, from the PBO full text [54]:)* Bailey et al. devote a
  section to dismantling the hold-out method: (i) on public data the hold-out is
  likely already in-sample; (ii) seasoned researchers *know* how the hold-out period
  behaved and design to it; (iii) hold-out is inadequate for **small samples** —
  shouldn't be used with < 1,000 obs (a weekly strategy needs > 20 years); (iv)
  **high variance** — which hold-out you pick can refute a valid strategy or bless
  an invalid one; (v) it **ignores the number of trials**. Reasons (iv)+(v) land
  squarely on our forward-paper-trade-as-OOS: it is one high-variance path that
  doesn't know how many configs we swept. The honest implication: the forward paper-
  trade is *necessary* (genuine unseen data) but must be **paired** with a CSCV/PBO
  on the bake-off matrix and a DSR/MinBTL on the crown — the multi-path/trial-aware
  antidote to single-hold-out variance, the same "single path is weakest" message
  as [79][84].
- **Look-ahead can hide in a *model*, not just the pipeline.** A pretrained LLM
  leaks the future through what it memorized; removing it is an open problem
  [67][20]. The benchmark can also leak: constituent-reconstitution ("look-ahead
  benchmark bias") inflates Sharpe up to ~8%/yr [68] — we're immune (single-coin
  B&H) except at the universe/coin-selection level [7].
- **Out-of-sample decay and non-replication are real and quantified — with a
  correction.** *(Full-text fix:)* McLean-Pontiff [94] (82 predictors, 68 studies)
  decompose decay into a **pure-statistical/data-mining component of only ~10% —
  and statistically *insignificant*** (a 5% in-sample alpha → ~4.5% out-of-sample,
  cannot reject zero bias) — plus a **post-publication decay of ~35%** (→ ~3.25%,
  significant), implying a **~25% arbitrage/crowding effect**. (The ledger's
  earlier "26% / 58%" was wrong and is now corrected against the source PDF.) The
  ~10% floor is for *single, refereed* hypotheses; our bake-off crowns the max of
  *many* configs, a far more aggressive selection setup, so our selection inflation
  should be **larger** than 10% — see the §A σ√(2 ln K) result. Separately,
  **64–85% of equity anomalies don't survive** a disciplined, microcap-robust,
  multiple-testing protocol [95] (liquidity signals are the worst, 93% fail). Net:
  refereed edges decay materially and most don't replicate; an un-refereed bake-off
  winner should be expected to do at least as badly — hard backing for "no active
  strategy robustly beats holding."

### B. Synthetic / Monte-Carlo data (why we resample reality)
- **The whole generator zoo is now mapped**: GAN (TimeGAN [76], Quant-GAN [5],
  Deep-Hedging market simulator [92]), **signature** methods (Sig-WGAN [85], SOCK
  feature-matching [91]), diffusion [65][11], **causally-constrained VAE** [86],
  copulas [69][33], and agent-based (RL-agent crypto ABM [71]). Every branch is
  heavier, harder to validate, and less reproducible than block-resampling.
- **The decisive verdict for RISK is consistent, now with a ROOT CAUSE and an
  honest caveat.** A VaR-focused review (14 models) finds **Historical Simulation,
  GARCH, and one CWGAN are the top performers** [87] — Historical Simulation is the
  non-block sibling of our bootstrap, i.e. *resampling reality wins for risk*. VAEs
  repeatedly **smooth away extremes** [50][61], and the rare-events survey [72]
  *(full-text deepening)* gives the **structural reason: a Gaussian latent prior
  fundamentally cannot generate heavy tails** — so the smoothing is architectural,
  not a tuning miss, and fixing it needs dedicated EVT machinery (Tail-GAN bakes
  VaR/ES into the loss; HTGAN swaps in a heavy-tailed prior) + tail-specific
  evaluation (extreme-coverage, extreme-magnitude, extremal-dependence). **Honest
  caveat that also bites our bootstrap:** plain HS (and our block resample) is
  *capped by the worst historical scenario* — it can reshuffle the crashes the coin
  actually had but **never invent a worse-severity one** [87]. That is the one real
  edge our method lacks; the principled fill is a deliberately tail-stressed slice
  or an EVT-augmented generator [72], not a generic GAN/VAE (which understate tails).
- **Small-sample = generators overfit the one path you have** [91] — the
  generator-side echo of effective-N≪raw-N [16][75]. The bootstrap *can't* overfit
  a path because it doesn't fit anything. Even the most *theoretically principled*
  generator — the causal-Wasserstein-bounded TC-VAE [86] *(full-text deepening)* —
  has a guarantee whose constant **C = 2(2^T − 1) blows up exponentially in path
  length T**, so the bound is near-vacuous for a realistic multi-hundred-bar
  backtest window: the guarantee degrades *exactly* where we'd use it. The block
  bootstrap [84] needs no such bound because it never leaves the real-data manifold
  (causal by construction: real blocks in real order). Net: **block bootstrap stays
  the default**; generators remain research-only, and if ever scoped, demand a
  causal constraint + utility/TSTR validation + explicit EVT-style tail check — and
  note that **crypto-crash synthesis specifically is still an open problem** [72].

### C. Point-in-time / data quality / provenance
- **Crypto price/volume integrity is a first-order risk** — pump-and-dump events
  are frequent and concentrated in thin coins [82] (on top of >70% wash volume
  [19]); both argue for a **large-liquid-coin selection scope** and skepticism on
  thin coins (the crypto analog of the microcap p-hacking trap [95]).
- **Crypto is 24/7 but not time-homogeneous** — real intraday/weekly seasonality
  clocked partly to funding settlements [83]; daily bars aggregate it away, a
  quiet point *for* our daily-bar choice ([38]), a caution if we ever go intraday.
  CEX remains where price discovery happens (DEX lags) [83][55].
- **The simulator is part of the data pipeline for trust.** Backtest-engine
  correctness hinges on **intra-bar fill assumptions** (which of a stop/target
  inside one candle fired first) [81] — an optimistic assumption silently inflates
  results, exactly like a mis-scaled cost [28]. Make the intra-bar assumption
  explicit, conservative, and tested.

### D. Labeling (path-aware, leakage-aware, regime-aware)
- **Triple-barrier remains the gold standard**; it pairs naturally with fractional
  differentiation in a crypto pipeline [74]. But **trend-from-future-turning-points
  labeling is leakage-prone** — large reported outperformance from a clever labeler
  (e.g. 498% vs B&H) is a red flag, not a result [66]. And **label noise is
  non-stationary** — labels are noisiest in exactly the volatile regimes a strategy
  most needs to be right [70]. "The data includes the labels," and label quality is
  its own first-class concern.

### E. Stationarity, stylized facts & crypto data-generating behavior
- **Cont (2001) [90] is the canonical stylized-fact checklist** — the formal reason
  i.i.d. bootstrap is wrong (volatility clustering / absolute-return ACF are
  *dependence* facts) and block bootstrap is right [84]. Its "statistical issues"
  caution (facts are robust qualitatively, parameters hard to pin down from finite
  samples) is the 2001 root of finite-sample skepticism [16][75][89].
- **Crypto's tails are large, recurring, and structural** — Bitcoin exceeded its
  Metcalfe fundamental in ≥4 bubble-and-burst episodes [93]; finite-sample bias
  contaminates even volatility/Hurst estimates [75]. This is the strongest case for
  preserving real extremes (block bootstrap) over any tail-smoothing generator, and
  for crisis-window inclusion + a worst-path/CVaR lens [24][51][88].

### F. Counterpoint cases (read skeptically — they prove the gate's value)
- Three recent crypto "we beat B&H" results — an **LSTM on one 2024 year** [77], an
  **SAE tuned to a bottleneck/noise sweet-spot** [74], and an **adaptive long-biased
  multi-coin trend book over 36 months** [99] — all share the same red flags: one
  window/regime, no multiple-testing correction, selection/long-bias repackaging
  market beta. They are *worked examples of why* our B&H-benchmarked, bootstrap-
  gated, DSR/PBO-corrected verdict discounts such headlines. Even the rigorous
  Oxford-Man DL benchmark [97] — which *does* beat linear baselines on futures —
  shows the net edge collapses at 5–10 bps of cost and demands seed/tail/regime
  robustness, exactly our evaluation philosophy.

## Methods / findings that hold up (and which don't)

- **Holds up:** Block bootstrap preserves within-block dependence and is the
  standard for weakly-dependent series [1]. Purging+embargo materially removes
  leakage; the effect is measurable and large [3]. Centered windows + same-bar
  fills inflate Sharpe by 15–30+ pts — robustly, across models/markets/years [2].
- **Holds up:** Multiple-testing/data-snooping corrections (White's Reality
  Check [9][23], Deflated Sharpe + MTRL [25], PBO via CSCV [8][27]) reliably
  shrink apparent edge to ~benchmark — the in-sample winner fails OOS [23].
  Block-length-from-correlogram (Politis-White [18]) is the principled way to
  set the one knob that matters [1][13]. Cost-model fidelity is a *correctness*
  property, not a parameter — it's the dominant source of engine divergence [28].
- **Holds up (as a caution):** GANs reproduce vol clustering/leverage [5][30] but
  fail at the tail/jump/multivariate features that matter most for survival
  [4][30]; diffusion is more stable but still doesn't nail all stylized facts [11].
- **Doesn't (for us):** Full LOB agent-based simulators — powerful but hard to
  calibrate [6] and irrelevant to a bar-level single-coin advisor. Generic CV
  augmentation (jitter/rotation/warp) — injects artifacts, meaningless for prices
  [12]. Trusting any single backtest path — effective sample size is tiny [16]
  and one path hides the variance the bootstrap/CPCV expose [21][24].

## Actionable takeaways for our advisor

1. **Bootstrap-variant sensitivity check.** Our FROZEN gate uses *fixed-length*
   moving-block. Run a one-off comparison (tapered + stationary/random-length)
   to confirm the weakest-link verdict is invariant to the block scheme. If it
   flips, the block choice is doing hidden work and must be documented. [1]
2. **Adopt a leakage-audit checklist as a gate.** Assert: (a) every indicator
   uses only trailing data through the decision bar (no centered windows); (b)
   fills happen on the *next* bar, never the signal bar's own OHLC; (c) any
   feature normalization is fit on train only. [2]
3. **Never trust a single in-sample fit on crypto.** The Bitcoin naive-vs-purged
   R² gap is the cautionary number; lean on OOS + bootstrap robustness. If we
   ever add ML/labels, purging+embargo are mandatory. [3]
4. **Prefer bootstrap of REAL returns over GAN synthesis** for stress/test data —
   it can't invent dynamics the asset never had and has no convergence risk. Only
   consider generative models for genuinely unseen regimes, with stylized-fact
   validation. [4][5]
5. **Consider surfacing Deflated/Probabilistic Sharpe + MinTRL** next to the
   buy-and-hold benchmark to express multiple-testing-adjusted confidence. [3][8]
6. **Track + display the trial count** (N strategies × N param settings) the
   bake-off swept — selection pressure is the hidden variable behind any crowned
   winner; without it the rank is overstated. [8]
7. **Guard the cost model.** The "significant gross, dead net" pattern recurs;
   ensure costs are never accidentally zeroed (a no-op cost resurrects phantom
   alpha — cf. our own v3-vol-overlay no-op precedent). [9]
8. **If we ever add labels/ML overlay, use triple-barrier (vol-scaled barriers +
   time limit), with purging+embargo** — not naive fixed-horizon returns. [10][3]
9. **Prefer diffusion over GANs if we ever generate synthetic paths**, but the
   default stays block-bootstrap of real returns (reproducible, no convergence
   risk, can't fabricate dynamics). Validate any synthetic data vs stylized
   facts: heavy tails, vol clustering, loss-cluster asymmetry. [11][13]
10. **Justify our block length explicitly** against the asset's volatility-
    cluster timescale — too short shreds crash dynamics & understates drawdown;
    document it as part of the FROZEN gate. [13][1]
11. **Test a GT-Score-style composite ranking** (return × benchmark-significance
    × consistency / downside-dev) vs raw return/Sharpe; likely fewer false
    winners crowned. [14]
12. **Set/justify our block length via Politis-White** (spectral plug-in from the
    coin's correlogram); verify the frozen value matches the data's correlation
    length rather than a guessed constant. [18]
13. **Add a data-source vetting + quality gate.** Prefer reputable/regulated
    exchange feeds; treat volume-based indicators as low-trust in crypto; consider
    a Benford/size-rounding sanity check on new coin histories. [19]
14. **Treat any crowned active edge as time-decaying** — keep re-validating the
    forward paper-trade vs buy-and-hold rather than trusting a one-time crown. [15]
15. **Consider a worst-path / CVaR statistic** across our 1000 bootstrap paths so
    the headline metric reflects adverse-regime survival, aligning with the
    distributionally-robust objective literature. [24]
16. **Out-of-sample forward check is non-negotiable** and is the step that kills
    snooped winners; our forward paper-trade IS this — protect it. [23][20]
17. **CPCV as an optional second robustness lens** (multi-path split-based) that
    natively yields PBO + deflated Sharpe alongside our return-block bootstrap. [21]
18. **Compute & report PSR / Deflated Sharpe / MTRL for the crowned config**,
    feeding in the real trial count + realized skew/kurtosis — the single most
    implementable honesty upgrade; crypto's fat tails make it bite harder. [25][8]
19. **Add an overfit-rejection gate (PBO threshold) before the forward run** so
    only configs that pass an explicit overfit hypothesis-test are paper-traded. [27]
20. **Give the cost model a dedicated audited test** (known-input fee assertion;
    explicit notional-vs-delta + pre/post-reallocation semantics) — high-turnover
    active configs are most sensitive, and a silent cost bug flatters them. [28]
21. **Audit IS/WFA/OOS separation:** ensure param-tuning never sweeps the window
    the winner is later judged on; keep tuning / validation / forward data
    strictly disjoint. [29]
22. **Judge any ML/predictive overlay by net P&L vs B&H, never by a statistical
    score** (accuracy/F1/R²/raw Sharpe) — high metric ≠ tradable edge. [31][9]
23. **If we ever build synthetic stress paths, validate on known-ground-truth
    data first** and score rolling-window moments; prefer real-data block
    bootstrap as default (no calibration/convergence burden vs GAN/diffusion/
    copula/ABM). [32][34][4]
24. **Set a causal, auditable gap-handling policy** for OHLCV: flag/exclude gappy
    windows, never impute with future bars, and never bootstrap over imputed bars
    as if they were real observations. [35]
25. **Adopt the "Seven Sins" as an explicit data-bias audit checklist** in our
    docs (we already address most); add an "outlier-dependence" check — does a
    crowned config's edge survive if its one or two biggest winning bars are
    resampled away? [36][37]
26. **Make outlier cleaning conservative + auditable:** distinguish impossible
    prints (instant-revert wicks, feed glitches → remove) from real violent moves
    (→ keep); a silent winsorizer would flatter every strategy and break the
    weakest-link verdict. [37]
27. **Document "why daily time bars"** and what it costs: time bars are
    statistically inferior, but operator-legible and wash-trade-safe vs volume/
    dollar bars in crypto; it justifies block bootstrap + non-normal metrics over
    a plain Sharpe + i.i.d. bootstrap. [38][19]
28. **Close the training–serving skew (F5):** ensure the forward paper-trade runs
    the *exact* strategy the bake-off ranked (reuse ComposedStrategy-from-TOML),
    not a proxy — the feature served must equal the feature evaluated. [39]
29. **Use THIS coin's empirical stylized facts as targets** (its tail index, ACF-
    decay exponent, leverage sign, Hurst) — not textbook/equity constants — for
    any synthetic-data validation or block-length justification; facts are asset-
    specific and crypto's leverage sign is flipped. [42][43][44]
30. **Track the Hurst / efficiency trend over the window**: if a coin's Hurst is
    drifting toward 0.5, exploitable autocorrelation is shrinking — a quantitative
    "is there even an edge to find?" pre-check that supports re-validation. [43][15]
31. **Block length must hold a crypto volatility/jump cluster:** crypto jumps
    dominate the tail and vol persistence is lower-but-spikier than equities, so
    pick/justify block length empirically per coin ([18]) rather than porting an
    equity default. [44][13][18]
32. **Compute PBO via CSCV on the bake-off matrix** (top implementable pick with
    DSR): report "in-sample winner falls below median OOS X% of the time" + a
    performance-degradation haircut + use it as the overfit-rejection gate. [54][27]
33. **Select the crowned config by a STABLE PARAMETER PLATEAU, not the single
    grid peak** — a cheap, high-value change that crowns far fewer flukes; combine
    with double-OOS and the existing block bootstrap. [58][26][14]
34. **Source price data from a reputable CEX / CEX-aggregate and stamp provenance**
    (CEX is more efficient than DEX, <5bps vs 10–50bps; volume is partly wash-
    traded); use CEX-realistic taker-fee + spread costs (~0.4% break-even sanity
    check). [55][19][58]
35. **Write a data-cleaning runbook**: reject impossible bars (zero/neg, high<low),
    fix duplicates/provenance, then a *loose* robust (median/MAD, ~10-MAD) outlier
    flag for review — never auto-winsorize; trailing-only if it feeds live. [64][37]
36. **Consider surfacing a B&H + strategy BLEND**, not only a single crowned active
    strategy — the crypto evidence shows blends cut drawdown ~50% while matching
    B&H, which fits an advisor's risk-aware framing. [58]
37. **Keep synthetic generators research-only; if ever scoped, use the full
    evaluation battery** (distributional + temporal + stylized-fact + downstream
    utility + TSTR), and prefer interpretable diffusion over GAN/VAE — but expect
    them to understate crypto jumps/tails. [61][49][65][50]
38. **Harden the selection/tuning boundary as the #1 leakage gate.** Tuning- and
    selection-stage leakage are empirically worse than preprocessing leakage [80];
    our bake-off's crown step is precisely that surface — enforce that the window
    used to select/tune a config is disjoint from the window it's judged on, and
    treat the act of selecting-the-best-of-N as a multiple-testing source. [80][29][58]
39. **Sensitivity-check block length AND scheme, because the "optimal" can't be
    pinned down.** Selectors converge slowly [89] and quantile (drawdown/CVaR)
    targets want a different scheme than variance targets [88]; confirm the
    weakest-link verdict is stable across nearby block lengths and MBB↔stationary
    [47][84][100] rather than betting on one value. [89][88][84]
40. **Surface a multiple-testing-adjusted number next to B&H — pick the cheapest to
    wire.** Of DSR/MTRL [25], PBO-via-CSCV [54], Harvey-Liu haircut [73][96], and a
    complexity penalty [98], the haircut and DSR are simplest on our existing
    bake-off output; CPCV-best/Walk-Forward-worst [79] argues our multi-path stance
    is right. Most active configs have *small* Sharpes — exactly where the haircut
    is largest (>50%) — so most crowned winners should deflate toward "= B&H." [73][25][54]
41. **Audit the engine's intra-bar fill assumption + add a test.** For any strategy
    with intra-bar exits (stop/take/triple-barrier levels), pin down which level
    fires first inside a daily candle; assume the *adverse* level conservatively, or
    resolve next-bar — an optimistic assumption silently inflates results like a
    cost bug. [81][28]
42. **Favor large, liquid coins; flag thin-coin histories as manipulation-prone.**
    Pump-and-dump [82] + wash volume [19] concentrate in thin coins — the crypto
    microcap p-hacking trap [95]; an "edge" on a thin coin may be a manufactured-
    pump artifact the bootstrap should resample away ([36] outlier sin). [82][19][95]
43. **Expect decay on any crowned edge — budget for it.** Refereed predictors lose
    **~10% OOS from data-mining (statistically insignificant) and a further ~35%
    post-publication from crowding** [94] (corrected figures), and **64–85% of
    anomalies don't replicate** [95]; report a realistic performance-degradation
    haircut (PBO gives one [54]) and keep re-validating the forward paper-trade vs
    B&H [15]. Note our bake-off selects the *max of many* configs (unlike a single
    refereed hypothesis), so expect *more* than the 10% statistical floor — the
    σ√(2 ln K) selection-inflation result [80] is the reason why. [94][95][54][80]
44. **If we ever build labels/ML, prefer triple-barrier + fractional differentiation,
    avoid future-turning-point labels, and account for regime-dependent label noise.**
    Trend labels from full-period peaks/troughs leak [66]; label reliability drops in
    volatile regimes [70]; vol-scaled triple-barrier + frac-diff is the crypto-proven
    stack [74] — but still subject to our robustness gate + DSR/PBO. [74][66][70]

## Open questions / things worth testing in our app

- Does our verdict change under stationary vs fixed-length blocks? (sensitivity) [1]
- What block length does our gate use, and is it justified by the data's
  autocorrelation decay (Politis-White plug-in)? [1][18]
- Do any of our indicators secretly use a window that peeks at the current bar's
  close before the fill bar? (leakage audit) [2][20]
- What is the Deflated Sharpe / MTRL of our crowned configs once we plug in the
  true trial count? Do any survive? [25][8]
- Does our cost model silently mis-scale or zero out under any config? Is there a
  known-input regression test for it? [28][9]
- What is the PBO (via CSCV) of our bake-off — how often does the in-sample winner
  fall below median out-of-sample? Could it become an overfit-rejection gate? [54][27]
- If we crown by a parameter-plateau instead of the single grid peak, does the
  crowned config change, and does it generalize better forward? [58]
- Which exchange/feed are we sourcing, and is its provenance stamped + its cost
  spec CEX-realistic (vs DEX/gas economics)? [55][19]
- Is the forward paper-trade running the EXACT crowned strategy (no SMA proxy)?
  (training–serving skew / F5) [39]
- Would surfacing a B&H+strategy blend (lower drawdown) be a better advisor output
  than a single crowned active strategy? [58]
- Does our outlier handling ever silently winsorize a real crash bar? Is cleaning
  a documented, loose, robust (median/MAD) filter rather than mean/SD? [64][37]
- Could a small bootstrap-derived (NOT GAN) "crash/jump" stress slice strengthen
  the robustness story without understating tails? [4][30]
- Is there leakage between the param-tuning window and the ranking/forward window
  in the advisor-param-tuning flow? [29]
- Would a worst-path / CVaR statistic across the 1000 bootstrap paths better match
  the weakest-link verdict we already claim? [24]
- Is our gate's block length consistent with Künsch's l→∞, l/n→0 condition for our
  typical window lengths, and is the verdict stable across nearby lengths? [84][89]
- Should the block length used for *drawdown/CVaR* CIs differ from the one used for
  Sharpe-style means (quantile vs variance targets)? [88]
- Which of DSR / PBO / Harvey-Liu haircut / complexity-penalty is cheapest to wire
  into the existing bake-off ranking, and do they agree on deflating our winners? [73][54][25][98]
- Does our engine make an explicit, conservative intra-bar fill assumption for
  stop/take exits, and is it covered by a test? [81]
- Is the advisor's coin-selection scope restricted to liquid coins, and do we warn
  when a chosen coin's history shows pump/wash artifacts? [82][19]
- What performance-degradation haircut should we show the operator, given OOS-decay
  evidence (~10% statistical / ~35% post-publication [94], corrected) and our own
  PBO [54]? And is our forward-paper-trade-as-holdout sufficient *alone*, or must it
  be paired with a CSCV/PBO on the bake-off matrix (single hold-out is high-variance
  + trial-count-blind per [54])? [94][54]
- If we add ML/labels: are we using triple-barrier (not future-turning-point trend
  labels), frac-diff features, and accounting for regime-dependent label noise? [74][66][70]

## Paper map (claim → supporting [N])

- Block-bootstrap family + block-length is the key knob → [1]
- Leakage beyond the split; centered windows & exec-alignment dominate → [2]
- Naive CV manufactures illusory R² on financial/crypto data; purge+embargo → [3]
- GANs miss tails/jumps/cross-asset structure; unstable training → [4][5]
- GANs reproduce vol clustering/leverage (the easy facts) → [5]
- Synthetic-data validation is subjective; learned summaries help; ABMs out of scope → [6]
- Survivorship/universe bias quantified; reappears as coin-selection + delisting → [7]
- Multiple testing manufactures winners; trial count ↔ min backtest length → [8]
- Technical rules beat B&H gross, die net of costs (same bootstrap machinery as us) → [9]
- Triple-barrier path-aware labeling > fixed-horizon; edge still thin → [10]
- Diffusion > GAN for synthetic series, but no model nails all stylized facts → [11]
- Generic TS augmentation injects artifacts; only block/permutation defensible → [12]
- Dependence = temporal positions of large moves (block bootstrap preserves) → [13]
- Overfitting-aware objective (benchmark-significance gate) > Sharpe-max → [14]
- AI/ML crowding causes alpha decay; commoditized signals decay fastest → [15]
- Effective sample size ≪ row count; non-stationarity is core ML obstacle → [16]
- Too-good sentiment backtest hides missing cost/leakage/MT controls → [17]
- Optimal block length set from correlogram (Politis-White plug-in) → [18]
- >70% of unregulated crypto volume is wash-traded; volume signals suspect → [19]
- Look-ahead bias measurable (LLMs −15 to −22pp alpha decay; classical MA/momentum decay far less); paired matched-B&H windows = a leakage test → [20]
- CPCV = modern standard of evidence (multi-path OOS distribution, PBO/DSR) → [21][3]
- Augmentation helps only when signal exists; encode priors not just data → [22]
- 7,846 rules: in-sample winner fails OOS after snooping correction → [23]
- Robustness = worst-adverse-subperiod, not average (weakest-link validated) → [24]
- Deflated Sharpe + PSR + MTRL: implementable multiple-testing/non-normality fix → [25]
- Selecting best-of-many noisy trials rewards luck; fair test is prospective → [26]
- Overfitting as a rejectable hypothesis-test / pre-deploy filter (crypto) → [27]
- Cost-model implementation is the dominant source of backtest divergence → [28]
- IS/WFA/OOS must be strictly non-overlapping; true untouched holdout → [29]
- Even careful GANs underestimate tails & miss breaks on volatile assets → [30]
- LOB/tick out of scope; metric≠tradable-edge; judge by net P&L vs B&H → [31]
- Deep generative models vs parametric; validate on known-truth first → [32]
- Copulas separate marginals (tails) from serial+cross dependence → [33]
- ABIDES agent-based simulator = mechanistic synthetic-data branch, out of scope → [34]
- DL imputation survey; imputation is a leakage/uncertainty surface → [35]
- Seven Sins of quant investing checklist (incl. outlier sin) → [36]
- Two kinds of outlier: data-error (remove) vs real extreme (keep) → [37]
- Time bars statistically inferior vs information-driven bars; we use daily → [38]
- Feature stores: PIT-correct retrieval + training–serving skew (= our F5) → [39]
- Overlapping-label concurrency; uniqueness weights + sequential bootstrap → [40]
- Fractional differentiation: stationarity-vs-memory; crypto trend lives in level → [41]
- Cont's stylized facts: 8/11 hold, asset-specific not universal → [42]
- Bitcoin stylized facts strong but Hurst 0.42→0.49 (efficiency rising) → [43]
- Crypto vol: inverse leverage, lower persistence, jumps dominate → [44]
- BOCPD: online/causal regime detection as a leakage-free diagnostic → [45][46]
- Stationary bootstrap (geometric block length) = our sensitivity-check cousin → [47]
- de Prado's 10 reasons ML funds fail (mostly data discipline) → [48]
- RCGAN + TSTR (train-on-synthetic-test-on-real) acceptance test → [49]
- Synthetic-data-for-risk: VAE smooths extremes; evaluate utility+robustness → [50]
- Adversarial/stress robustness is regime-conditional (calm≠stress) → [51]
- Leakage drives the ML reproducibility crisis; model info sheets → [52]
- Financial labeling: triple-barrier + meta-labeling (precision/recall split) → [53]
- PBO via CSCV: IS-best below OOS median; computable on our bake-off matrix; AND dismantles single hold-out (high-variance + trial-blind) ⇒ forward paper-trade alone insufficient → [54]
- CEX > DEX price-data quality; source/stamp a reputable CEX feed → [55]
- RMT/Marčenko-Pastur denoising: noise-vs-signal null (covariance, out of scope) → [56]
- Rigorous daily-bar signals net ~0%, regime-dependent; honest-null reporting → [57]
- Parameter smoothing (stable plateau > single peak); double-OOS; ON CRYPTO OOS only matches B&H → [58]
- Augmentation is a small-data crutch; causal transforms only; Reverse hurts → [59]
- GAN augmentation helps ML forecasting on scarce crypto data (MSE≠edge) → [60]
- Synthetic-data evaluation battery + generation "trilemma"; VAE smooths extremes → [61]
- Derive rule params from process (OU optimal-stopping); misfits crypto → [62]
- DFDR multiple-testing correction; some TA survives but regime/turnover-bound → [63]
- Field-standard robust (median/MAD) loose outlier cleaning checklist → [64]
- Diffusion-TS interpretable generator = least-bad if forced off real data → [65]
- Continuous trend labeling boosts accuracy but future-turning-points leak → [66]
- Look-ahead bias can live in a pretrained LLM's parameters (hard to remove) → [67]
- Look-ahead benchmark bias (constituent reconstitution) inflates Sharpe ~8%/yr → [68]
- Crypto lower-tail dependence (co-crash) rising over time; copula tail asymmetry → [69]
- Label noise is non-stationary — worst in volatile regimes → [70]
- RL-agent crypto ABM (mechanistic generator, regime-spanning), out of scope → [71]
- Tail/rare-event synthesis: Gaussian latent prior CAN'T make heavy tails (root cause); needs EVT-augmented, crisis-metric-evaluated generators; crypto-crash synthesis still open → [72]
- Harvey-Liu Sharpe haircut: non-linear in magnitude, grows with trial count → [73]
- Frac-diff + triple-barrier + SAE on crypto: good only at tuned sweet-spot → [74]
- Finite-sample bias contaminates Bitcoin volatility/Hurst estimates → [75]
- TimeGAN: temporal fidelity needs supervised/embedding loss (the baseline) → [76]
- Bitcoin LSTM beats B&H on ONE 2024 year, no snooping correction (counterpoint) → [77]
- 7,000+ Chinese-market rules: SPA-corrected, regime/efficiency-dependent edge → [78]
- CPCV best, Walk-Forward worst by PBO/DSR in controlled environment → [79]
- Selection/tuning leakage ~40× preprocessing; inflation = σ√(2 ln K), ~90% noise at small n → [80]
- Backtest-engine correctness hinges on intra-bar fill assumptions → [81]
- Crypto pump-and-dump frequent in thin coins; price-integrity risk → [82]
- Crypto intraday/weekly seasonality (funding-clocked); daily bars aggregate it → [83]
- Künsch (1989): THE moving-block bootstrap; l→∞, l/n→0; MBB variance = spectral density at zero = long-run variance (⇒ too-short blocks under-estimate it → over-narrow CIs) → [84]
- Conditional Sig-WGAN: signature path-summary generator branch → [85]
- Time-Causal VAE: causal-Wasserstein bound, but constant C=2(2^T−1) blows up in path length (near-vacuous for long windows) → [86]
- For VaR, Historical Simulation, GARCH & CWGAN top; but HS/bootstrap capped by worst historical scenario (can't invent worse crash) → [87]
- Optimal block scheme differs for QUANTILE (drawdown/CVaR) targets → [88]
- Block-length selectors converge slowly; "optimal" can't be pinned precisely → [89]
- Cont (2001): canonical stylized-fact checklist; why block bootstrap is right → [90]
- SOCK feature-matching generator targets single-path/small-sample overfitting → [91]
- Deep-Hedging market simulator: why people build generators (= why we don't) → [92]
- Bitcoin ≥4 bubble/crash episodes vs Metcalfe fundamental; tails structural → [93]
- Academic research destroys predictability: ~10% OOS (data-mining, insignificant) / ~35% post-publication decay [corrected] → [94]
- Replicating Anomalies: 64–85% don't survive microcap-robust multiple testing → [95]
- Evaluating Trading Strategies: real-time haircut (Bonferroni/Holm/BHY) → [96]
- Oxford-Man DL benchmark: rigorous (CVaR/breakeven-cost/seed); edge thins net → [97]
- Covariance/complexity penalty deflates backtest by parameters × data → [98]
- Adaptive long-biased multi-coin crypto trend "beats B&H" (counterpoint) → [99]
- Stationary bootstrap gives stable error from ONE correlated series (physics) → [100]

## Topic-level verdict (for our advisor)

Across 100 papers the message is consistent and reinforces our product thesis:
1. **Real-data moving-block bootstrap is the right default** for our robustness
   gate — now grounded in its foundational proof (Künsch 1989 [84]; l→∞, l/n→0)
   and re-confirmed for risk by the VaR review where Historical Simulation/GARCH
   tie-or-beat deep generators [87]. It preserves the tails and volatility-cluster
   *positions* [13][90] that matter, can't fabricate dynamics [4][30][91], and is
   reproducible. Set its block length from the data's correlogram [18], note the
   "optimal" is slowly-estimated and quantile-target-dependent [89][88], and
   **sensitivity-check** across nearby lengths + MBB↔stationary [47][100].
2. **Synthetic generators (GAN/diffusion/signature/VAE/copula/ABM) are research-
   only** for us — the now-complete generator map [76][85][86][87][91][92][69][71]
   confirms they understate exactly the crash risk crypto has [4][11][30][50][72],
   overfit a single short path [91], and add nothing without signal [22]. Tail
   fidelity is a separate, harder problem [72]; if ever scoped, demand a causal
   constraint [86] + utility/TSTR validation + explicit tail check.
3. **The honest gauntlet is leakage-audit → cost/engine-fidelity → multiple-testing
   deflation → OOS forward check.** Concretely: leakage checklist [2][20] with the
   selection/tuning boundary as the #1 gate [80]; audited cost model [28][9] +
   intra-bar fill assumption [81]; a multiple-testing-adjusted number next to B&H
   via Deflated-Sharpe/MTRL [25], PBO-via-CSCV [54], or the Harvey-Liu haircut
   [73][96] (CPCV-best/Walk-Forward-worst [79]); strict IS/WFA/OOS separation [29];
   and a protected forward paper-trade [23][26]. **Corrective sharpening from the
   full-text read of the PBO paper [54]:** the forward paper-trade is *necessary*
   but **not sufficient on its own** — a single hold-out is high-variance and
   *blind to the trial count* (Bailey et al.'s reasons (iv)+(v)); it must be
   **paired** with a trial-aware multi-path number (CSCV/PBO on the bake-off matrix
   + DSR/MinBTL on the crown). This is the same "single path is the weakest
   evaluation" verdict that [79] (Walk-Forward worst) and [84] (resample, don't
   trust one path) reach from other directions — and it is the one place the new
   reading actually *amends* our design rather than just endorsing it.
4. **Any crowned active edge is fragile and time-decaying** [15][26], now with
   *corrected* decay numbers from full-text — McLean-Pontiff [94] is **~10% OOS
   from data-mining (statistically insignificant) + ~35% post-publication from
   crowding** (NOT the "26%/58%" previously logged), and **64–85% of anomalies
   don't replicate** [95]. Critically, that ~10% statistical floor is for *single
   refereed* hypotheses; our bake-off selects the *max of many* configs, and the
   selection-inflation decomposition **σ√(2 ln K), of which ~90% is pure noise at
   small n** [80], says our crown is inflated *more* — and *worse* on a small
   single-coin sample (inflation ∝ √(ln K) but ↓ only as 1/√n). Three recent crypto
   "beats-B&H" results [77][74][99] all fail our standards (one window, no
   MT-correction, long-bias/selection). The realistic, well-supported expectation
   remains: **no active strategy robustly beats buy-and-hold net of costs**
   [9][23][58].

*Round-3 (66–100) added no paper that overturns this; the strongest new "beats-
B&H" claims are all single-window, selection-biased, or uncorrected for multiple
testing — i.e. exactly what the bootstrap + DSR/PBO gate exists to discount. The
new methods papers (Künsch [84], Cont [90], PBO/CPCV comparison [79], the haircut
family [73][96][98]) instead **strengthen the case for our gate and for adding
Deflated-Sharpe/PBO to it.***

*Deep-read pass (depth, not breadth — no new papers): full-text reads of 12 high-
value entries [20][41][53][54][72][80][84][86][87][90][94][95] (plus a paywall-
noted enrichment of [79]) sharpened the gate-data rationale and produced one
correction and one design amendment.* **Correction:** McLean-Pontiff [94] decay is
**~10% statistical (insignificant) + ~35% post-publication**, not the "26%/58%"
previously logged (fixed throughout). **Amendment:** the PBO paper's [54] own
hold-out critique shows our **forward paper-trade alone is insufficient** (single
hold-out = high-variance + trial-count-blind) and must be paired with a CSCV/PBO +
DSR/MinBTL on the bake-off matrix. **Three reinforcements with new full-text
evidence:** (1) selection inflation is **~40× preprocessing leakage and decomposes
as σ√(2 ln K) — ~90% pure noise at small n** [80], the direct data-side proof that
our large-K-small-n bake-off needs trial-count deflation (DSR/MinBTL); (2) the MBB
variance estimate *is* the **spectral density at zero = long-run variance** [84], so
a too-short block under-estimates it and yields over-narrow CIs — turning "l→∞,
l/n→0" into a concrete sensitivity-check imperative; (3) generators understate tails
for a **structural** reason — a **Gaussian latent prior cannot produce heavy tails**
[72] — while the one honest limit of *our* method (HS/bootstrap can't invent a
worse-than-seen crash [87]) is real and best filled by a tail-stressed slice or EVT
generator, not a generic GAN/VAE.*
