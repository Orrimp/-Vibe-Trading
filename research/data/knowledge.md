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
- Could a small bootstrap-derived (NOT GAN) "crash/jump" stress slice strengthen
  the robustness story without understating tails? [4][30]
- Is there leakage between the param-tuning window and the ranking/forward window
  in the advisor-param-tuning flow? [29]
- Would a worst-path / CVaR statistic across the 1000 bootstrap paths better match
  the weakest-link verdict we already claim? [24]

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
- Look-ahead bias measurable; PIT discipline; data revisions are a subtle leak → [20]
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

## Topic-level verdict (for our advisor)

Across 30 papers the message is consistent and reinforces our product thesis:
1. **Real-data moving-block bootstrap is the right default** for our robustness
   gate — it preserves the tails and volatility-cluster *positions* [13] that
   matter, can't fabricate dynamics [4][30], and is reproducible. Set its block
   length from the data's correlogram [18], and consider tapered/stationary
   sensitivity [1].
2. **Synthetic generators (GAN/diffusion/ABM) are research-only** for us — they
   understate exactly the crash risk crypto has [4][11][30] and add nothing
   without signal [22].
3. **The honest gauntlet is leakage-audit → cost-fidelity → multiple-testing
   deflation → OOS forward check.** Concretely: leakage checklist [2][20],
   audited cost model [28][9], Deflated-Sharpe/MTRL on the winner [25][8],
   PBO/overfit filter [27][21], strict IS/WFA/OOS separation [29], and a
   protected forward paper-trade [23][26].
4. **Any crowned active edge is fragile and time-decaying** [15][26]; the
   realistic, well-supported expectation remains: **no active strategy robustly
   beats buy-and-hold net of costs** [9][23].
