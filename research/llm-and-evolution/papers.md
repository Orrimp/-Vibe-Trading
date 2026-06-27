# Papers — LLM Applications + Evolutionary / Genetic Methods in Trading

Ledger for the `llm-and-evolution` topic. One entry per paper, appended
immediately after reading. Source of truth for resume. See `../README.md` for
the contract.

Focus — **LLM:** LLM trading agents (FinGPT, FinMem, FinAgent, TradingGPT);
LLM financial sentiment / news; multi-agent LLM trading frameworks; LLM
reasoning for finance; financial LLM benchmarks (BloombergGPT, FinBen);
retrieval-augmented finance; LLM-generated signals/factors. **Evolution:**
genetic programming for trading rules; genetic algorithms for strategy /
parameter optimization; symbolic regression & automated alpha-factor mining
(Alpha101, RL-based AlphaGen, genetic alpha); AutoML for trading; evolutionary
strategy discovery; neuroevolution.

Skeptical lens (per task): where do LLMs *genuinely* add value (narration,
news/context, research assistance) vs. hype? Is evolutionary alpha-mining just
industrialized overfitting our FROZEN robustness gate would reject? Note
data-snooping / multiple-testing risk in every GP / symbolic-regression alpha
search.

---

### [1] FinGPT: Open-Source Financial Large Language Models
- **Authors / Venue:** Hongyang Yang, Xiao-Yang Liu, Christina Dan Wang / FinLLM Workshop @ IJCAI 2023 (Best Presentation Award); AI4Finance
- **Year:** 2023
- **Source:** arXiv:2306.06031
- **% read:** 40% (abstract + framework sections via abstract page)
- **Summary:** Proposes an open-source, *data-centric* financial LLM framework as a democratized alternative to the proprietary BloombergGPT. Core contribution is an automatic data-curation pipeline over Internet-scale financial text plus lightweight LoRA fine-tuning so the model can be cheaply re-tuned as markets/news drift (RLHF/RLSP for personalization). Targets downstream tasks: financial sentiment, NER, robo-advising, and algorithmic-trading signal generation. The paper is framed as infrastructure / a "stepping stone" — it reports the pipeline and tasks, NOT a controlled trading-PnL study.
- **Relevance to our system:** Two distinct uses. (a) The narration seam — a small LoRA-tuned open model could power our "why this one" explanations cheaply and locally. (b) Sentiment/NER as an *exogenous feature*, not a strategy on its own. Key caveat for us: the paper validates NLP capability and data plumbing, NOT that any of it beats buy-and-hold net of costs — exactly the gap our FROZEN gate exists to expose. Use FinGPT-style tooling for *context/narration*, never as an un-gated alpha source.

### [2] FinMem: A Performance-Enhanced LLM Trading Agent with Layered Memory and Character Design
- **Authors / Venue:** Yangyang Yu, Haohang Li, Zhi Chen, et al. / AAAI 2024 Spring Symposium (also arXiv)
- **Year:** 2023
- **Source:** arXiv:2311.13743
- **% read:** 70% (abstract + experiments/results via ar5iv HTML)
- **Summary:** An LLM trading agent with three modules: Profiling (persona + risk preference), a human-like layered Memory (shallow/intermediate/deep layers with different decay rates so daily news fades fast and 10-Ks persist), and Decision-making. Tested on 5 high-news-volume tickers (TSLA, NFLX, MSFT, AMZN, COIN), train Aug-2021→Oct-2022, test Oct-2022→Apr-2023, vs buy-and-hold, A2C/PPO/DQN DRL agents, generic Generative Agents, and FinGPT. Reported strong out-performance, e.g. TSLA +61.8% (Sharpe 2.68) vs B&H −18.6% (Sharpe −0.54); COIN +35.0% vs −30.0%; AMZN/MSFT/NFLX more marginal.
- **Relevance to our system:** The single most important caveat for us: results are on **only 5 tickers, hand-picked for highest news volume** (selection bias), over **one ~6-month bearish test window** where any agent that goes flat/short beats a falling-knife B&H — this is precisely the kind of small-N, single-path, favorable-window result our 1000-path moving-block bootstrap gate is designed to reject. The paper does **not** address look-ahead bias and trains on "future price data" to derive labels (asymmetry vs test). Layered-memory *idea* (decay-weighted context) is a genuinely interesting input-construction trick; the *trading claims* are not gate-credible evidence. Treat as: good agent-architecture inspiration, NOT proof LLMs beat holding.

### [3] TradingAgents: Multi-Agents LLM Financial Trading Framework
- **Authors / Venue:** Yijia Xiao, Edward Sun, Di Luo, Wei Wang / arXiv (Tauric Research; open-source)
- **Year:** 2024
- **Source:** arXiv:2412.20138
- **% read:** 70% (abstract + experiments via HTML v1)
- **Summary:** Models a trading firm as a society of specialized LLM agents — fundamental/sentiment/technical/news analysts, Bull vs Bear *researcher debate*, a trader, and a risk-management team — that communicate in natural language to reach a decision. Tested on AAPL/GOOGL/AMZN over Jun–Nov 2024 vs B&H, MACD, KDJ, RSI, ZMR, SMA. Reports eye-popping numbers: AAPL CR 26.6% (Sharpe 8.21, MDD 0.91%) vs best baseline 2.05%; similar for GOOGL/AMZN.
- **Relevance to our system:** A Sharpe of 8 with <1% max-drawdown is a giant red flag, not a selling point — it is the signature of an over-fit / leaked result, not a deployable edge. The setup has *every* failure mode our gate guards against simultaneously: 3 assets, one 5-month window, **no transaction costs**, and LLM training cutoffs (gpt-4o / o1-preview) that overlap the 2024 test period (the model may "remember" the price path). Useful to us ONLY as a design pattern: the *debate / multi-role decomposition* is a nice way to structure a **narration / research-assistant** layer (bull-case vs bear-case text for the operator). As an alpha engine it is exactly what our 1000-path bootstrap-vs-B&H gate exists to reject.

### [4] FinAgent: A Multimodal Foundation Agent for Financial Trading (Tool-Augmented, Diversified, Generalist)
- **Authors / Venue:** Wentao Zhang, Lingxuan Zhao, Haochong Xia, et al. / ACM SIGKDD 2024
- **Year:** 2024
- **Source:** arXiv:2402.18485
- **% read:** 70% (abstract + experiments via HTML v3)
- **Summary:** A multimodal (numeric + text + visual/chart-image) LLM trading agent with a market-intelligence module, a dual-level reflection module, diversified memory retrieval, and tool augmentation (it can call established indicators/expert strategies). Evaluated on 5 US stocks + ETHUSD, train 2022-06→2023-06, test 2023-06→2024-01, against 12 baselines (rule-based, LGBM/LSTM/Transformer, DQN/SAC/PPO, FinGPT, FinMem). Claims >36% avg profit improvement; headline 92.27% ARR on TSLA; beats B&H on ARR for all 6 assets.
- **Relevance to our system:** More baselines and more assets than TradingAgents — better, but still **single test window, costs only vaguely "considered," and GPT-4V's Apr-2023 cutoff overlaps the test window** (visual chart "analysis" could be memorization). Tellingly, on the one asset closest to ours (crypto, ETHUSD) it *underperforms* baselines and the authors admit their tools are stock-specialized — a direct caution that stock-tuned LLM agents do NOT transfer to crypto. The genuinely reusable idea is **tool-augmentation**: an LLM that *calls* vetted indicators rather than inventing signals. For us that maps to: LLM orchestrates/narrates over our gated strategy library, it does not itself emit ungated alpha.

### [5] BloombergGPT: A Large Language Model for Finance
- **Authors / Venue:** Shijie Wu, Ozan Irsoy, Steven Lu, et al. (Bloomberg) / arXiv
- **Year:** 2023
- **Source:** arXiv:2303.17564
- **% read:** 40% (abstract + corpus/eval framing)
- **Summary:** A 50B-parameter decoder-only LLM trained from scratch on ~708B tokens, ~363B of them proprietary financial text (FinPile) + ~345B general. Outperforms similarly-sized open models on financial NLP by large margins while staying competitive on general benchmarks. Evaluated purely on NLP: sentiment, NER, classification, ConvFinQA, plus standard LM benchmarks. Closed/proprietary (no weights released).
- **Relevance to our system:** Important *negative* data point: the flagship "finance LLM" paper makes **zero trading-PnL claims** — it proves better *language* understanding of finance, not better *returns*. Directly supports our thesis split: LLMs are validated for **text comprehension** (sentiment, extraction, narration — useful to our advisor's context/"why this one" layer) but NOT for alpha. Also a cost/closedness caution: a 50B from-scratch model is irrelevant to our local advisor; a small fine-tuned open model (FinGPT-style, [1]) is the realistic narration substrate.

### [6] Assessing Look-Ahead Bias in Stock Return Predictions Generated by GPT Sentiment Analysis
- **Authors / Venue:** Paul Glasserman, Caden Lin (Columbia) / arXiv (also J. of Financial Data Science)
- **Year:** 2023
- **Source:** arXiv:2309.17322
- **% read:** 50% (abstract + methodology summary)
- **Summary:** Tests whether GPT-based news-sentiment return prediction is contaminated by look-ahead bias (the model "knowing" how the stock later moved, because the headline + outcome were in its training data). Method: compare strategies on *original* headlines vs *anonymized* headlines (company identifiers stripped), in-sample vs out-of-sample. Surprising finding: in-sample, anonymized headlines *outperform* — a "distraction effect" (the model's general knowledge of the named company interferes) dominates raw look-ahead bias. Proposes anonymization as a de-biasing / out-of-sample backtesting procedure.
- **Relevance to our system:** Directly load-bearing for any LLM-sentiment feature we might add. Two concrete protocols for us: (1) if we ever feed news to an LLM for a signal, **test only on data after the model's training cutoff**, or use **anonymized text**, to keep the backtest honest; (2) be aware LLM sentiment is noisy in *both* directions (look-ahead inflation AND name-distraction). Reinforces that our gate must treat any LLM-derived signal as suspect until validated on genuinely out-of-sample (post-cutoff) data — abstract NLP accuracy is not enough.

### [7] FinBen: A Holistic Financial Benchmark for Large Language Models
- **Authors / Venue:** Qianqian Xie, Weiguang Han, Zhengyu Chen, et al. / NeurIPS 2024 Datasets & Benchmarks
- **Year:** 2024
- **Source:** arXiv:2402.12659
- **% read:** 50% (abstract + results framing)
- **Summary:** Largest open financial LLM benchmark at publication: 36 datasets, 24 tasks, 7 aspects — information extraction, textual analysis, QA, text generation, risk management, **forecasting**, and **decision-making / stock trading** (first benchmark to evaluate trading + agent + RAG settings). Headline finding is a *stark divide*: LLMs (incl. GPT-4) excel at IE and textual analysis but **struggle with forecasting and complex reasoning**; they "remain fundamentally constrained in quantitative prediction."
- **Relevance to our system:** This is the cleanest *independent, benchmark-grade* confirmation of our thesis split. The very paper that adds trading/forecasting evaluation concludes LLMs are strong on language, weak on numbers/forecasting. So: use LLMs for the text-shaped jobs in our advisor (extraction, summarization, the "why this one" narration) and **do not expect them to forecast price** — consistent with "no active strategy robustly beats holding." Also a reusable asset: FinBen's task taxonomy is a good menu for scoping *which* LLM jobs are even worth attempting.

### [8] Retrieval-Augmented LLMs for Financial Time Series Forecasting (FinSeer / StockLLM)
- **Authors / Venue:** Mengxi Liu, Yibo Wang, et al. / arXiv
- **Year:** 2025
- **Source:** arXiv:2502.05878
- **% read:** 75% (abstract + method + results + limitations via HTML)
- **Summary:** First RAG framework for financial time-series forecasting. A retriever (FinSeer) is trained via LLM-feedback / knowledge distillation to fetch historically *significant* sequences that help a small fine-tuned 1B model (StockLLM) predict next-day up/down. On ACL18/BIGDATA22/STOCK23 (US stocks), RAG beats bare StockLLM and text-trained retrievers — but the absolute accuracy is **~52–54% (barely above a coin flip)**, and crucially **text-trained retrievers often *degrade* performance**. The authors are honest: no profitability/cost/regime analysis; modest absolute accuracy.
- **Relevance to our system:** A careful, honest RAG-for-forecasting paper that still lands at ~54% directional accuracy with **no claim it translates to profit** — i.e. even "good" LLM forecasting is marginal and unproven on PnL. Two lessons: (1) generic text embeddings are the *wrong* retrieval substrate for numeric time series (they chase surface patterns) — a caution if we ever bolt RAG onto context; (2) directional accuracy ≠ tradeable edge: 54% can still lose to costs, exactly why our gate measures *equity vs B&H net of costs*, not hit-rate. Confirms RAG helps *retrieval/context*, not magically *forecasting profit*.

### [9] Can ChatGPT Forecast Stock Price Movements? Return Predictability and Large Language Models
- **Authors / Venue:** Alejandro Lopez-Lira, Yuehua Tang (Univ. of Florida) / arXiv + SSRN (widely cited)
- **Year:** 2023 (rev. through 2025)
- **Source:** arXiv:2304.07619
- **% read:** 60% (abstract + key-results discussion)
- **Summary:** The seminal "can an LLM read the news and predict returns" study. ChatGPT rates each headline good/bad/irrelevant; scores form a daily long-short portfolio. Finds a positive ChatGPT-score → next-day-return correlation; ChatGPT beats traditional sentiment dictionaries; smaller models (GPT-1/2, BERT) can't do it — return predictability is an *emergent* capability. Crucially, using **post-cutoff** headlines GPT-4 gets ~90% hit rate on the **non-tradable initial reaction** and *also* predicts a *tradable post-publication drift*, concentrated in **small stocks and negative news**. Returns **decline as LLM adoption rises** (price efficiency).
- **Relevance to our system:** The most careful pro-LLM-signal evidence — and even it is heavily hedged in ways that matter for us. The big hit-rate is on the *non-tradable* instantaneous move; the tradable part lives in *illiquid small-caps* (high cost/slippage) and is *decaying with adoption*. For a single-coin crypto advisor that's a triple caution: our asset is liquid+efficient, costs bite, and any edge self-erodes. Net: LLM news-sentiment is a *real but fragile, cost-sensitive, decaying* signal — plausibly worth a gated experiment as an exogenous feature, but absolutely not a standalone alpha, and must be tested post-cutoff + net of costs. Strongly consistent with our "no robust edge over holding" thesis.

### [10] EFS: Evolutionary Factor Searching for Sparse Portfolio Optimization Using Large Language Models
- **Authors / Venue:** (Chinese A-share factor research group) / arXiv
- **Year:** 2025
- **Source:** arXiv:2507.17211
- **% read:** 55% (abstract + method + skim of results via PDF; some numerics unreadable in compressed PDF)
- **Summary:** Bridges the two halves of this topic — an **evolutionary loop where the LLM is the mutation/generation operator**: the LLM proposes candidate factors, they're scored, and the best are iteratively refined for sparse-portfolio construction. Evaluated on Chinese A-shares vs traditional value/momentum/quality factors, equal-weight, and buy-and-hold; claims improved returns and Sharpe.
- **Relevance to our system:** Textbook example of the **industrialized-overfitting risk** my mandate flags. An evolutionary search that iteratively optimizes factors *against the same dataset* by performance feedback is multiple-testing at scale — the search WILL find spurious patterns. The paper shows no clear walk-forward / held-out validation, no transaction-cost modeling, and no multiple-testing correction. Verdict for us: this is exactly the class of result our **1000-path moving-block bootstrap vs B&H gate is built to reject** — an in-sample-tuned factor sweep would almost certainly collapse under the weakest-link bootstrap verdict. Using an LLM as the factor-*generator* doesn't change the statistics; it just generates spurious candidates faster. Lesson: if we ever do automated factor/param search, the search budget must be *charged* against significance (deflated Sharpe / PBO) and validated out-of-sample — see backtesting topic.

<!-- NOTE: "Chain-of-Alpha" (arXiv:2508.06312), a dual-chain LLM formulaic-alpha
miner, was found but has been WITHDRAWN by arXiv admins (licensing) and the full
text is unavailable. Not logged as a read paper per the no-fabrication rule.
The LLM-alpha-mining theme is covered by EFS [10] and (pending) the survey. -->

### [11] Using Genetic Algorithms to Find Technical Trading Rules
- **Authors / Venue:** Franklin Allen, Risto Karjalainen / Journal of Financial Economics 51(2):245-271
- **Year:** 1999
- **Source:** DOI:10.1016/S0304-405X(98)00052-X (JFE 51(2):245-271; RePEc handle eee/jfinec/v51y1999i2p245-271)
- **% read:** 25% (search-result abstract + bibliographic record; full PDF could not be machine-parsed and the paywalled RePEc page has no abstract — % read kept honest)
- **Summary:** The foundational study using genetic programming to *evolve* technical trading rules (as program trees over price/indicator primitives) on daily S&P 500, 1929–1995, with an explicit train / selection / out-of-sample test split designed to fight overfitting. The motivation is itself the data-snooping critique: hand-picked technical rules look profitable precisely *because* they were chosen for past profitability, so the authors let GP search the rule space "optimally, ex ante." Widely cited result: the evolved rules did **not** earn consistent excess returns over buy-and-hold out-of-sample once transaction costs were included (the S&P equity result is the skeptical headline; related FX work found more out-of-sample profit).
- **Relevance to our system:** The 25-year-old direct ancestor of every modern GP/RL "alpha miner" — and its honest conclusion *is our thesis*: a careful evolutionary search over trading rules, properly held out and costed, **fails to robustly beat buy-and-hold on a liquid equity index**. This is the strongest historical evidence that automated rule discovery is largely industrialized data-snooping. It also models the right discipline (train/select/**test** split + costs) that the flashy modern papers ([3][4][10]) drop. For us: validates that our B&H-benchmarked, cost-aware, bootstrapped gate is the correct and *time-tested* defense; and warns that any param/rule sweep we run will tend to manufacture in-sample winners that die out-of-sample.

### [12] Generating Synergistic Formulaic Alpha Collections via Reinforcement Learning (AlphaGen)
- **Authors / Venue:** Shuo Yu, Hongyan Xue, Xiang Ao, et al. / KDD 2023 (Applied Data Science track)
- **Year:** 2023
- **Source:** arXiv:2306.12964 (also arXiv:2401.02710 follow-up); code github.com/RL-MLDM/alphagen
- **% read:** 65% (abstract + method + experiments via HTML)
- **Summary:** Pioneers RL (PPO) to *generate sets of formulaic alphas* as expression trees; the reward optimizes a *synergistic* combined-factor IC (diversity/complementarity, not single-factor IC), so the agent builds a portfolio of factors a downstream model combines. On CSI300 (train 2009-2018, valid 2019, test 2020-2021), reports IC improving ~0.045 → ~0.085 over baselines, via a Top-K/Swap-N long-only strategy.
- **Relevance to our system:** The modern, more rigorous heir to [11] — and notably it *does* use a temporal train/valid/test split (better than the LLM-agent papers). But the gaps that matter to us remain: **no transaction costs**, **no quantitative buy-and-hold / index outperformance** (only a visual cumulative-return chart vs CSI300), and the metric is IC (correlation), which is **not** PnL-net-of-costs. A doubled IC of 0.085 is still a *weak* correlation that can easily fail to monetize after costs/turnover (the Top-K/Swap-N rebalancing is turnover-heavy). For us: even the best-engineered alpha-mining pipeline reports *statistical* signal, not *gate-credible* tradeable edge. If we ever evaluate such factors, we must run them through our cost-aware bootstrap-vs-B&H gate — IC is necessary, never sufficient.

### [13] 101 Formulaic Alphas
- **Authors / Venue:** Zura Kakushadze / Wilmott Magazine 2016 (+ arXiv)
- **Year:** 2016
- **Source:** arXiv:1601.00991
- **% read:** 45% (abstract + structure + properties discussion)
- **Summary:** Discloses 101 explicit formulaic alpha expressions (as code) representative of those used at WorldQuant. Key reported aggregate properties: short average holding periods (~0.6–6.4 days) and **low average pairwise correlation (~15.9%)**. The paper is a *catalog of weak, short-horizon, low-correlation signals*, framed around their *aggregate/diversification* properties — NOT a claim that any single alpha is strongly profitable. (Note: a web snippet claimed these were "derived via GP"; the paper itself does **not** say that — it presents them as disclosed formulas. Correction logged.)
- **Relevance to our system:** The canonical reference for "what an alpha factor actually is" — and the honest picture is sobering: industrial alphas are *individually weak*, short-horizon, and only useful *combined across hundreds* with heavy diversification, frequent rebalancing, and (implicitly) institutional-scale low costs. That entire regime is **inapplicable to our single-coin, retail, cost-sensitive advisor** — we have one asset, so cross-sectional diversification across 101 alphas is impossible, and short holding periods mean ruinous turnover costs. Strong support for our thesis: the alpha-factor paradigm is a *cross-sectional, institutional* game; for a single coin held by a retail budget, it offers little, and buy-and-hold remains the bar. Useful background for *why* single-name technical signals rarely clear costs.

### [14] Agent-Based Genetic Algorithm for Crypto Trading Strategy Optimization (CGA-Agent)
- **Authors / Venue:** (crypto GA / multi-agent group) / arXiv
- **Year:** 2025
- **Source:** arXiv:2510.07943
- **% read:** 65% (abstract + method + results via HTML)
- **Summary:** A GA where six LLM/agent roles (Analysis, Generate, Evaluate, Choose, Crossover, Mutation) tune the parameters of a dual-RSI scalping strategy, fitness = a weighted blend of 11 metrics (Sharpe, Sortino, …). Tested on BTC/ETH/BNB 5-minute candles over ~252 days (Dec-2024→Sep-2025), vs the *un-optimized* same strategy. Reports huge PnL: ETH +550%, BNB +169%, BTC +29%.
- **Relevance to our system:** A near-perfect case study of what my mandate warns against — and *directly in our asset class* (crypto, BTC/ETH). It has **no held-out test (re-optimizes every 30 days on the same stream), no transaction costs (on 5-minute scalping!), no buy-and-hold benchmark, and zero overfitting discussion.** A +550% scalping return with no costs is not evidence of edge; it is the fingerprint of in-sample over-optimization that would evaporate under our gate. The "agent-based" wrapper around the GA changes nothing statistically — it just searches the over-fit space more elaborately. For us this is the canonical *anti-pattern*: our 1000-path bootstrap-vs-B&H-net-of-costs gate exists precisely to reject results shaped exactly like this one.

### [15] A Novel Approach to Trading-Strategy Parameter Optimization Using Double Out-of-Sample Data and Walk-Forward Techniques
- **Authors / Venue:** (tmr-crypto; GitHub repo wf_optim_crypto_analysis) / arXiv
- **Year:** 2026
- **Source:** arXiv:2602.10785
- **% read:** 80% (abstract + method + results + limitations via HTML)
- **Summary:** The methodological *antithesis* of [14], on crypto. Splits data into a Global Training period (optimize via walk-forward) and an **Unseen period evaluated exactly once** with pre-selected params (double-out-of-sample, to stop the "test repeatedly on 'OOS' data" contamination). Optimizes EMA-crossover params *and* the walk-forward window lengths (81 window combos × 6 intraday frequencies), with realistic **0.1% transaction costs** throughout. Honest result: in-sample, every config beat B&H; **out-of-sample the strategy merely *matched* B&H (with lower drawdown)**; at 1–30-min frequencies mean Sharpe was *negative*. A bootstrap vs 1000 random EMA parameter sets showed the *optimized* params beat random only **8–13%** of the time. The only win was a *B&H + strategy portfolio* (diversification, −50% drawdown), not strategy superiority.
- **Relevance to our system:** Essentially an external replication of OUR ENTIRE THESIS and gate. It (a) uses a 1000-path bootstrap baseline (our gate), (b) insists on cost-aware, truly-once OOS evaluation, and (c) concludes optimized active params **do not robustly beat buy-and-hold** on crypto and often lose to *random* params — the definitive rebuttal to GA/agent optimizers like [14]. Direct, gate-credible support for "no active strategy robustly beats holding, net of costs." Methodologically it is a model we can cite: double-out-of-sample + optimize-the-window-too + bootstrap-vs-random as an *overfitting detector*. The "B&H + strategy portfolio reduces drawdown" finding is the one genuinely actionable positive — worth testing as an advisor *risk* feature, not an alpha claim.

### [16] AlphaForge: A Framework to Mine and Dynamically Combine Formulaic Alpha Factors
- **Authors / Venue:** Hao Shi, Weili Song, Xinting Zhang, et al. / AAAI 2025 (also arXiv)
- **Year:** 2024
- **Source:** arXiv:2406.18394
- **% read:** 70% (abstract + method + results via HTML v3)
- **Summary:** A generative-predictive (neural) alpha miner: a generator proposes factors to maximize a surrogate predictor's fitness while a diversity loss keeps them distinct — contrasted against GP, Deep Symbolic Optimization (DSO), and RL miners on CSI300/CSI500. Its headline idea is **dynamic factor combination**: re-fit factor weights *daily* via regression, keeping the top-N by recent IC/ICIR to combat **factor decay** ("congestion, market-style shifts"). Reports higher IC (4.40% vs 2.09% RL) and a 21.68% real-money excess return over 9 months on CSI500.
- **Relevance to our system:** The one genuinely interesting *idea* in the alpha-mining cluster for us is the explicit acknowledgment of **factor decay** and the response of *dynamically re-weighting* — i.e. a crowned strategy's edge is not stationary. That maps onto a real question for our advisor: should a crowned pick be *periodically re-baked* as its edge decays? But the usual caveats stand hard: **no transaction costs, no buy-and-hold/index net comparison**, IC of ~4% is still weak, and the 9-month real-money claim is short and unverified. The decay insight is worth borrowing (periodic re-evaluation cadence); the alpha-mining machinery itself is not gate-credible and remains a cross-sectional, cost-blind exercise inapplicable to single-coin retail.

### [17] NEAT Algorithm-based Stock Trading Strategy with Multiple Technical Indicators Resonance
- **Authors / Venue:** (NEAT trading group) / arXiv
- **Year:** 2025
- **Source:** arXiv:2501.14736
- **% read:** 70% (abstract + method + results via HTML)
- **Summary:** Applies NEAT (neuroevolution — evolving both NN topology and weights) to evolve a trading network over 7 technical indicators ("resonance" across indicators) on 503 S&P 500 constituents, 22 years (1999–2022), vs buy-and-hold. Result: evolved model averages **18.76% return vs B&H's 27.97%** — it *underperforms* B&H by ~9 pts, with somewhat lower volatility and exposure. Authors note unused nodes/connections (evolutionary "bloat").
- **Relevance to our system:** A refreshingly honest *negative* result: a sophisticated neuroevolution approach with multi-indicator inputs **loses to buy-and-hold by ~9 percentage points before transaction costs even enter** — and costs would only widen the gap. The slight volatility reduction is the same "risk-not-alpha" pattern seen in [13][15]. Strong, direct support for our thesis on a 22-year liquid-market sample. Also a concrete caution about neuroevolution specifically: topology-evolving methods bloat and overfit, and "resonance across many indicators" does not manufacture an edge that survives the simplest benchmark. For us: neuroevolution offers nothing our gate would crown over B&H.

### [18] FinLlama: Financial Sentiment Classification for Algorithmic Trading Applications
- **Authors / Venue:** Thanos Konstantinidis, Giorgos Iacovides, et al. / ACM ICAIF 2024 (also arXiv)
- **Year:** 2024
- **Source:** arXiv:2403.12285
- **% read:** 40% (abstract + architecture; trading-backtest specifics not in the excerpt)
- **Summary:** A finance-tuned sentiment model: LoRA fine-tune of Llama-2-7B on 34,180 labeled financial-text samples across 4 datasets, with a generator-discriminator scheme that outputs both sentiment *polarity* (3-class) and *strength*, feeding a small NN decision mechanism. Claims the sentiment signal supports "enhanced portfolio management decisions and increased market returns."
- **Relevance to our system:** Representative of the "fine-tune a small open LLM for finance sentiment" recipe that is the realistic substrate for any sentiment feature we'd add (and for narration, à la FinGPT [1]). The *NLP* contribution (polarity + strength via LoRA) is credible and cheap. But the trading claim is unquantified in the abstract — **no stated transaction costs, no explicit B&H comparison, no post-cutoff OOS framing** — i.e. the same evidentiary gap as the rest of the LLM-trading literature ([2][3][4][9]). Take-away for us: FinLlama-style models are a fine *engineering* choice for a sentiment/narration component, but the "increased returns" claim must be re-proven inside our cost-aware, post-cutoff, bootstrap-vs-B&H gate before we believe it. NLP accuracy ≠ tradeable edge (theme echoing [5][7]).

### [19] From Deep Learning to LLMs: A Survey of AI in Quantitative Investment
- **Authors / Venue:** (quant-AI survey group) / arXiv
- **Year:** 2025
- **Source:** arXiv:2503.21422
- **% read:** 55% (full HTML skim of taxonomy + critical sections)
- **Summary:** A landscape survey organizing AI-in-quant across three eras (statistical → deep learning → LLM agents) along the pipeline Data → Prediction → Portfolio Opt → Execution. Notably *circumspect*: states LLM deployment "is still in its early stages," that LLM-for-portfolio-optimization yields "modest outcomes so far," that DL models risk overfitting and lack transparency, and that "model predictions themselves cannot be directly used as investment decisions." Makes **no claim** that AI/DL/LLM reliably beats passive benchmarks; it is a taxonomy, not a validation study.
- **Relevance to our system:** A useful authoritative, *neutral-skeptical* anchor citation. Even a broad survey of the whole field declines to assert AI beats passive, and explicitly flags overfitting, the prediction→decision gap, and LLM immaturity — all consistent with our thesis and our gate's reason to exist. Two reusable framings for our docs: (1) the four-stage pipeline (Data→Prediction→PortfolioOpt→Execution) is a clean way to scope where our advisor uses AI (mostly *not* in prediction); (2) its honest "modest outcomes so far" is the field's own verdict on LLM alpha — supports keeping LLMs on narration/context, not signal generation. Critique: the survey *under*-examines costs and the backtest-vs-live gap, which our gate foregrounds.

### [20] Alpha-GPT 2.0: Human-in-the-Loop AI for Quantitative Investment
- **Authors / Venue:** Hang Yuan, Saizhuo Wang, Jian Guo, et al. / arXiv (follows Alpha-GPT, arXiv:2308.00016)
- **Year:** 2024
- **Source:** arXiv:2402.09746
- **% read:** 35% (abstract + framework positioning; full empirical detail not in excerpt)
- **Summary:** A multi-agent LLM system spanning the quant pipeline — specialized agents for alpha *mining*, *modeling*, and *analysis*, fronted by an "AlphaBot" LLM layer that translates human research intent into structured tasks and uses RAG over a vector DB of financial literature + historical alphas. The explicit design philosophy is **Human-in-the-Loop**: AI accelerates and orchestrates, the human researcher steers and judges throughout, rather than autonomous alpha generation.
- **Relevance to our system:** The most aligned paper with our actual LLM posture. It frames the LLM as a **research assistant / orchestrator** — interpreting intent, retrieving relevant prior work, driving tools — with a human in the loop to catch nonsense. That is exactly the safe, value-adding role for LLMs in our advisor: narrate, retrieve context, help the *operator* reason, while the **robustness gate (not the LLM)** decides what's real. The human-in-the-loop is, in effect, a manual analogue of our automated gate against spurious alphas. Caveat: the paper still operates in the alpha-mining paradigm and the abstract doesn't quantify whether the human-in-the-loop measurably reduces false discoveries. Reusable design: RAG-over-financial-literature + intent-translation is a clean blueprint if we expand our "why this one" seam into an interactive research helper.

### [21] Navigating the Alpha Jungle: An LLM-Powered MCTS Framework for Formulaic Factor Mining
- **Authors / Venue:** Yu Shi, et al. / AAAI 2026 (also arXiv)
- **Year:** 2025
- **Source:** arXiv:2505.11122
- **% read:** 35% (abstract + method; specific metrics not in excerpt)
- **Summary:** Combines an LLM (to generate/refine *symbolic* alpha formulas) with Monte Carlo Tree Search, where MCTS exploration is **guided by backtest feedback** on each candidate factor, plus a "frequent-subtree-avoidance" mechanism for diversity. Claims superior predictive accuracy + trading performance vs prior GP/RL miners, with formulas "more amenable to human interpretation."
- **Relevance to our system:** The apex example of the data-snooping concern my mandate flags: an LLM proposing formulas *and* an MCTS search *both optimizing against backtest feedback* over a vast formula space is a maximally efficient multiple-testing machine — and the abstract gives **no held-out test, no multiple-testing correction, no transaction costs, no B&H-net comparison** (the metric is predictive accuracy, not cost-aware PnL). The interpretability gain (readable formulas vs black-box) is the one genuine plus, and matters for *our* narration goal: a crowned strategy's logic in human-readable form aids explanation. But the mining itself is exactly what our 1000-path bootstrap-vs-B&H gate should expect to reject — "better at finding in-sample formulas faster" is not "found a real edge." Confirms the pattern across [10][12][16]: LLM/RL/GP make the *search* fancier without changing the statistics that doom it.

### [22] StockBench: Can LLM Agents Trade Stocks Profitably in Real-world Markets?
- **Authors / Venue:** (StockBench group) / arXiv
- **Year:** 2025
- **Source:** arXiv:2510.02209
- **% read:** 70% (abstract + setup + results via HTML v2)
- **Summary:** A *contamination-clean* reality check: 14 frontier LLMs (GPT-5, Claude-4-Sonnet, o3, DeepSeek-V3, Qwen3, Kimi-K2, …) trade the top-20 DJIA stocks over Mar 3–Jun 30 2025 — a window **deliberately after the models' knowledge cutoffs to prevent leakage**. Agents see prices, fundamentals, and recent news; **costs/slippage are NOT modeled (making trading easier than reality)**. Result: B&H returned 0.4% (MDD −15.2%). Best model Kimi-K2 1.9%; most clustered ~2–2.5%; two models lost money. Crucial nuance: "most models outperform B&H" only in aggregate over a roughly flat/up window — **during downturns ALL LLM agents underperform the passive baseline.** Strong financial-QA ability "does not necessarily translate into effective trading."
- **Relevance to our system:** The single best capstone for our thesis on the LLM side — and the most rigorous LLM-trading evaluation in the set (post-cutoff = no leakage). Even so, the "win" over B&H is tiny (≈1–2%), happens in a benign window *without costs*, and **completely inverts in downturns** — i.e. once you add our transaction costs and a 1000-path bootstrap spanning bad regimes, the apparent edge would almost certainly vanish or go negative. Directly validates: (a) keep LLMs off the alpha rail; (b) our gate's regime-spanning bootstrap + cost-awareness is exactly the right test (StockBench's downturn result is what a single benign backtest hides); (c) QA/benchmark skill ≠ trading skill (echoes [5][7][19]). This is the paper to cite when asked "but can't a frontier LLM just trade?"

### [23] Standard Benchmarks Fail — Auditing LLM Agents in Finance Must Prioritize Risk
- **Authors / Venue:** (LLM-finance-audit group) / arXiv
- **Year:** 2025
- **Source:** arXiv:2502.15865
- **% read:** 45% (abstract + argument + framework)
- **Summary:** A position/audit paper arguing that accuracy- and return-based benchmarks give "an illusion of reliability" for LLM finance agents, masking vulnerabilities: **hallucinated facts, stale data, and adversarial prompt manipulation.** Proposes risk-first, multi-tier auditing (model / workflow / system level) and treating a "safety budget" as the primary success metric rather than performance.
- **Relevance to our system:** The governance/safety complement to the performance critique. For our advisor, an LLM in the loop (narration, news ingestion) introduces non-PnL risks our backtest gate does *not* catch: it can **hallucinate a "why this one" rationale, ingest stale/false news, or be manipulated by adversarial headlines.** Concrete implications: (1) any LLM narration must be *grounded* in the actual gated numbers (templated/constrained, not free-form invention); (2) any news/sentiment feed needs provenance + recency checks; (3) the LLM must never be on the decision path — it explains the gate's verdict, it doesn't make it. This paper justifies a hard architectural boundary: LLM = explanation/UX layer with its own safety checks; robustness gate = sole arbiter of what to trade.
