# Knowledge — Deep Learning & Reinforcement Learning for Trading

> Status: batch 2 complete — **54 papers** logged ([1]–[54]). See `papers.md` for the ledger.
> Our app context: Rust single-coin crypto **paper advisor** — bake off strategies on (coin, window) → rank under a FROZEN 1000-path moving-block-bootstrap robustness gate (weakest-link verdict; **buy-and-hold is always the benchmark**) → forward paper-trade the simulated budget. Validated thesis: **no active strategy robustly beats holding, net of costs.**

---

## Bottom line for the operator (read this first)

**No deep-learning or RL method in this literature is realistically deployable as the alpha engine of a single-coin paper advisor.** Three independent reasons:
1. **Where DL/RL genuinely adds value, it is cross-sectional and tiny.** The most rigorous study ([24] Gu-Kelly-Xiu, RFS) shows ML's real edge comes from nonlinear interactions across *many features and many assets*, and even then absolute out-of-sample R² is sub-1% per month. A single coin with a handful of features is the opposite of that setting.
2. **The forecasting edge of deep models over trivial baselines is small or nonexistent.** [8] (DLinear, AAAI-Oral) shows one-layer linear models beat Informer/Autoformer; [22] (PatchTST) only recovers a single-digit-% forecasting-error edge after careful design — and on physics/utility data, never cost-net trading P&L.
3. **The spectacular crypto-trading numbers do not survive scrutiny.** [10] 4x/50d (2017 bull + survivorship), [21] 6654%/yr (implausible 82% accuracy, leakage-smelling, costs not credible), [23] Sharpe 2.7 (30-day out-of-sample, no buy-and-hold compare), [28] +176% crypto SAC (no buy-and-hold benchmark). The field's own people say so: [12] (no consistent baseline-beating standard), [18] (DRL profits are "false positives due to overfitting"), [20] (FinRL-Meta names low SNR, survivorship bias, backtest overfitting as the defining problems), [25] (2024 RL survey: still unresolved), [27] (offline RL policy "is not trustworthy" — memorizes one path). **And the strongest single corroboration: [42] — a rigorously-built SAC portfolio with real costs, 16 OOS folds over 2003–2026, three markets, and HAC-robust inference finds "no strategy achieves statistically significant excess returns relative to Buy & Hold." That is our validated thesis, reproduced by RL proponents.**

**Where DL IS worth our time: generative models for SYNTHETIC TEST DATA, not for alpha.** [9] QuantGAN, [16] diffusion, [19] TimeGAN, [38] time-causal VAE, [26] neural SDEs, [31] agent-based MARL sim can produce synthetic price paths that reproduce stylized facts (fat tails, vol clustering) and could enrich our moving-block bootstrap with novel-but-plausible regimes. This is the single actionable DL lane for us — BUT [41] warns stylized-fact fidelity is fragile and architecture-dependent, so validate hard. (Details below.)

**Second actionable lane: borrow the FALSIFICATION / leakage-audit discipline.** [39] (run the pipeline on structural-null data and confirm it crowns nothing), [37] (8-type leakage taxonomy + model-info-sheet checklist), [34] (validate each input independently, not by end-to-end accuracy). These are concrete tests/checklists we can add, not models.

---

## Key themes

1. **Two paradigms.** (a) *Predict-then-trade* — forecast price/return/direction with LSTM/CNN/Transformer, then act ([3][4][5][6][7][8][11][17][21][22][24][33][40][51][52]). (b) *Learn-the-decision directly* — RL or deep hedging optimize a trading/hedging policy end-to-end against a P&L or risk objective ([1][2][10][13][15][18][23][27][28][29][36][42][43][45][48][49]). Paradigm (b) is more principled (no forecast→action gap) and has the best transferable ideas, but is data-hungry and overfits one regime.
2. **Generative models are a distinct, more promising lane FOR US** — [9][14][16][19][26][31][38][41][43][44][46][53][54] generate synthetic market data; relevant to test-data discipline, not alpha. Batch-2 deepened this: non-adversarial generators (diffusion [16][53], time-causal VAE [38], signature-MMD [46]) are now preferred over GANs for stability, and [54] gives the full validation protocol incl. leakage tests.
3. **The skeptical/meta/reproducibility literature is strong, consistent, and largely from insiders** — [8][12][18][20][24][25][27][34][37][39][47][50] all converge on: trivial baselines are hard to beat, costs/leakage/survivorship/seeds are routinely mishandled, and there is no reproducible standard. Batch-2 added the heavy hitters: [47] (RL results flip on the random seed alone), [37]+[39] (leakage manufactures fake alpha; falsify against structural nulls), [34] (don't validate a feature by end-to-end accuracy).
4. **Costs and out-of-sample length are where headline results die** — papers that model costs honestly and use long out-of-sample windows ([15][24][42][52]) report small-or-zero edges; papers with astronomical returns ([21][23][10][28]) use short windows, no buy-and-hold compare, and/or undercount costs. **The decisive batch-2 evidence: [42] (SAC, 16 folds, HAC-robust) and [52] (hourly BTC, 27-fold walk-forward, bootstrap-adjusted) BOTH find no statistically significant outperformance vs buy-and-hold — our exact thesis, reproduced rigorously, [52] on our exact asset class.**
5. **Costs push the optimal policy toward INACTION** — independently across deep hedging ([15][36]) and crypto ML ([52]): with realistic costs the optimal action is to trade less / hold, and a cost-proportional trade filter is the difference between viable and ruinous. This is the structural reason buy-and-hold is so hard to beat.

## Methods / findings that HOLD UP (and which don't)

**Credible / replicated:**
- **Simplicity wins in forecasting** [8]: one-layer linear (DLinear/NLinear) ≥ Informer/Autoformer across 9 datasets; self-attention is permutation-invariant → loses temporal order. [22] only partially rehabilitates transformers (small margin, careful design).
- **ML's real asset-pricing edge is cross-sectional, nonlinear, and tiny in R²** [24][32]: doubling a small Sharpe via momentum/liquidity/volatility interactions across thousands of stocks; autoencoder factor models [32] win OOS only on the cross-section. Rigorous — and irrelevant to a single coin (no cross-section).
- **Generative models reproduce stylized facts** [9][16][19][38][46][54]; diffusion [16][53] / VAE [38] / signature-MMD [46] are more stable than GANs (no mode collapse, no adversarial instability); TimeGAN [19] + [54] contribute the standard validation metrics (discriminative score, TSTR, MMD, AND leakage tests). BUT [41] proves GAN stylized-fact fidelity is fragile and architecture-dependent — validate per-fact, never assume.
- **Direct risk/Sharpe-objective policy learning is sound** [2][15][36][45]: deep hedging minimizes a convex risk measure; Deep Momentum Networks optimize a Sharpe loss; [36] tracks mean+variance of cost via two Q-functions; [45] uses a Dirichlet policy for valid weights. All "learn the decision" done right. [15] honestly shows the edge dies at ~2–3 bps cost; [36] shows costs → trade less.
- **Probabilistic/quantile + interpretable forecasting** [5] TFT: the honest way to forecast (distribution + variable importance).
- **Overfitting/leakage is testable** [18][37][39][47]: PBO/CSCV [18] gives an overfitting probability; [37] an 8-type leakage taxonomy + model-info-sheet; [39] falsification against structural nulls + a Backtest Inflation Factor; [47] shows single-seed RL results are noise → demand many seeds + significance.
- **SAC > DDPG for stability** [28][42]: entropy regularization + twin critics make SAC/TD3 far more stable than vanilla DDPG in noisy/volatile markets — the consistent algorithm-choice lesson if continuous-action RL is ever used.

**Overstated / do NOT hold up:**
- **DRL/DL "beats the market" crypto returns** [10][21][23][28]: bull-market drift, survivorship/selection, leakage, short single-path windows, missing buy-and-hold benchmark. Not reproducible. Directly contradicted by rigorous [42][52] (no significant edge vs buy-and-hold).
- **Directional accuracy → profit leap** [3][4][11][21][33][40]: high classification accuracy reported without cost-net P&L or tradability. Accuracy ≠ alpha — [40] proves it (F1 collapses once the threshold = the spread); [52] proves it (positive gross → −80% net at 10 bps).
- **"Complex transformer = better time-series modeling"** as a general claim [6][7]: refuted by [8].
- **Single-window Sharpe as evidence of robustness** [13][23][28]: a Sharpe of 1–2.7 on one ~1-month-to-3-year path has enormous error bars — [47] shows even the random seed can produce it.
- **Offline RL trained on one history** [27]: "your offline policy is not trustworthy" — it memorizes the single historical path and fails under non-stationarity.
- **GNN relational-graph edge** [11][33]: may rest on graphs that aren't actually informative — [34] shows the literature validates graphs only by downstream accuracy, conflating graph quality with model quality.
- **Models fail exactly in crises** [51]: learned vol forecasters degrade most in high-vol regimes (MAPE >16% in 2008) — worst when it matters most.

## Actionable takeaways for our advisor

1. **Keep the always-benchmark-vs-buy-and-hold, cost-net, bootstrap gate. It is precisely what this literature lacks — and the rigorous papers that DO use it reach our exact conclusion.** [8][10][12][18][20][21][23] show that without a trivial baseline + cost discipline + multi-path validation, DL/RL "wins" are illusions; and the two most rigorous, cost-aware, bootstrap/HAC-tested studies [42][52] independently find NO significant edge over buy-and-hold ([52] on hourly BTC). Our gate is a competitive advantage and is externally vindicated.
2. **Highest-value DL experiment for us = a generative simulator for TEST DATA — now with a clear recipe.** Prototype a NON-adversarial generator — diffusion [16][53], time-causal VAE [38], or signature-kernel MMD [46] (all preferred over GANs [9][41] for stability) — fit to our coin to augment the moving-block bootstrap with novel stylized-fact-preserving regimes, including a **conditional STRESS/crash knob** [43] to generate worse-than-observed tail scenarios the bootstrap can't. Validate with the full [54] suite: distributional+temporal fidelity (MMD/KS, vol-clustering autocorr), TAIL-event fidelity (VAEs over-smooth tails — a dealbreaker), downstream utility (TSTR [19]), and LEAKAGE (nearest-neighbor + membership-inference [54]) to enforce the no-test-path-leak guardrail.
3. **NEW high-value, low-cost test: falsify our own pipeline against structural nulls** [39]. Run the entire bake-off + ranking on synthetic NULL data (white noise, GARCH with zero drift edge, bid-ask bounce) and assert it crowns NOTHING above buy-and-hold. If any "winner" emerges from pure noise, we have a leak or selection bug. Pair with a leakage-audit checklist (a "model info sheet" [37]) for any learned component. This is a test/CI item, not a model.
4. **If we ever add a learned signal, the rules are fixed by this literature:** (a) benchmark against a DLinear-style linear baseline AND buy-and-hold BEFORE anything else [8][22]; (b) forecast a DISTRIBUTION, not a point [5]; (c) if learning a policy, optimize a risk/Sharpe objective directly, add turnover regularization, use a Dirichlet parametrization for valid sizing [2][15][36][45]; (d) report a cost-sensitivity curve and expect the edge to vanish by a few bps [15][52]; (e) report across many SEEDS + bootstrap paths with a significance measure [42][47] — a single-path number is noise. Do not reach for a Transformer.
5. **Borrow the overfitting/leakage diagnostics, not the models.** Consider a PBO/CSCV [18] overfitting probability and/or a Backtest-Inflation-Factor [39] as complementary readouts beside our weakest-link verdict. Adopt the survivorship-bias warning [20] if we expand beyond one coin.
6. **Treat all DRL/EIIE/DQN/SAC/DDPG trading machinery as multi-asset-only and unproven** [1][10][13][23][28][42][45]. Irrelevant to a single coin; not worth the overfitting surface. Reusable *sub-ideas* only: validation-Sharpe agent-selection + turbulence risk-off [13], turnover regularization [15], two-Q mean-variance cost objective [36], Dirichlet sizing [45], SAC-over-DDPG if forced to choose [28].
7. **Test a cost-proportional trade filter as a strategy primitive** [52]: only act when the expected move exceeds a multiple of round-trip cost. This is the single mechanism that turned ruinous crypto ML strategies viable — a concrete, cheap thing to add to the bake-off (and likely STILL won't beat buy-and-hold, which is the honest expected result).

## Open questions / things worth testing in our app

- **Synthetic-data prototype (TOP candidate):** Can a diffusion [16][53] / time-causal VAE [38] / signature-MMD [46] generator fit to our coin produce bootstrap-complementary paths that the gate treats as a *harder* test (more stress, novel regimes)? Add a conditional stress/crash intensity [43]. Validate with the full [54] suite incl. tail-fidelity + leakage. 
- **Null-data falsification (cheap, high value):** Does our bake-off+ranking pipeline correctly crown NOTHING on white-noise / zero-edge-GARCH / bid-ask-bounce nulls [39]? If not, find the leak. A direct, implementable CI test.
- **Cost-proportional trade filter:** Does a "trade only if expected move > k·cost" gate [52] change any verdict in our bake-off? Hypothesis: it reduces turnover but still doesn't beat buy-and-hold.
- **Linear-baseline sanity check:** Does a DLinear-style trend/seasonal+linear forecaster [8] beat buy-and-hold net of costs under the frozen gate? Hypothesis: no — but it's the cheap minimal "is there ANY time-series signal" probe.
- **Risk-measure sizing:** Could a deep-hedging-style convex-risk objective [2][36] or Sharpe-loss-with-turnover-penalty [15] + Dirichlet output [45] inform a *position-sizing* policy (how much of the €200, when to de-risk) rather than a directional signal? Expect the cost curve to bite.
- **Overfitting metric:** Is PBO/CSCV [18] or a Backtest-Inflation-Factor [39] worth wiring in as a second opinion alongside the bootstrap weakest-link verdict?

## Paper map (claim → supporting [N])

- Simple linear models beat / match deep Transformers for forecasting → [8] (refuting [6][7]); only partially rehabilitated by [22]
- ML's genuine edge is cross-sectional, nonlinear, tiny absolute R² (irrelevant to one coin) → [24][32]
- DL "market-beating" crypto returns are bull-market/selection/leakage/short-window artifacts → [10][21][23][28]
- **Rigorous, cost-aware, multi-fold/bootstrap studies find NO significant edge over buy-and-hold (our thesis, reproduced)** → [42] (SAC, HAC-robust, equities) and [52] (hourly BTC, walk-forward, bootstrap) — strongest external validation
- The field's own experts: no reproducible baseline-beating standard; profits are overfit false positives; low SNR + survivorship + backtest overfitting are THE problems → [12][18][20][25][50]
- Accuracy / RMSE ≠ cost-net profit; tradability ignored → [3][4][11][21][33][40]; F1 collapses at spread-sized threshold → [40]; gross→net collapse at 10 bps → [52]
- Edge dies once realistic transaction costs are applied; costs push optimal policy toward inaction/holding → [15][36][52]
- Generative models reproduce financial stylized facts (synthetic TEST data) → [9][16][19][14][38][46][53][54]; NON-adversarial (diffusion/VAE/signature-MMD) more stable than GANs → [16][38][46][53]; GAN fidelity is fragile/architecture-dependent → [41]; standard validation metrics (discriminative/TSTR/MMD + leakage) → [19][54]
- Synthetic STRESS/crash scenario generation to harden the gate (beyond bootstrap) → [43] (conditional diffusion); neural-SDE / agent-based alternatives → [26][31]; reactive (market-impact) simulators → [44]
- Leakage manufactures fake alpha; falsify pipelines against structural nulls; quantify backtest inflation → [37][39]; validate each feature independently, not by end-to-end accuracy → [34]
- RL results are dominated by random-seed/hyperparameter variance → demand many seeds + significance → [47]; offline RL memorizes one path ("not trustworthy") → [27]
- Optimize a policy directly against a risk/Sharpe objective (decision, not forecast) → [2][15][36][45] (also [10][13][18])
- Forecast a distribution + keep it interpretable → [5]
- Trend/seasonal decomposition is a cheap, robust primitive → [7][8]
- Overfitting/leakage is testable (PBO/CSCV, BIF, leakage taxonomy, multi-seed) → [18][37][39][47]
- Reusable RL sub-ideas (validation-Sharpe selection, turbulence risk-off, turnover regularization, two-Q mean-variance, Dirichlet sizing, SAC>DDPG, cost-proportional trade filter) → [13][15][36][45][28][52]
- Models fail worst in crises / high-vol regimes → [51]
- Canonical DRL-for-trading scaffolding (MDP framing; env/agent/backtest) → [1][20]; cross-asset/LOB/market-making/execution deep models (background, multi-asset/HFT) → [3][11][14][17][29][30][33][40][49][51][53]
