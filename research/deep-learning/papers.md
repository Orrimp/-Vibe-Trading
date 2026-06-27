# Papers — Deep Learning & Reinforcement Learning for Trading

Ledger of papers read for this topic. One entry per paper, appended immediately
after reading. Source of truth for resume (skip listed titles, continue numbering).

### [1] FinRL: A Deep Reinforcement Learning Library for Automated Stock Trading in Quantitative Finance
- **Authors / Venue:** Xiao-Yang Liu, Hongyang Yang, Qian Chen, Runjia Zhang, Liuqing Yang, Bowen Xiao, Christina Dan Wang / Deep RL Workshop, NeurIPS 2020
- **Year:** 2020
- **Source:** arXiv:2011.09607
- **% read:** 50%
- **Summary:** Introduces the original FinRL open-source library — a three-layer (environment / agent / application) modular pipeline that turns stock-market datasets into Gym-style environments, trains DRL agents (DQN, DDPG, PPO, SAC, A2C, TD3), and backtests them with transaction costs, liquidity constraints and a risk-aversion (turbulence) control. Demonstrates single-stock, multi-stock and portfolio-allocation tasks across NASDAQ-100, S&P 500, DJIA and several Asian indices. Positioned as a reproducibility/education tool rather than a performance claim — it standardizes baselines and reward functions to lower the DRL learning curve.
- **Relevance to our system:** This is the canonical DRL-for-trading scaffolding everyone cites; its env/agent/backtest separation mirrors our bake-off → forward-plan split. Useful as a reference for *how* DRL practitioners frame the trading MDP (state = prices+indicators+holdings, action = target weights, reward = Δ portfolio value − costs) and for the "turbulence index" risk-off switch. Notably the paper makes NO claim that DRL beats buy-and-hold net of costs — consistent with our validated thesis. Background/reference; not directly deployable in a single-coin advisor without heavy adaptation and out-of-sample discipline.

### [2] Deep Hedging
- **Authors / Venue:** Hans Bühler, Lukas Gonon, Josef Teichmann, Ben Wood / Quantitative Finance 19(8):1271–1291
- **Year:** 2018 (arXiv) / 2019 (journal)
- **Source:** arXiv:1802.03042
- **% read:** 50%
- **Summary:** The foundational "deep hedging" paper. Frames hedging a derivatives portfolio under realistic frictions (transaction costs, market impact, liquidity, risk limits) as a Markov Decision Process and trains a neural-network policy to directly minimize a convex risk measure (OCE / entropic) of terminal P&L — bypassing any pricing model or Greeks. Proves the class of constrained NN strategies is rich enough to ε-approximate the optimal hedge. In synthetic Heston markets with costs it matches/beats the frictionless "complete market" delta and scales to high-dimensional portfolios; performance is driven by the set of available hedging instruments, not portfolio size.
- **Relevance to our system:** Background only for the hedging math, BUT the *methodological pattern* is highly relevant: optimize the policy directly against a risk-aware objective (a convex risk measure of P&L) rather than predicting prices then trading. That "learn the decision, not the forecast" framing and the use of a coherent/convex risk measure as the training loss is a transferable idea if we ever train a sizing policy. The reliance on a known synthetic generator (Heston) for training data also foreshadows our interest in market simulators/GANs for test data. Not deployable as-is (we hold spot, not options).

### [3] DeepLOB: Deep Convolutional Neural Networks for Limit Order Books
- **Authors / Venue:** Zihao Zhang, Stefan Zohren, Stephen Roberts / IEEE Transactions on Signal Processing 67(11):3001–3012
- **Year:** 2018 (arXiv) / 2019 (journal)
- **Source:** arXiv:1808.03668
- **% read:** 50%
- **Summary:** Predicts short-term mid-price direction (up/stationary/down) from raw limit-order-book snapshots. Architecture stacks convolutional filters that learn spatial structure across LOB price levels, an Inception module for multi-scale features, then an LSTM for temporal dependence. Trained/evaluated on the FI-2010 benchmark and one year of London Stock Exchange quotes; reports state-of-the-art out-of-sample classification accuracy beating SVM, MLP and prior CNNs, and — importantly — transfers to instruments NOT in the training set, suggesting it learns "universal" microstructure features.
- **Relevance to our system:** Mostly background — we run on OHLCV bars for a single spot coin, not full L2 order-book tape, so the architecture isn't directly usable. Two transferable lessons: (1) a model can learn *generalizable* microstructure features that transfer across instruments (relevant if we ever go multi-coin), and (2) the paper reports classification accuracy but does NOT convert the signal into a cost-aware P&L or address tradability — a recurring pattern in this literature that our buy-and-hold-benchmarked, cost-net robustness gate is specifically designed to expose. A high-accuracy directional classifier ≠ a profitable strategy.

### [4] A Survey of Forex and Stock Price Prediction Using Deep Learning
- **Authors / Venue:** Zexin Hu, Yiqi Zhao, Matloob Khushi / Applied System Innovation 4(1):9
- **Year:** 2021
- **Source:** arXiv:2103.09750
- **% read:** 35%
- **Summary:** Survey of DL price-prediction work across six families: CNN, LSTM, DNN, RNN, reinforcement learning, and "other" (HAN, NLP-driven, WaveNet). Finds the dominant trend is LSTM-hybrid models (LSTM+CNN, LSTM+DNN) and reports that RL and hybrids "yielded great returns." Catalogs the metric zoo used: RMSE, MAPE, MAE, MSE, accuracy, Sharpe ratio, return rate.
- **Relevance to our system:** Useful mainly as a map of the field and as evidence of a methodological problem we already guard against: the survey aggregates dozens of papers reporting glowing RMSE/return numbers but does NOT critically engage with overfitting, data-snooping, transaction costs, or reproducibility. The mismatched-metric soup (some report regression error, some report accuracy, some report Sharpe) makes cross-paper comparison meaningless — exactly why our app fixes ONE frozen robustness gate with buy-and-hold as the universal benchmark. Skeptical takeaway: treat headline numbers in this literature as upper bounds under data-snooping, not as deployable performance.

### [5] Temporal Fusion Transformers for Interpretable Multi-horizon Time Series Forecasting
- **Authors / Venue:** Bryan Lim, Sercan O. Arik, Nicolas Loeff, Tomas Pfister (Google Cloud AI) / International Journal of Forecasting 37(4):1748–1764
- **Year:** 2019 (arXiv) / 2021 (journal)
- **Source:** arXiv:1912.09363
- **% read:** 50%
- **Summary:** The Temporal Fusion Transformer (TFT) is an attention architecture for multi-horizon forecasting with heterogeneous inputs — static covariates, known-future inputs (e.g. calendar), and observed-only time series. Components: per-input variable-selection networks (which double as feature-importance), an LSTM encoder-decoder for local processing, gating (GRN) layers to skip unused components, and an interpretable multi-head attention layer for long-range dependence; it outputs quantiles (probabilistic forecasts). Reports significant gains over DeepAR/Seq2Seq/MQRNN on retail, electricity, traffic, and volatility benchmarks while remaining inspectable via attention + variable weights.
- **Relevance to our system:** Moderately relevant as a *forecasting* tool but with the same deployability caveat as all forecasters. Two genuinely useful ideas if we ever add a learned component: (1) quantile/probabilistic outputs (gives an honest uncertainty band rather than a point price — fits our risk-aware ethos), and (2) built-in interpretability (variable-selection weights) so a learned signal isn't a black box. BUT TFT needs many related series + rich covariates to shine; a single coin's OHLCV is data-poor for it, and the volatility benchmark in the paper is forecasting *realized vol*, not generating tradable, cost-net alpha. Treat as a "if we ever forecast, forecast a distribution and keep it interpretable" reference.

### [6] Informer: Beyond Efficient Transformer for Long Sequence Time-Series Forecasting
- **Authors / Venue:** Haoyi Zhou, Shanghang Zhang, Jieqi Peng, Shuai Zhang, Jianxin Li, Hui Xiong, Wancai Zhang / AAAI 2021 (Best Paper)
- **Year:** 2020 (arXiv) / 2021 (AAAI)
- **Source:** arXiv:2012.07436
- **% read:** 50%
- **Summary:** Tackles long-sequence time-series forecasting where vanilla Transformers are O(L²) in time/memory. Three innovations: (1) ProbSparse self-attention selecting only the dominant queries for O(L log L) cost; (2) self-attention distilling that halves layer length to handle very long inputs; (3) a generative-style decoder that emits the whole horizon in one forward pass (no slow autoregression, no error accumulation). Beats prior methods on ETT (electricity transformer temperature), ECL, and weather benchmarks for long horizons.
- **Relevance to our system:** Background. Informer is an efficiency story for *very long* horizons/sequences (hundreds–thousands of steps), which is the opposite of a single-coin advisor that decides over modest windows. The benchmarks are physical/utility series, not finance. The real signal for us comes from the *critique* of this whole transformer-forecasting line (see DLinear, next): on many series simple linear baselines match or beat these elaborate models, so importing Informer-class complexity into our advisor would add compute + overfitting surface for little expected edge.

### [7] Autoformer: Decomposition Transformers with Auto-Correlation for Long-Term Series Forecasting
- **Authors / Venue:** Haixu Wu, Jiehui Xu, Jianmin Wang, Mingsheng Long (Tsinghua) / NeurIPS 2021
- **Year:** 2021
- **Source:** arXiv:2106.13008
- **% read:** 50%
- **Summary:** Long-term forecasting Transformer with two ideas: (1) makes seasonal-trend *series decomposition* an inner block repeated through the network (progressively separating trend from seasonal) rather than a one-off preprocessing step; (2) replaces dot-product self-attention with an Auto-Correlation mechanism that uses FFT to find period-based sub-series dependencies and aggregate them, giving O(L log L) cost. Reports a 38% relative error reduction over prior baselines across six benchmarks spanning energy, traffic, economics, weather, and disease.
- **Relevance to our system:** Background as an architecture, but ONE idea is broadly useful and cheap: explicit trend/seasonal decomposition. Decomposing a price/return series into trend + seasonal + residual before any modeling is a classic, low-overfitting-risk preprocessing step that even simple strategies can exploit, and it's far lighter than the full Autoformer. The "economics" benchmark is the standard exchange-rate dataset, not crypto, and there's no cost-aware trading evaluation. Net: borrow the decomposition idea conceptually; skip the Auto-Correlation Transformer itself.

### [8] Are Transformers Effective for Time Series Forecasting?
- **Authors / Venue:** Ailing Zeng, Muxi Chen, Lei Zhang, Qiang Xu / AAAI 2023 (Oral)
- **Year:** 2022 (arXiv) / 2023 (AAAI)
- **Source:** arXiv:2205.13504
- **% read:** 75%
- **Summary:** The key skeptical counterpoint to the whole Transformer-forecasting line. Argues that self-attention is permutation-invariant, so it inherently loses the temporal *ordering* that time-series forecasting depends on (positional encodings only partly compensate). Proposes embarrassingly simple one-layer linear models — DLinear (trend/seasonal decomposition + a linear layer each) and NLinear (subtract-last normalization + linear) — as baselines. Across nine real-world datasets these linear models *outperform* Informer, Autoformer, FEDformer, etc., often by a large margin, implying the elaborate architectures' reported gains have "little to do with" Transformer temporal modeling.
- **Relevance to our system:** HIGHLY relevant as a guardrail. It is direct evidence that complex DL forecasters frequently fail to beat trivial linear baselines on real series — the forecasting analogue of our validated "no active strategy robustly beats buy-and-hold" thesis. The lesson for the advisor: any learned/DL component MUST be benchmarked against a dead-simple baseline (linear extrapolation, last-value, or just buy-and-hold) before it earns its place, and "we used a Transformer" is not evidence of edge. This justifies our frozen-gate + always-benchmark design and our reluctance to add DL forecasters. Strong support for staying simple.

### [9] Quant GANs: Deep Generation of Financial Time Series
- **Authors / Venue:** Magnus Wiese, Robert Knobloch, Ralf Korn, Peter Kretschmer / Quantitative Finance 20(9):1419–1440
- **Year:** 2019 (arXiv) / 2020 (journal)
- **Source:** arXiv:1907.06673
- **% read:** 50%
- **Summary:** A GAN for generating synthetic financial price paths. Both generator and discriminator are Temporal Convolutional Networks (TCNs, dilated causal convs) so the model captures long-range dependence like volatility clustering; it models log-returns and is constructed so the resulting process can be transformed toward a risk-neutral measure. The synthetic series reproduce the classic stylized facts — fat-tailed return distributions, volatility clusters, leverage effect, and slow decay of autocorrelation in squared/absolute returns — with distributional agreement across short and long lags. Evaluated by comparing stylized-fact statistics of real vs. generated paths.
- **Relevance to our system:** Directly relevant to our test-data discipline. This is a concrete recipe for generating realistic synthetic price paths to stress-test strategies and the robustness gate beyond the single historical path we actually observed. Compared to our current moving-block bootstrap (which only reshuffles real returns and can't produce unseen-but-plausible regimes), a QuantGAN-style generator could enrich the test set with novel volatility-clustered scenarios — IF we validate that it reproduces our coin's stylized facts and don't overfit the generator to the same data we test on. Caveats to respect: GANs can mode-collapse, may not capture true non-stationarity/regime shifts, and synthetic data that's too close to training data leaks. Flag: a strong candidate to augment (not replace) the bootstrap. See also diffusion-based generators as a more-stable alternative.

### [10] A Deep Reinforcement Learning Framework for the Financial Portfolio Management Problem
- **Authors / Venue:** Zhengyao Jiang, Dixing Xu, Jinjun Liang / arXiv preprint (widely cited)
- **Year:** 2017
- **Source:** arXiv:1706.10059
- **% read:** 50%
- **Summary:** The canonical model-free DRL crypto-portfolio paper. Introduces the EIIE (Ensemble of Identical Independent Evaluators) topology — a shared network scoring each asset independently — plus a Portfolio-Vector Memory (last weights as input, to handle costs) and Online Stochastic Batch Learning, trained to maximize log return. Tested on Poloniex crypto with 30-min bars and a stated 0.25% commission; reports "at least 4-fold returns in 50 days" across three backtests.
- **Relevance to our system:** Important as a *cautionary* exemplar. The 4x-in-50-days headline is exactly the kind of result our robustness gate exists to debunk: the test period (2017) was an extreme crypto bull run where buy-and-hold itself multiplied several-fold, the asset universe was selected by post-hoc liquidity (survivorship/selection bias), there is no 1000-path bootstrap or weakest-link verdict, and slippage/market-impact beyond a flat commission is ignored. Later independent work repeatedly fails to reproduce such returns out-of-sample. For us: the EIIE/PVM *architecture* is interesting if we ever go multi-coin, but the result should NOT be read as evidence DRL beats holding — benchmark against buy-and-hold over the SAME window and it likely evaporates. Reinforces our thesis and gate design.

### [11] Graph-Based Learning for Stock Movement Prediction with Textual and Relational Data
- **Authors / Venue:** Qinkai Chen, Christian-Yann Robert / arXiv (also Journal of Financial Data Science)
- **Year:** 2021
- **Source:** arXiv:2107.10941
- **% read:** 35%
- **Summary:** Proposes MGRN (Multi-Graph Recurrent Network): combines textual sentiment from financial news with multiple relational graphs among stocks to predict next-day movement. The GNN fuses relation edges with a recurrent temporal encoder. Evaluated on STOXX Europe 600; reports better accuracy AND better trading-simulation metrics than non-relational benchmarks.
- **Relevance to our system:** Largely background — graph/cross-asset methods need MANY assets and rich relational + news data, which a single-coin advisor does not have. It does at least run a trading simulation (better than pure-accuracy papers), but the abstract doesn't confirm transaction-cost/slippage handling, and relational graphs over correlated equities are an overfitting and look-ahead-leakage hazard (the graph can encode future-correlated structure). Filed as a "if we ever go multi-coin, cross-asset relations might add signal, but mind leakage and costs" reference. Not actionable now.

### [12] Deep Reinforcement Learning for Trading — A Critical Survey
- **Authors / Venue:** Adrian Millea / Data (MDPI) 6(11):119
- **Year:** 2021
- **Source:** DOI:10.3390/data6110119 (arXiv-adjacent; MDPI open access)
- **% read:** 40%
- **Summary:** A deliberately *critical* survey of DRL applied to (mostly crypto) trading. Its central message is structural rather than celebratory: the community suffers from a "lack of consistency" — incomparable datasets, benchmarks, reward functions, state representations and risk measures — that "can significantly impede research and the development of DRL agents." Surveys promising directions (hierarchical RL to decompose the problem, model-based RL learning a world model, risk measures as reward-shaping) but pointedly does NOT claim DRL reliably beats baselines, leaving that an open question.
- **Relevance to our system:** Strong meta-support for our design philosophy. An expert survey concludes the DRL-trading field can't even compare results because everyone uses different data/metrics — precisely the problem our FROZEN gate (one bootstrap, one weakest-link verdict, buy-and-hold benchmark, fixed costs) is engineered to avoid. The author's emphasis on risk measures as reward-shaping echoes deep hedging [2]. Takeaway: don't chase DRL state-of-the-art; the field's own critical survey says there isn't a reproducible, baseline-beating standard to chase. Reinforces "stay simple, benchmark honestly."

### [13] Deep Reinforcement Learning for Automated Stock Trading: An Ensemble Strategy
- **Authors / Venue:** Hongyang Yang, Xiao-Yang Liu, Shan Zhong, Anwar Walid / ACM ICAIF 2020
- **Year:** 2020
- **Source:** arXiv:2511.12120 (mirror of ICAIF '20; also SSRN 3690996)
- **% read:** 50%
- **Summary:** The widely-cited FinRL ensemble paper. Trains three actor-critic agents (PPO, A2C, DDPG), then at each rebalancing picks the agent with the best recent validation Sharpe (rolling 3-month window), forming an ensemble that "adjusts to different market situations"; uses a turbulence index to force risk-off in crises. On 30 Dow stocks it reports ensemble Sharpe ≈ 1.12 vs A2C 1.10 / PPO 1.10 / DDPG 0.87, beating the DJIA and a min-variance portfolio.
- **Relevance to our system:** Cautionary, with one genuinely good idea. The good idea = the *validation-Sharpe agent-selection* and *turbulence risk-off* are exactly the kind of regime-adaptive switching logic worth knowing. BUT: it benchmarks against DJIA/min-variance, NOT a clean buy-and-hold-each-asset, costs handling is unclear, and a single ~2-3 year out-of-sample window gives a Sharpe with huge error bars — far from our 1000-path bootstrap weakest-link standard. A Sharpe of 1.1 on one path is not robust evidence. Reinforces: our gate would not crown this; the ensemble-selection idea is reusable, the performance claim is not trustworthy as stated.

### [14] Generative AI for End-to-End Limit Order Book Modelling (Token-Level Autoregressive Message-Flow Model)
- **Authors / Venue:** Peer Nagy, Sascha Frey, Silvia Sapora, Kang Li, Anisoara Calinescu, Stefan Zohren, Jakob Foerster / ACM ICAIF 2023
- **Year:** 2023
- **Source:** arXiv:2309.00638
- **% read:** 40%
- **Summary:** Treats the limit-order-book message stream like language: tokenizes order messages (digit-group tokens, LLM-style) and trains an autoregressive deep *state-space* network (S4-style) to generate the next message; a JAX-LOB simulator replays generated messages to evolve the book state end-to-end. Reports low out-of-sample perplexity and mid-price returns from generated order flow that significantly correlate with real data.
- **Relevance to our system:** Background for us specifically (it's high-frequency message-level; we trade spot bars), but it's an important data point in the "generative market simulator" lane I'm tracking for test data. The transferable insight: an autoregressive / sequence-model generator can produce realistic conditional market dynamics, and state-space models are an efficient backbone. For a single-coin bar-level advisor, a much simpler return-level generator (QuantGAN [9] or a diffusion model) is the right scale — this paper shows the high end of the same idea. Flag: confirms the field is moving from GANs toward autoregressive/diffusion simulators (more stable training).

### [15] Enhancing Time Series Momentum Strategies Using Deep Neural Networks (Deep Momentum Networks)
- **Authors / Venue:** Bryan Lim, Stefan Zohren, Stephen Roberts / Journal of Financial Data Science
- **Year:** 2019
- **Source:** arXiv:1904.04912
- **% read:** 75%
- **Summary:** Embeds deep learning into the classic time-series-momentum (TSMOM) volatility-scaling framework. A single LSTM jointly learns BOTH the trend signal AND the position size, trained by directly optimizing a differentiable **Sharpe-ratio loss** (decision, not forecast). Backtested on 88 continuous futures, the Sharpe-optimized LSTM more than DOUBLES traditional TSMOM Sharpe *with zero costs*. Crucially, they add a turnover-regularization term and show the edge persists only up to ~2–3 bps of transaction cost; beyond that the advantage erodes because the model trades a lot.
- **Relevance to our system:** One of the most relevant and *methodologically honest* papers here. (1) The "directly optimize Sharpe as the loss" idea is the forecasting-world version of deep hedging's risk-measure objective [2] — a clean template if we ever learn a sizing rule. (2) Its explicit cost-sensitivity curve is the discipline our literature mostly lacks and that our cost-net gate enforces: the headline 2x shrinks fast once you pay 2-3 bps, and crypto spot costs (spread + fees + slippage) typically EXCEED that. So even a well-built deep momentum model likely fails our buy-and-hold-net-of-costs bar on a single coin. Strong, concrete support for our thesis: the edge lives in the cost assumption. Also: turnover regularization is a transferable trick to keep any learned policy from overtrading.

### [16] Generation of Synthetic Financial Time Series by Diffusion Models
- **Authors / Venue:** Tomonori Takahashi, Takayuki Mizuno / Quantitative Finance (2025); arXiv 2024
- **Year:** 2024
- **Source:** arXiv:2410.18897
- **% read:** 50%
- **Summary:** Applies denoising diffusion probabilistic models (DDPMs) to generate synthetic financial series. Pipeline: wavelet-transform multiple series (price, volume, spread) into image representations, train a DDPM on the images, then inverse-wavelet back to time series. Reports that the generated data satisfy the key stylized facts (fat tails, volatility clustering, seasonality). Frames the contribution against GANs/VAEs, noting that "no model yet satisfies ALL the stylized facts" and positioning diffusion as a more stable, mode-collapse-resistant alternative.
- **Relevance to our system:** Directly relevant to our synthetic-test-data interest and a likely BETTER choice than a GAN for us. Diffusion models avoid the adversarial training instability and mode collapse that plague GANs like QuantGAN [9], while still reproducing stylized facts — exactly the properties we'd want from a generator that augments the moving-block bootstrap with novel-but-plausible regimes. The wavelet-to-image trick also jointly models price+volume+spread, which could give richer multi-feature synthetic paths. Caveats remain (validate stylized-fact match to OUR coin; the honest admission that no generator nails every stylized fact; guard against test-set leakage). Flag: top candidate generator to prototype for enriching test data.

### [17] (Re-)Imag(in)ing Price Trends
- **Authors / Venue:** Jingwen Jiang, Bryan T. Kelly, Dacheng Xiu / Journal of Finance 78(6):3193–3249
- **Year:** 2021 (SSRN) / 2023 (journal)
- **Source:** SSRN:3756587 (Journal of Finance, DOI:10.1111/jofi.13268)
- **% read:** 40%
- **Summary:** Renders each stock's recent OHLC + volume + moving-average history as a small candlestick *image*, then trains a CNN to classify the direction of future returns. The learned image patterns predict returns more accurately than hand-crafted trend/momentum signals, generate significant long-short portfolio alpha, and — striking — transfer across horizons (short-window patterns work at longer scales) and across markets (US-trained patterns work internationally). Published in a top finance journal (rigorous by the field's standards).
- **Relevance to our system:** Background with caveats. Two reasons it doesn't translate to our advisor: (1) the alpha is realized via a *long-short* portfolio across the cross-section of stocks — a single long-only coin can't take the short leg, and most of the documented edge is in shorting the predicted losers; (2) the headline results are not clearly net of realistic transaction costs, and long-short equity costs are far below crypto spot costs. The *method* (chart-as-image CNN) is elegant and the cross-market transfer is a genuinely robust finding, but for us it's a multi-asset, cost-sensitive technique. Interesting, not actionable for a single-coin paper advisor.

### [18] Deep Reinforcement Learning for Cryptocurrency Trading: Practical Approach to Address Backtest Overfitting
- **Authors / Venue:** Berend Gort, Xiao-Yang Liu, Xinghang Sun, Jiechao Gao, Shuaiyu Chen, Christina Dan Wang / arXiv (FinRL-related, AAAI Bridge 2023)
- **Year:** 2022
- **Source:** arXiv:2209.05559
- **% read:** 60%
- **Summary:** Confronts head-on that DRL-trading papers "optimistically report increased profits in backtesting [that] may suffer from false positives due to overfitting." Formulates overfitting detection as a hypothesis test: estimate the Probability of Backtest Overfitting (PBO, via combinatorially-symmetric cross-validation) for each trained agent and REJECT agents above a threshold before deployment. On 10 cryptos over a 2022 crash period, the less-overfitted (accepted) agents earned higher returns than more-overfitted agents, an equal-weight strategy, and the S&P crypto index.
- **Relevance to our system:** Very relevant — it's a kindred-spirit methodology. PBO / CSCV is a well-known López de Prado tool, and this paper operationalizes exactly the discipline our robustness gate embodies: assume backtest results are overfit until a statistical test says otherwise. We could consider adding a PBO-style overfitting probability as a *complementary* diagnostic alongside our moving-block bootstrap weakest-link verdict. Caveat to stay honest: their "beats equal-weight + index" claim rests on a SHORT 2-month out-of-sample window — directionally encouraging but not the multi-regime robustness we demand. Takeaway: adopt the *mindset and the PBO metric*; don't over-read the thin out-of-sample win.

### [19] Time-series Generative Adversarial Networks (TimeGAN)
- **Authors / Venue:** Jinsung Yoon, Daniel Jarrett, Mihaela van der Schaar / NeurIPS 2019
- **Year:** 2019
- **Source:** NeurIPS 2019 proceedings (paper 8789); arXiv-adjacent
- **% read:** 40%
- **Summary:** A general framework for generating realistic multivariate time series. Combines an autoencoder (embedding + recovery nets, so adversarial learning happens in a learned latent space) with a GAN (generator + discriminator) AND a *stepwise supervised loss* that explicitly rewards matching the real one-step-ahead transition distribution. This supervised term is the key innovation for preserving temporal dynamics. Evaluated on several datasets including stocks; introduces the now-standard discriminative-score and predictive-score (train-on-synthetic/test-on-real) metrics plus t-SNE/PCA overlap; reports better fidelity than prior GANs.
- **Relevance to our system:** Relevant as a generator option for synthetic test data, and as the source of the *evaluation protocol* we'd want regardless of generator choice: (1) discriminative score — can a classifier tell synthetic from real? and (2) predictive score / TSTR — does a model trained on synthetic transfer to real? These are exactly the validation gates we'd impose before trusting any generator (QuantGAN [9], diffusion [16], or TimeGAN) to augment our bootstrap. As for the generator itself, TimeGAN is general-purpose; the finance-specialized QuantGAN and the more-stable diffusion approach are likely better-suited, but TimeGAN's TSTR/discriminative-score evaluation is the durable contribution for us.

### [20] FinRL-Meta: Market Environments and Benchmarks for Data-Driven Financial Reinforcement Learning
- **Authors / Venue:** Xiao-Yang Liu, Ziyi Xia, Jingyang Rui, Jiechao Gao, Hongyang Yang, Ming Zhu, Christina Dan Wang, Zhaoran Wang, Jian Guo / NeurIPS 2022 Datasets & Benchmarks
- **Year:** 2022
- **Source:** arXiv:2211.03107
- **% read:** 40%
- **Summary:** Successor to FinRL focused on *data and benchmarks* for financial RL. Notably names the three core obstacles plainly: (1) the LOW signal-to-noise ratio of financial data, (2) SURVIVORSHIP BIAS in historical data, and (3) MODEL OVERFITTING in backtesting. Provides an automatic pipeline turning real-market data into hundreds of gym-style environments, reproduces popular papers as baselines, and uses cloud-hosted community competitions to expose over-optimized backtests via shared out-of-sample evaluation.
- **Relevance to our system:** Useful mainly because even the *FinRL ecosystem itself* — the group most invested in DRL trading — explicitly lists survivorship bias, low SNR, and backtest overfitting as the field's defining problems. That is third-party validation of our skeptical priors. Concretely, the "low signal-to-noise" framing is why a single-coin advisor should expect tiny, fragile edges, and the survivorship-bias warning matters if we ever expand the coin universe (only-surviving-coins backtests flatter active strategies). Background as software, but a strong citation for "the experts agree the failure modes are real."

### [21] Deep Learning for Bitcoin Price Direction Prediction: Models and Trading Strategies Empirically Compared
- **Authors / Venue:** Oluwadamilare Omole, David Enke / Financial Innovation 10(1) (Springer, open access)
- **Year:** 2024
- **Source:** DOI:10.1186/s40854-024-00643-1 (open mirror: scholarsmine.mst.edu/engman_syseng_facwork/1487)
- **% read:** 50%
- **Summary:** Compares CNN-LSTM, LSTNet, TCN and an ARIMA benchmark for Bitcoin price-DIRECTION prediction from on-chain features, with Boruta / genetic-algorithm / LightGBM feature selection. Best model (CNN-LSTM + Boruta) reaches 82.44% directional accuracy. Converting predictions into a long-AND-short trading strategy, they report an "extraordinary annual return of 6654%" and conclude this is "evidence of the potential profitability of predictive models in Bitcoin trading."
- **Relevance to our system:** A near-perfect CAUTIONARY case — exactly the kind of result our gate exists to reject. Red flags: (1) 82% directional accuracy on daily Bitcoin is implausibly high and screams look-ahead/feature leakage from on-chain data alignment; (2) a 6654% annual return is not a serious net-of-cost figure — at realistic crypto round-trip costs and the high turnover a daily directional signal implies, such a strategy bleeds out, and the paper's cost treatment is not credibly conservative; (3) it's long-AND-short (our advisor is long-only spot) and reports a single backtest path with no bootstrap/robustness test or buy-and-hold-over-same-window comparison stated. This is the literature's failure mode in one paper: high accuracy → astronomical headline return → "potential profitability," with the costs/leakage/robustness questions unaddressed. Strong reinforcement that our frozen, cost-net, buy-and-hold-benchmarked gate is necessary; treat such claims as null until independently reproduced.

### [23] Multi-level Deep Q-Networks for Bitcoin Trading Strategies
- **Authors / Venue:** Sattarov Otabek, Jaeyoung Choi / Scientific Reports 14 (Nature, open access)
- **Year:** 2024
- **Source:** DOI:10.1038/s41598-024-51408-w (open: PMC10774387)
- **% read:** 55%
- **Summary:** A DQN-based Bitcoin trader. Three stacked Q-networks: Trade-DQN (buy/hold/sell from price), Predictive-DQN (forecast price change from price + Twitter sentiment), and Main-DQN (fuses both for the final action). Reward balances profit, low risk (investment-threshold penalty), and trade activity; transaction fees modeled at a flat 1.5%. Reports a 29.93% increase in investment value and a Sharpe > 2.7, claimed to beat prior Bitcoin-trading studies.
- **Relevance to our system:** Directly on the DQN sub-area and crypto-specific, but a textbook example of why we don't trust such results. Critical flaws: (1) the OUT-OF-SAMPLE TEST IS ONLY THE FINAL 30 DAYS (720 hourly steps) — a Sharpe of 2.7 on one month is statistically meaningless (huge error bars), the antithesis of our 1000-path bootstrap weakest-link verdict; (2) NO buy-and-hold comparison over the same window is reported, so the headline gain may simply track Bitcoin's drift in that month; (3) single backtest path, no multiple seeds / walk-forward. To its modest credit it DOES model a (high) 1.5% fee and penalizes overtrading. Net: the M-DQN architecture is a reasonable engineering pattern, but the performance claim fails every robustness criterion our gate enforces. Confirms: short-window single-path Sharpe is the field's most common overstatement.

### [24] Empirical Asset Pricing via Machine Learning
- **Authors / Venue:** Shihao Gu, Bryan Kelly, Dacheng Xiu / Review of Financial Studies 33(5):2223–2273
- **Year:** 2018 (NBER w25398) / 2020 (journal)
- **Source:** NBER w25398; SSRN:3159577 (RFS, DOI:10.1093/rfs/hhaa009)
- **% read:** 50%
- **Summary:** The rigorous, large-scale benchmark for ML in (cross-sectional equity) asset pricing. Compares linear models, penalized regressions, random forests, GBRT, and neural networks on predicting monthly stock risk premia using ~94 firm characteristics. Trees and shallow neural nets win — roughly DOUBLING the out-of-sample Sharpe of regression-based strategies — and the gain is traced specifically to capturing NONLINEAR predictor interactions. All methods agree the dominant signals are variations on momentum, liquidity, and volatility. Crucially, the authors are explicit that absolute out-of-sample R² is TINY (≈0.3–0.4% monthly).
- **Relevance to our system:** The most credible "ML genuinely adds value" result in this whole batch — and it teaches the right lesson for us. (1) Where ML helps, it helps via *nonlinear interactions among MANY features across MANY assets in the cross-section* — precisely the setting a single-coin, few-feature advisor lacks; the edge is cross-sectional (rank assets), not time-series-directional on one coin. (2) Even in this favorable, data-rich setting the absolute predictability is minuscule (sub-1% R²); "doubling the Sharpe" is doubling a small number, realized only by a diversified long-short book over thousands of stocks. (3) The dominant signals (momentum/liquidity/volatility) are the SAME classic factors our simple strategies already encode. Bottom line: rigorous ML evidence says the genuine edge is cross-sectional and tiny — not a reason to add DL to a single-coin advisor, and consistent with "no active single-coin strategy robustly beats holding net of costs."

### [22] A Time Series is Worth 64 Words: Long-term Forecasting with Transformers (PatchTST)
- **Authors / Venue:** Yuqi Nie, Nam H. Nguyen, Phanwadee Sinthong, Jayant Kalagnanam (IBM/Princeton) / ICLR 2023
- **Year:** 2022 (arXiv) / 2023 (ICLR)
- **Source:** arXiv:2211.14730
- **% read:** 50%
- **Summary:** The Transformer "comeback" paper responding to DLinear [8]. Two ideas: (1) *patching* — split each series into subseries-level patches used as tokens (retains local semantics, cuts attention cost quadratically, lets the model see longer history); (2) *channel-independence* — each univariate channel shares one Transformer (no cross-channel attention). Adds masked self-supervised pretraining. Reports ~20% MSE reductions over prior transformers and, importantly, edges out the DLinear linear baseline on standard long-term forecasting benchmarks — restoring (a modest) case that a *properly designed* transformer can beat the trivial baseline.
- **Relevance to our system:** Background, but it refines the DLinear lesson rather than overturning it. The honest reading: simple linear models are a HARD baseline that most transformers fail to beat; only after careful design (patching + channel-independence + pretraining) does a transformer eke out a ~single-digit-percent forecasting-error edge — on physical/utility benchmarks, with no trading P&L or costs. For a single-coin advisor that's nowhere near worth the complexity/compute/overfitting cost. The durable takeaway pair [8]+[22]: (a) always beat the linear baseline first; (b) even when a fancy model wins, the margin is small and the metric is forecasting error, not cost-net return. Stay simple.


