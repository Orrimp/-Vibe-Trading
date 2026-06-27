# Knowledge — Deep Learning & Reinforcement Learning for Trading

> Status: batch 1 complete — **24 papers** logged ([1]–[24]). See `papers.md` for the ledger.
> Our app context: Rust single-coin crypto **paper advisor** — bake off strategies on (coin, window) → rank under a FROZEN 1000-path moving-block-bootstrap robustness gate (weakest-link verdict; **buy-and-hold is always the benchmark**) → forward paper-trade the simulated budget. Validated thesis: **no active strategy robustly beats holding, net of costs.**

---

## Bottom line for the operator (read this first)

**No deep-learning or RL method in this literature is realistically deployable as the alpha engine of a single-coin paper advisor.** Three independent reasons:
1. **Where DL/RL genuinely adds value, it is cross-sectional and tiny.** The most rigorous study ([24] Gu-Kelly-Xiu, RFS) shows ML's real edge comes from nonlinear interactions across *many features and many assets*, and even then absolute out-of-sample R² is sub-1% per month. A single coin with a handful of features is the opposite of that setting.
2. **The forecasting edge of deep models over trivial baselines is small or nonexistent.** [8] (DLinear, AAAI-Oral) shows one-layer linear models beat Informer/Autoformer; [22] (PatchTST) only recovers a single-digit-% forecasting-error edge after careful design — and on physics/utility data, never cost-net trading P&L.
3. **The spectacular crypto-trading numbers do not survive scrutiny.** [10] 4x/50d (2017 bull + survivorship), [21] 6654%/yr (implausible 82% accuracy, leakage-smelling, costs not credible), [23] Sharpe 2.7 (30-day out-of-sample, no buy-and-hold compare). The field's own people say so: [12] (no consistent baseline-beating standard), [18] (DRL profits are "false positives due to overfitting"), [20] (FinRL-Meta names low SNR, survivorship bias, backtest overfitting as the defining problems).

**Where DL IS worth our time: generative models for SYNTHETIC TEST DATA, not for alpha.** [9] QuantGAN, [16] diffusion, [19] TimeGAN can produce synthetic price paths that reproduce stylized facts (fat tails, vol clustering) and could enrich our moving-block bootstrap with novel-but-plausible regimes. This is the single actionable DL lane for us. (Details below.)

---

## Key themes

1. **Two paradigms.** (a) *Predict-then-trade* — forecast price/return/direction with LSTM/CNN/Transformer, then act ([3][4][5][6][7][8][11][17][21][22][24]). (b) *Learn-the-decision directly* — RL or deep hedging optimize a trading/hedging policy end-to-end against a P&L or risk objective ([1][2][10][13][15][18][23]). Paradigm (b) is more principled (no forecast→action gap) and has the best transferable ideas, but is data-hungry and overfits one regime.
2. **Generative models are a distinct, more promising lane FOR US** — [9][14][16][19] generate synthetic market data; relevant to test-data discipline, not alpha.
3. **The skeptical/meta literature is strong, consistent, and largely from insiders** — [8][12][18][20][24] all converge on: trivial baselines are hard to beat, costs/leakage/survivorship are routinely mishandled, and there is no reproducible standard.
4. **Costs and out-of-sample length are where headline results die** — papers that model costs honestly and use long out-of-sample windows ([15][24]) report small edges; papers with astronomical returns ([21][23][10]) use short windows, no buy-and-hold compare, and/or undercount costs.

## Methods / findings that HOLD UP (and which don't)

**Credible / replicated:**
- **Simplicity wins in forecasting** [8]: one-layer linear (DLinear/NLinear) ≥ Informer/Autoformer across 9 datasets; self-attention is permutation-invariant → loses temporal order. [22] only partially rehabilitates transformers (small margin, careful design).
- **ML's real asset-pricing edge is cross-sectional, nonlinear, and tiny in R²** [24]: doubling a small Sharpe via momentum/liquidity/volatility interactions across thousands of stocks. Rigorous.
- **Generative models reproduce stylized facts** [9][16][19]; diffusion [16] is more stable than GANs (no mode collapse); TimeGAN [19] contributes the standard validation metrics (discriminative score, TSTR/predictive score).
- **Direct risk/Sharpe-objective policy learning is sound** [2][15]: deep hedging minimizes a convex risk measure; Deep Momentum Networks optimize a Sharpe loss — both are "learn the decision" done right. [15] also honestly shows the edge dies at ~2–3 bps cost.
- **Probabilistic/quantile + interpretable forecasting** [5] TFT: the honest way to forecast (distribution + variable importance).
- **Overfitting is testable** [18]: PBO / combinatorially-symmetric cross-validation gives a probability-of-overfitting metric.

**Overstated / do NOT hold up:**
- **DRL/DL "beats the market" crypto returns** [10][21][23]: bull-market drift, survivorship/selection, leakage, short single-path windows, missing buy-and-hold benchmark. Not reproducible.
- **Directional accuracy → profit leap** [3][4][11][21]: high classification accuracy reported without cost-net P&L or tradability. Accuracy ≠ alpha.
- **"Complex transformer = better time-series modeling"** as a general claim [6][7]: refuted by [8].
- **Single-window Sharpe as evidence of robustness** [13][23]: a Sharpe of 1–2.7 on one ~1-month-to-3-year path has enormous error bars.

## Actionable takeaways for our advisor

1. **Keep the always-benchmark-vs-buy-and-hold, cost-net, bootstrap gate. It is precisely what this literature lacks.** [8][10][12][18][20][21][23] independently show that without a trivial baseline + cost discipline + multi-path validation, DL/RL "wins" are illusions. Our gate is a competitive advantage, not a limitation.
2. **Highest-value DL experiment for us = a generative simulator for TEST DATA.** Prototype a diffusion model [16] (preferred over GAN [9] for training stability) fit to our coin's history to augment the moving-block bootstrap with novel stylized-fact-preserving regimes. Validate with TimeGAN's [19] discriminative score (can a classifier separate synthetic from real?) and TSTR (train-on-synthetic/test-on-real). Hard guardrail: must reproduce OUR coin's stylized facts and must NOT leak the held-out test path.
3. **If we ever add a learned signal, the rules are fixed by this literature:** (a) benchmark against a DLinear-style linear baseline AND buy-and-hold BEFORE anything else [8][22]; (b) forecast a DISTRIBUTION, not a point [5]; (c) if learning a policy, optimize a risk/Sharpe objective directly and add turnover regularization [2][15]; (d) report a cost-sensitivity curve and expect the edge to vanish by a few bps [15] — crypto spot costs exceed that. Do not reach for a Transformer.
4. **Borrow the overfitting diagnostics, not the models.** Consider adding a PBO / CSCV [18] overfitting probability as a *complementary* readout next to our weakest-link verdict. Adopt the survivorship-bias warning [20] if we expand beyond one coin.
5. **Treat all DRL/EIIE/DQN trading machinery as multi-asset-only and unproven** [1][10][13][23]. Irrelevant to a single coin; not worth the overfitting surface. Reusable *sub-ideas* only: validation-Sharpe agent-selection and turbulence risk-off [13], turnover regularization [15].

## Open questions / things worth testing in our app

- **Synthetic-data prototype:** Can a diffusion [16] (or QuantGAN [9]) generator fit to our coin produce bootstrap-complementary paths that the robustness gate treats as a *harder* test (more stress, novel regimes)? Validate stylized-fact match + discriminative/TSTR scores [19]; prove no leakage. **(Top candidate next experiment.)**
- **Linear-baseline sanity check:** Does a DLinear-style trend/seasonal+linear forecaster [8] beat buy-and-hold net of costs on our coins under the frozen gate? Hypothesis: no — but it's the cheap, correct minimal experiment and a useful "is there ANY time-series signal here" probe.
- **Risk-measure sizing:** Could a deep-hedging-style convex-risk objective [2] or a Sharpe-loss with turnover penalty [15] inform a *position-sizing* policy (how much of the €200 to deploy / when to de-risk) rather than a directional signal? Even here, expect the cost curve to bite.
- **Overfitting metric:** Is PBO/CSCV [18] worth wiring in as a second opinion alongside the bootstrap weakest-link verdict?

## Paper map (claim → supporting [N])

- Simple linear models beat / match deep Transformers for forecasting → [8] (refuting [6][7]); only partially rehabilitated by [22]
- ML's genuine edge is cross-sectional, nonlinear, tiny absolute R² → [24]
- DL "market-beating" crypto returns are bull-market/selection/leakage/short-window artifacts → [10][21][23]
- The field's own experts: no reproducible baseline-beating standard; profits are overfit false positives; low SNR + survivorship + backtest overfitting are THE problems → [12][18][20]
- Accuracy / RMSE ≠ cost-net profit; tradability ignored → [3][4][11][21]
- Edge dies once realistic transaction costs are applied (cost-sensitivity curve) → [15]
- Generative models reproduce financial stylized facts (synthetic TEST data) → [9][16][19][14]; diffusion more stable than GANs → [16]; standard validation metrics (discriminative/TSTR) → [19]
- Optimize a policy directly against a risk/Sharpe objective (decision, not forecast) → [2][15] (also [10][13][18])
- Forecast a distribution + keep it interpretable → [5]
- Trend/seasonal decomposition is a cheap, robust primitive → [7][8]
- Overfitting is testable (PBO / CSCV) → [18]
- Reusable RL sub-ideas (validation-Sharpe selection, turbulence risk-off, turnover regularization) → [13][15]
- Canonical DRL-for-trading scaffolding (MDP framing; env/agent/backtest) → [1][20]; cross-asset/LOB deep models (background, multi-asset) → [3][11][14][17]
