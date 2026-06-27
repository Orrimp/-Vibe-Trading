# Papers — LLMs (likelihood-computing neural nets) in Finance

Ledger for the `llms` topic. One entry per paper, appended immediately after
reading. Source of truth for resume. See `../README.md` for the contract.

Focus — LLMs as likelihood-computing neural nets in finance: LLM trading agents
(FinGPT, FinMem, FinAgent, TradingAgents); LLM financial sentiment / news;
multi-agent LLM trading frameworks; LLM reasoning for finance; financial LLM
benchmarks (BloombergGPT, FinBen, StockBench); retrieval-augmented finance
(RAG). **Special focus (operator's open question):** has anyone trained an
LLM / foundation model directly **on financial (numeric) time series** rather
than text? → the **time-series foundation model** literature (TimeGPT,
Lag-Llama, Chronos, Time-LLM, MOMENT, TimesFM, Moirai, "LLMs are zero-shot
time series forecasters") and whether it is applied to / critiqued on
financial/crypto data. That thread has its own labeled sub-section in
`knowledge.md`.

Skeptical lens (per task): where do LLMs *genuinely* add value (narration,
news/context, research assistance) vs. hype? Any LLM-derived signal is treated
as suspect until validated on genuinely out-of-sample (post-training-cutoff)
data, **net of costs**, against buy-and-hold — abstract NLP accuracy is not
tradeable edge.

> **Provenance:** entries [1]–[14] were **migrated** (2026-06-27) from the
> retired `llm-and-evolution/papers.md` — the LLM-side papers only, renumbered
> from [1]. The genetic/evolutionary papers from that ledger went to
> `evolution/`. New papers ([15]+) are added in subsequent rounds.

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

### [10] FinLlama: Financial Sentiment Classification for Algorithmic Trading Applications
- **Authors / Venue:** Thanos Konstantinidis, Giorgos Iacovides, et al. / ACM ICAIF 2024 (also arXiv)
- **Year:** 2024
- **Source:** arXiv:2403.12285
- **% read:** 40% (abstract + architecture; trading-backtest specifics not in the excerpt)
- **Summary:** A finance-tuned sentiment model: LoRA fine-tune of Llama-2-7B on 34,180 labeled financial-text samples across 4 datasets, with a generator-discriminator scheme that outputs both sentiment *polarity* (3-class) and *strength*, feeding a small NN decision mechanism. Claims the sentiment signal supports "enhanced portfolio management decisions and increased market returns."
- **Relevance to our system:** Representative of the "fine-tune a small open LLM for finance sentiment" recipe that is the realistic substrate for any sentiment feature we'd add (and for narration, à la FinGPT [1]). The *NLP* contribution (polarity + strength via LoRA) is credible and cheap. But the trading claim is unquantified in the abstract — **no stated transaction costs, no explicit B&H comparison, no post-cutoff OOS framing** — i.e. the same evidentiary gap as the rest of the LLM-trading literature ([2][3][4][9]). Take-away for us: FinLlama-style models are a fine *engineering* choice for a sentiment/narration component, but the "increased returns" claim must be re-proven inside our cost-aware, post-cutoff, bootstrap-vs-B&H gate before we believe it. NLP accuracy ≠ tradeable edge (theme echoing [5][7]).

### [11] From Deep Learning to LLMs: A Survey of AI in Quantitative Investment
- **Authors / Venue:** (quant-AI survey group) / arXiv
- **Year:** 2025
- **Source:** arXiv:2503.21422
- **% read:** 55% (full HTML skim of taxonomy + critical sections)
- **Summary:** A landscape survey organizing AI-in-quant across three eras (statistical → deep learning → LLM agents) along the pipeline Data → Prediction → Portfolio Opt → Execution. Notably *circumspect*: states LLM deployment "is still in its early stages," that LLM-for-portfolio-optimization yields "modest outcomes so far," that DL models risk overfitting and lack transparency, and that "model predictions themselves cannot be directly used as investment decisions." Makes **no claim** that AI/DL/LLM reliably beats passive benchmarks; it is a taxonomy, not a validation study.
- **Relevance to our system:** A useful authoritative, *neutral-skeptical* anchor citation. Even a broad survey of the whole field declines to assert AI beats passive, and explicitly flags overfitting, the prediction→decision gap, and LLM immaturity — all consistent with our thesis and our gate's reason to exist. Two reusable framings for our docs: (1) the four-stage pipeline (Data→Prediction→PortfolioOpt→Execution) is a clean way to scope where our advisor uses AI (mostly *not* in prediction); (2) its honest "modest outcomes so far" is the field's own verdict on LLM alpha — supports keeping LLMs on narration/context, not signal generation. Critique: the survey *under*-examines costs and the backtest-vs-live gap, which our gate foregrounds.

### [12] Alpha-GPT 2.0: Human-in-the-Loop AI for Quantitative Investment
- **Authors / Venue:** Hang Yuan, Saizhuo Wang, Jian Guo, et al. / arXiv (follows Alpha-GPT, arXiv:2308.00016)
- **Year:** 2024
- **Source:** arXiv:2402.09746
- **% read:** 35% (abstract + framework positioning; full empirical detail not in excerpt)
- **Summary:** A multi-agent LLM system spanning the quant pipeline — specialized agents for alpha *mining*, *modeling*, and *analysis*, fronted by an "AlphaBot" LLM layer that translates human research intent into structured tasks and uses RAG over a vector DB of financial literature + historical alphas. The explicit design philosophy is **Human-in-the-Loop**: AI accelerates and orchestrates, the human researcher steers and judges throughout, rather than autonomous alpha generation.
- **Relevance to our system:** The most aligned paper with our actual LLM posture. It frames the LLM as a **research assistant / orchestrator** — interpreting intent, retrieving relevant prior work, driving tools — with a human in the loop to catch nonsense. That is exactly the safe, value-adding role for LLMs in our advisor: narrate, retrieve context, help the *operator* reason, while the **robustness gate (not the LLM)** decides what's real. The human-in-the-loop is, in effect, a manual analogue of our automated gate against spurious alphas. Caveat: the paper still operates in the alpha-mining paradigm and the abstract doesn't quantify whether the human-in-the-loop measurably reduces false discoveries. Reusable design: RAG-over-financial-literature + intent-translation is a clean blueprint if we expand our "why this one" seam into an interactive research helper.

### [13] StockBench: Can LLM Agents Trade Stocks Profitably in Real-world Markets?
- **Authors / Venue:** (StockBench group) / arXiv
- **Year:** 2025
- **Source:** arXiv:2510.02209
- **% read:** 70% (abstract + setup + results via HTML v2)
- **Summary:** A *contamination-clean* reality check: 14 frontier LLMs (GPT-5, Claude-4-Sonnet, o3, DeepSeek-V3, Qwen3, Kimi-K2, …) trade the top-20 DJIA stocks over Mar 3–Jun 30 2025 — a window **deliberately after the models' knowledge cutoffs to prevent leakage**. Agents see prices, fundamentals, and recent news; **costs/slippage are NOT modeled (making trading easier than reality)**. Result: B&H returned 0.4% (MDD −15.2%). Best model Kimi-K2 1.9%; most clustered ~2–2.5%; two models lost money. Crucial nuance: "most models outperform B&H" only in aggregate over a roughly flat/up window — **during downturns ALL LLM agents underperform the passive baseline.** Strong financial-QA ability "does not necessarily translate into effective trading."
- **Relevance to our system:** The single best capstone for our thesis on the LLM side — and the most rigorous LLM-trading evaluation in the set (post-cutoff = no leakage). Even so, the "win" over B&H is tiny (≈1–2%), happens in a benign window *without costs*, and **completely inverts in downturns** — i.e. once you add our transaction costs and a 1000-path bootstrap spanning bad regimes, the apparent edge would almost certainly vanish or go negative. Directly validates: (a) keep LLMs off the alpha rail; (b) our gate's regime-spanning bootstrap + cost-awareness is exactly the right test (StockBench's downturn result is what a single benign backtest hides); (c) QA/benchmark skill ≠ trading skill (echoes [5][7][11]). This is the paper to cite when asked "but can't a frontier LLM just trade?"

### [14] Standard Benchmarks Fail — Auditing LLM Agents in Finance Must Prioritize Risk
- **Authors / Venue:** (LLM-finance-audit group) / arXiv
- **Year:** 2025
- **Source:** arXiv:2502.15865
- **% read:** 45% (abstract + argument + framework)
- **Summary:** A position/audit paper arguing that accuracy- and return-based benchmarks give "an illusion of reliability" for LLM finance agents, masking vulnerabilities: **hallucinated facts, stale data, and adversarial prompt manipulation.** Proposes risk-first, multi-tier auditing (model / workflow / system level) and treating a "safety budget" as the primary success metric rather than performance.
- **Relevance to our system:** The governance/safety complement to the performance critique. For our advisor, an LLM in the loop (narration, news ingestion) introduces non-PnL risks our backtest gate does *not* catch: it can **hallucinate a "why this one" rationale, ingest stale/false news, or be manipulated by adversarial headlines.** Concrete implications: (1) any LLM narration must be *grounded* in the actual gated numbers (templated/constrained, not free-form invention); (2) any news/sentiment feed needs provenance + recency checks; (3) the LLM must never be on the decision path — it explains the gate's verdict, it doesn't make it. This paper justifies a hard architectural boundary: LLM = explanation/UX layer with its own safety checks; robustness gate = sole arbiter of what to trade.

### [15] Chronos: Learning the Language of Time Series
- **Authors / Venue:** Abdul Fatir Ansari, Lorenzo Stella, Caner Turkmen, et al. (Amazon) / arXiv (TMLR 2024)
- **Year:** 2024
- **Source:** arXiv:2403.07815
- **% read:** 45% (abstract + method + benchmark framing via abstract page)
- **Summary:** A foundation model for time series that treats forecasting as *language modeling on numbers*: it scales and **quantizes** continuous time-series values into a fixed token vocabulary, then trains standard T5-family transformer LMs (20M–710M params) on those tokens via cross-entropy. Pretrained on a large collection of public datasets plus a **synthetic dataset generated from Gaussian processes** to improve generalization. On a 42-dataset benchmark, Chronos dominates in-corpus and is **comparable or occasionally superior zero-shot** on unseen datasets vs models trained specifically on them. Code + checkpoints open (amazon-science/chronos-forecasting).
- **Relevance to our system:** Directly addresses the operator's open question — yes, people *do* train transformer "LLM-architecture" models directly on **numeric** time series (not text). Chronos is the canonical example: a real, open, pretrained time-series foundation model. For our advisor the key honesty checks are: (a) the headline is *competitive*, not crushing — zero-shot it merely *matches* dataset-specific models, it doesn't reliably beat them; (b) the benchmark is general forecasting accuracy (WQL/MASE), **not** tradeable PnL net of costs vs buy-and-hold; (c) no financial/crypto evaluation in the paper. So Chronos is plausibly a *forecasting-input* worth a gated experiment on crypto, but "good zero-shot forecaster" is NOT "beats holding net of costs" — that remains for our 1000-path bootstrap-vs-B&H gate to decide. See knowledge.md special-focus section.

### [16] Large Language Models Are Zero-Shot Time Series Forecasters (LLMTime)
- **Authors / Venue:** Nate Gruver, Marc Finzi, Shikai Qiu, Andrew Gordon Wilson (NYU) / NeurIPS 2023
- **Year:** 2023
- **Source:** arXiv:2310.07820
- **% read:** 55% (abstract + method + why-it-works + caveats via abstract page)
- **Summary:** Shows that *general-purpose* LLMs (GPT-3, LLaMA-2), with **no fine-tuning**, can zero-shot extrapolate time series at a level "comparable to or exceeding" purpose-built models. The trick (LLMTime) is careful encoding of the series as a **string of numerical digits** with a tokenization scheme that turns the discrete token distribution into a flexible continuous density; reported higher likelihood/CRPS than ARIMA, TCN, N-HiTS in zero-shot. The authors attribute success to LLMs naturally representing multimodal distributions plus simplicity/repetition biases that match seasonal/periodic structure. Notable caveat: **GPT-4 *underperforms* GPT-3** here, blamed on its number tokenization and poor uncertainty calibration from RLHF/alignment.
- **Relevance to our system:** The seminal "text LLM can forecast numbers" result — important for the operator's question because it's the *opposite* approach to Chronos [15]: no numeric pretraining, just clever digit-encoding of a text LLM. But the caveats are exactly what matters to us: the win is on *likelihood/CRPS* on generic benchmarks (weather, traffic, etc.), **not** financial PnL; alignment/RLHF *hurts* (a warning that the frontier chat models we'd actually run are mis-calibrated for numbers); and the "simplicity + repetition + seasonality" mechanism is precisely what crypto price series *lack* (near-random-walk, weak seasonality). So this supports skepticism that LLM zero-shot forecasting transfers to crypto returns — a hypothesis to test through our gate, not assume.

### [17] Lag-Llama: Towards Foundation Models for Probabilistic Time Series Forecasting
- **Authors / Venue:** Kashif Rasul, Arjun Ashok, Andrew Robert Williams, et al. / arXiv (also TS-foundation-models site)
- **Year:** 2023 (v3 Feb 2024)
- **Source:** arXiv:2310.08278
- **% read:** 40% (abstract + architecture framing via abstract page)
- **Summary:** The **first open-source foundation model for time series forecasting**. A LLaMA-style **decoder-only transformer** for *univariate probabilistic* forecasting that uses **lagged values as covariates** (plus date-time features) and outputs a parametric (Student-t) predictive distribution. Pretrained on a large corpus of diverse time series spanning many domains (explicitly including finance among them). Reports strong zero-shot generalization across domains and, when fine-tuned on small fractions of unseen data, state-of-the-art results beating prior deep-learning methods. All data/models/code open.
- **Relevance to our system:** Another concrete "yes" to the operator's question — a foundation model trained directly on numeric series (lags), not text, and the paper lists *finance* among its pretraining/eval domains. For us the caveats are familiar: it is *univariate probabilistic forecasting* scored on distributional accuracy (CRPS), not PnL-vs-B&H-net-of-costs; "strong zero-shot" means competitive with, not dominant over, supervised models; and crypto returns are near-random-walk, where a probabilistic forecaster's best honest output is "wide distribution centered near no-change" — which is *exactly* what buy-and-hold already encodes. Worth a gated experiment as a forecast input, but no reason yet to expect it clears our gate.

### [18] A Decoder-Only Foundation Model for Time-Series Forecasting (TimesFM)
- **Authors / Venue:** Abhimanyu Das, Weihao Kong, Rajat Sen, Yichen Zhou (Google Research) / ICML 2024
- **Year:** 2023 (ICML 2024)
- **Source:** arXiv:2310.10688
- **% read:** 45% (abstract + method + results framing via abstract page + search summary)
- **Summary:** Google's time-series foundation model: a **patched-decoder** attention model (~200M params) pretrained on ~**100 billion real-world time-points**, inspired by LLMs but much smaller. Produces accurate forecasts **zero-shot** (no target fine-tuning) across varied history lengths, horizons, and granularities, with out-of-the-box accuracy that "comes close to" state-of-the-art *supervised* per-dataset models. Open-sourced (Apache 2.0); a 200M checkpoint on HuggingFace.
- **Relevance to our system:** The third major TSFM (with Chronos [15], Lag-Llama [17]) confirming the operator's question: large transformer models *are* trained directly on numeric series at scale. TimesFM's honest framing is again instructive — its selling point is *matching* supervised models zero-shot while being small/general, **not** beating them, and the benchmark is general forecasting accuracy, not trading. No finance/crypto evaluation in the paper. For our advisor: TimesFM is the most deployable (small, open, Apache-2.0) candidate if we ever want a zero-shot price forecaster to feed as a *gated input* — but "competitive zero-shot MASE on weather/traffic" gives zero assurance of edge on crypto returns net of costs. The financial-TSFM applications/critiques (see following entries) are the decisive evidence.

### [19] Foundation Time-Series AI Model for Realized Volatility Forecasting
- **Authors / Venue:** Anubha Goel, Puneet Pasricha, Martin Magris, Juho Kanniainen / arXiv
- **Year:** 2025
- **Source:** arXiv:2505.11163
- **% read:** 55% (abstract + method + results framing via abstract page)
- **Summary:** Directly applies a **time-series foundation model (TimesFM [18])** to a *financial* forecasting task — **realized volatility** — testing it both zero-shot and with a custom **incremental-learning fine-tune**, against econometric benchmarks (HAR/GARCH-family). Key result: the **pretrained zero-shot model is only "a reasonable baseline"**; the **fine-tuned** variants statistically outperform traditional models (Diebold-Mariano / Giacomini-White tests). The stated conclusion is that **incremental fine-tuning is *essential*** for the model to learn volatility patterns — zero-shot alone is not enough.
- **Relevance to our system:** One of the most decisive papers for the operator's open question. It is a *finance* application of a numeric-pretrained TSFM, and its honest finding is exactly the skeptical one: **zero-shot foundation-model forecasting of a financial series is merely OK; you must fine-tune (with care) to beat plain econometric baselines.** Two lessons for us: (a) don't expect an off-the-shelf TSFM to forecast crypto vol/returns better than simple models without adaptation; (b) even the *win* here is on **volatility** (a genuinely forecastable, persistent quantity) — *not* on directional return / PnL, which is the near-random-walk part that matters for beating buy-and-hold. Supports using such a model, if at all, for a *risk/vol-sizing* input (forecastable) rather than a return/alpha signal — and only after our cost-aware gate vets it.

### [20] Generalisation Bounds of Zero-Shot Economic Forecasting using Time Series Foundation Models
- **Authors / Venue:** Jittarin Jetwiriyanon, Teo Susnjak, Surangika Ranathunga / arXiv
- **Year:** 2025
- **Source:** arXiv:2506.15705
- **% read:** 55% (abstract + findings + caveats via abstract page)
- **Summary:** Evaluates three TSFMs — **Chronos [15], TimeGPT, and Moirai** — zero-shot on *economic* series, focusing on **data-scarce regimes and structural breaks.** Finds that "appropriately engineered" TSFMs can internalize economic dynamics and give well-behaved uncertainty out of the box, **matching or exceeding classical models under stable conditions** with no fine-tuning. The critical caveat: they show **"vulnerability to degradation during periods of rapid shocks"** — i.e. they break exactly when conditions shift abruptly. Verdict: zero-shot TSFM deployment is viable for *stable*-period monitoring but needs caution in volatile episodes.
- **Relevance to our system:** Reinforces [19] with a multi-model economic study and pins down the failure mode that matters most for crypto: **TSFMs are competitive in calm regimes and degrade during shocks/regime breaks.** Crypto is *dominated* by shocks and regime breaks — the precise condition under which these models are weakest, and the precise condition where beating buy-and-hold would actually be valuable (crash avoidance). So a zero-shot TSFM gives you accuracy when you don't need it (calm) and fails when you do (crash). Strong, finance-adjacent support for our thesis. Methodologically the "generalisation under structural breaks" framing maps onto our regime-spanning 1000-path bootstrap — both are ways to stop calm-period accuracy from masquerading as a robust edge.

### [21] MOMENT: A Family of Open Time-series Foundation Models
- **Authors / Venue:** Mononito Goswami, Konrad Szafer, Arjun Choudhry, et al. (CMU AutonLab) / ICML 2024
- **Year:** 2024
- **Source:** arXiv:2402.03885
- **% read:** 40% (abstract + framing via abstract page + search summary)
- **Summary:** A family of open foundation models for *general-purpose* time-series **analysis** (not just forecasting) — covering forecasting, classification, anomaly detection, and imputation. Contributes the **"Time Series Pile"** (a large, cohesive public TS corpus) to enable multi-dataset pretraining, plus a limited-supervision benchmark. The models work out-of-the-box with no/few task-specific exemplars (zero-shot forecasting, few-shot classification) and improve with task-specific fine-tuning. Pretrained models (MOMENT-1-large) and the Time Series Pile are open on HuggingFace.
- **Relevance to our system:** Broadens the operator's-question answer beyond forecasting: numeric-pretrained TS foundation models also do *anomaly detection* and *classification* — capabilities arguably more useful to a risk-aware advisor than point forecasting (e.g. flag an anomalous regime). But the same honesty applies: MOMENT's wins are on standard TS-analysis benchmarks with limited-supervision metrics, **not** trading PnL; no finance/crypto evaluation in the paper. For us the interesting seam is *anomaly/regime detection* on a crypto series as a possible *risk* signal (de-risk on anomaly), which would still have to survive our cost-aware gate — point-forecast accuracy is the part least likely to help against buy-and-hold.

### [22] Unified Training of Universal Time Series Forecasting Transformers (Moirai)
- **Authors / Venue:** Gerald Woo, Chenghao Liu, Akshat Kumar, Caiming Xiong, Silvio Savarese, Doyen Sahoo (Salesforce AI) / ICML 2024 (Oral)
- **Year:** 2024
- **Source:** arXiv:2402.02592
- **% read:** 45% (abstract + method + results framing via abstract page + search summary)
- **Summary:** Salesforce's **Moirai** — a **masked-encoder** Transformer for *universal* forecasting that handles **any number of variates** (multivariate), cross-frequency learning, and varying distributions, outputting a probabilistic forecast. Pretrained on **LOTSA**, a 27B-observation open archive across **nine domains** (which include finance/energy). Reports competitive-or-superior zero-shot performance vs full-shot (dataset-specific) models. Open code (uni2ts).
- **Relevance to our system:** The most *multivariate-capable* TSFM here, and LOTSA's nine domains include finance — so again the operator's answer is "yes, numeric-pretrained, including some financial data." Moirai is also one of the three models the economic-forecasting study [20] found *degrades during shocks*. The any-variate ability is theoretically attractive (we could feed BTC alongside on-chain/funding covariates), but: (a) probabilistic forecast accuracy ≠ tradeable edge net of costs; (b) [20]'s shock-degradation result applies directly to crypto; (c) a single-coin advisor rarely benefits from multivariate machinery. Worth knowing as the SOTA general TSFM; not evidence it beats holding on crypto.

### [23] Time-LLM: Time Series Forecasting by Reprogramming Large Language Models
- **Authors / Venue:** Ming Jin, Shiyu Wang, Lintao Ma, et al. / ICLR 2024
- **Year:** 2023 (ICLR 2024)
- **Source:** arXiv:2310.01728
- **% read:** 50% (abstract + method + results framing via abstract page + search summary)
- **Summary:** Rather than train on numbers from scratch, Time-LLM **reprograms a frozen text LLM** (Llama-7B / GPT-2 / BERT backbones, kept intact) for forecasting: input patches are mapped to **text-prototype representations** the LM can consume, and a **Prompt-as-Prefix (PaP)** injects declarative context (domain knowledge, task instructions) to steer the LM. Reports that the reprogrammed LLM excels at **few-shot and zero-shot** forecasting and *outperforms* specialized forecasting models on standard benchmarks.
- **Relevance to our system:** Important contrast in the operator's-question taxonomy: there are (a) numeric-pretrained TSFMs (Chronos/TimesFM/Lag-Llama/Moirai/MOMENT) and (b) **text-LLM-reprogramming** methods (Time-LLM, LLMTime [16]) that keep a language model's weights and adapt the *interface*. Time-LLM is the leading example of (b). The caveats for us are the strongest in the whole TSFM cluster: standard-benchmark wins on weather/traffic/electricity, **no finance/crypto eval**, and — per the "Are language models actually useful for time series?" critiques (logged below) — much of the apparent benefit of LLM-reprogramming **survives even when the LLM is ablated away**, suggesting the gains come from the surrounding architecture, not the language model. So Time-LLM is the *least* likely to give us anything for crypto. Catalog it as background; the ablation critiques are the load-bearing evidence.

### [24] TimeGPT-1
- **Authors / Venue:** Azul Garza, Cristian Challu, Max Mergenthaler-Canseco (Nixtla) / arXiv
- **Year:** 2023
- **Source:** arXiv:2310.03589
- **% read:** 40% (abstract + framing via abstract page + Nixtla/search summary)
- **Summary:** The first *commercial* time-series foundation model: an **encoder-decoder transformer** (positional encoding + multi-head attention) for **zero-shot** forecasting and anomaly detection, served via API. Per Nixtla it is trained on **>100B time-series data points across domains including finance, healthcare, weather, demographics, and IoT/transport.** Claims zero-shot inference beats established statistical, ML, and DL methods on performance, efficiency, and simplicity. (Closed weights; API/SaaS product.)
- **Relevance to our system:** Directly relevant to the operator's question — TimeGPT's training set *explicitly includes finance*, so the answer is unambiguously "yes, models have been trained on financial numeric series." But it's closed/SaaS (a poor fit for our local Rust advisor vs the open Chronos/TimesFM), and the headline is again *general* zero-shot accuracy, not crypto-trading PnL. Crucially TimeGPT is one of the models scrutinized in the independent benchmark critiques ([20] economic, and the "how foundational" study logged below): those find it competitive in stable regimes but *not* a reliable winner, especially under shocks. So: a real, finance-trained TSFM exists and is purchasable, but there is no gate-credible evidence it beats buy-and-hold on crypto net of costs — and being closed-source it's the wrong substrate for us anyway.

### [25] Are Language Models Actually Useful for Time Series Forecasting?
- **Authors / Venue:** Mingtian Tan, Mike A. Merrill, Vinayak Gupta, Tim Althoff, Thomas Hartvigsen / NeurIPS 2024 (Spotlight)
- **Year:** 2024
- **Source:** arXiv:2406.16964
- **% read:** 60% (abstract + ablation design + headline results via abstract page + search summary)
- **Summary:** A rigorous **ablation** of three popular LLM-for-time-series methods (incl. Time-LLM-style reprogramming). Devastating headline: **removing the LLM component — or replacing it with a trivial attention layer — does NOT degrade forecasting; in most cases it *improves*.** Pretrained LLMs do no better than from-scratch models, do not represent sequential dependencies, and do not help in few-shot settings. Plain patching + attention matches LLM-based forecasters while being **up to 3 orders of magnitude cheaper** in train/inference time.
- **Relevance to our system:** The single most important skeptic paper for the operator's question, on the *text-LLM-reprogramming* branch. It directly debunks the premise that a **language** model contributes anything to numeric forecasting: the gains attributed to LLMs come from the surrounding patch/attention machinery, not the language prior. For our advisor this means: (1) the "reprogram a chat LLM to forecast crypto" idea (Time-LLM [23], LSTPrompt) is almost certainly not worth the cost/complexity — a small dedicated model is cheaper and as good; (2) it sharpens the distinction between *purpose-built numeric TSFMs* (Chronos/TimesFM — these at least learn from numbers) and *text-LLM repurposing* (debunked here). Strongly reinforces keeping LLMs on the *language* jobs (narration/news) and **not** the forecasting/decision rail.

### [26] How Foundational are Foundation Models for Time Series Forecasting?
- **Authors / Venue:** Nouha Karaouli, Denis Coquenet, Elisa Fromont, Martial Mermillod, Marina Reyboz / NeurIPS 2025 Workshop (TS Foundation Models)
- **Year:** 2025
- **Source:** arXiv:2510.00742
- **% read:** 50% (abstract + critical findings via abstract page)
- **Summary:** A critique of how "foundational" TSFMs really are. Two key findings: (1) a TSFM's **zero-shot ability is tightly tied to the domains it was pretrained on** (it generalizes far less than the "foundation model" branding implies); (2) on genuinely unseen data, even **fine-tuned** TSFMs **do not consistently beat smaller dedicated models** once you account for their far larger parameter count / memory. Implication: TSFMs underperform under distribution shift and offer no clear advantage for their cost.
- **Relevance to our system:** The purpose-built-TSFM counterpart to [25]'s LLM-reprogramming debunk — together they bracket the whole field. The "zero-shot is tied to pretraining domains" point is decisive for crypto: unless a TSFM was heavily pretrained on *crypto-like* high-vol, fat-tailed, regime-switching data, its zero-shot crypto forecasts will be weak — and [20] already showed TSFMs degrade under shocks, which dominate crypto. Bottom line for the operator's question: foundation models for numeric series **exist and are real**, but the independent evidence ([20][25][26]) is that they are *competitive-not-dominant*, *domain-bound*, *shock-fragile*, and *not* cost-justified over small dedicated models — so there is no credible basis to expect one to beat buy-and-hold on crypto net of costs. Our gate would be the arbiter, but the prior is strongly negative.

### [27] Re(Visiting) Time Series Foundation Models in Finance
- **Authors / Venue:** Eghbal Rahimikia, Hao Ni, Weiguan Wang / arXiv
- **Year:** 2025
- **Source:** arXiv:2511.18578
- **% read:** 55% (abstract + key findings via abstract page + search summary)
- **Summary:** Billed as the **first comprehensive empirical study of TSFMs in global financial markets**, on a large-scale dataset of **daily excess returns across diverse markets.** The central, stark finding: **off-the-shelf generic pretrained TSFMs perform *poorly* in finance — in both zero-shot AND fine-tuning settings — whereas models pretrained *from scratch on financial data* achieve substantial forecasting *and economic* improvements.** Further gains come from larger data, synthetic augmentation, and hyperparameter tuning. Frames finance data as noisy, non-stationary, heterogeneous.
- **Relevance to our system:** Possibly the single most direct answer to the operator's question. It says, with the biggest financial sample to date: the famous general-purpose TSFMs (Chronos/TimesFM/Moirai/etc.) **do NOT transfer to finance off-the-shelf** — you have to *retrain from scratch on financial data* to get an edge. For our advisor that is a double-edged result: (a) it confirms generic zero-shot TSFM forecasting of crypto is a dead end (consistent with [20][26]); (b) it leaves open that a *finance-specialized* from-scratch model *might* help — but building/retraining one is far beyond a retail single-coin advisor's scope, and the paper's "economic improvements" are still measured as forecasting/portfolio metrics on equities, not as crypto PnL surviving a 1000-path cost-aware bootstrap-vs-B&H gate. Net: don't expect off-the-shelf TSFMs to work on crypto; a bespoke financial TSFM is out of scope and unproven on our exact test.

### [28] Pretrained Time-Series Foundation Models for Financial Return Forecasting
- **Authors / Venue:** Miquel Noguer i Alonso, Rodolfo Pereira Franklin / arXiv
- **Year:** 2026
- **Source:** arXiv:2606.27100
- **% read:** 60% (abstract + setup + DM-test results + conclusion via abstract page + search summary)
- **Summary:** A careful head-to-head of **six TSFMs** (TimeGPT / TimeGPT-LH, TimesFM-2.5, Moirai-2.0, Chronos, Chronos-2) vs **five deep baselines** (NBEATS, NHITS, PatchTST, iTransformer, KAN) on **financial *return* forecasting** for 5 liquid US equities (AAPL, AMZN, GOOG, JPM, META). Explicitly motivates return forecasting as a *hard* TSFM test case (low signal-to-noise, structural breaks, heavy tails, weak persistence). Result: **gains over the random-walk benchmark are "small and sparse"** — Diebold-Mariano rejects equal/inferior accuracy only for **Chronos on AMZN and Moirai-2.0 on GOOG** (i.e. 2 of many model-asset pairs). Conclusion: TSFMs are "useful practical priors that reduce model-development cost in low-data forecasting, but are **not universal engines for statistically reliable alpha generation.**"
- **Relevance to our system:** The most quantitatively decisive entry for the operator's question. On the *exact* quantity that matters for beating buy-and-hold — **financial returns** (not vol, not weather) — the best TSFMs barely beat a **random walk**, and only on 2 of ~30 model-asset combinations with statistical significance. A random walk is, for returns, essentially the buy-and-hold null. This is direct, gate-adjacent evidence that **TSFMs do not produce reliable return-forecasting alpha** even on liquid large-caps (crypto, with worse SNR and heavier tails, would be worse). The authors' own framing — "useful priors, not alpha engines" — is precisely our posture. Strongest single citation for "no, an off-the-shelf TSFM will not beat holding on returns."

### [29] FinCast: A Foundation Model for Financial Time-Series Forecasting
- **Authors / Venue:** Zhuohang Zhu, Haodong Chen, Qiang Qu, Vera Chung / arXiv
- **Year:** 2025
- **Source:** arXiv:2508.19609
- **% read:** 45% (abstract + framing via abstract page)
- **Summary:** A foundation model **purpose-built for *financial* time series** (the constructive answer to "generic TSFMs don't transfer" [27]). Trained on large-scale financial datasets across **stocks, commodities, and futures** and multiple resolutions (per-second to weekly), designed to handle financial pattern-shifts without heavy domain fine-tuning. Reports **robust zero-shot performance** that "surpasses existing state-of-the-art methods" with strong generalization. (Abstract does not break out return vs price-level vs volatility targets, nor mention crypto explicitly.)
- **Relevance to our system:** The constructive counterpoint within the operator's question — yes, someone has built a foundation model *specifically on financial numeric series* (echoing [27]'s "train from scratch on financial data"). It's the most directly on-point artifact. But two skeptical flags for us: (a) the abstract's "surpasses SOTA / robust zero-shot" claim does **not** separate *return* forecasting (the hard, alpha-relevant target where [28] shows TSFMs barely beat a random walk) from *price-level/volatility* forecasting (easier, less tradeable) — a distinction that decides whether it could ever beat B&H; (b) no crypto eval and no cost-aware PnL-vs-B&H test. So FinCast establishes that finance-specialized TSFMs exist and claim good general financial forecasting, but provides no gate-credible evidence of crypto return-alpha. A model to watch, not adopt.

### [30] Profit Mirage: Revisiting Information Leakage in LLM-based Financial Agents
- **Authors / Venue:** Xiangyu Li, Yawen Zeng, Xiaofen Xing, Jin Xu, Xiangmin Xu / arXiv
- **Year:** 2025
- **Source:** arXiv:2510.07920
- **% read:** 55% (abstract + method + findings via abstract page)
- **Summary:** A direct attack on the credibility of LLM-trading backtests. Defines **information leakage** as the LLM having *memorized* historical data + outcomes within its training window, so impressive backtest returns are a **"profit mirage"** that **collapses once you test beyond the training cutoff.** Builds **FinLake-Bench**, a leakage-robust benchmark quantifying the effect across four dimensions, and shows substantial performance decay attributable to memorization rather than genuine prediction. Proposes **FactFin** (counterfactual perturbations + RAG + MCTS + counterfactual simulator) to force causal learning and improve out-of-sample generalization.
- **Relevance to our system:** Direct, named, quantified support for the leakage critique we apply to every LLM-trading paper ([2][3][4]) — and the methodological complement to StockBench [13] (post-cutoff testing). The "profit mirage" framing is exactly the failure our gate and post-cutoff discipline exist to expose: reported LLM-agent profits are largely memorization, not edge. Two concrete takeaways: (1) any LLM signal we ever consider must be evaluated *strictly post-training-cutoff* (or with counterfactual perturbation), or its backtest is worthless; (2) the counterfactual-perturbation idea (does the signal survive when you scramble the memorizable specifics?) is a clever overfitting/leakage detector worth knowing alongside our bootstrap. Strong citation for "LLM-agent trading profits are an artifact."

### [31] CryptoTrade: A Reflective LLM-based Agent to Guide Zero-shot Cryptocurrency Trading
- **Authors / Venue:** Yuan Li, Bingqiao Luo, Qian Wang, Nuo Chen, Xu Liu, Bingsheng He / EMNLP 2024 (main)
- **Year:** 2024
- **Source:** arXiv:2407.09546
- **% read:** 50% (abstract + method via abstract page + search summary; ACL anthology)
- **Summary:** The canonical **crypto-specific** LLM trading agent. Uniquely combines **on-chain** data (transparent, immutable blockchain signals) with **off-chain** data (news), plus a **reflective mechanism** that refines daily decisions by analyzing prior-decision outcomes. Tested zero-shot across various cryptocurrencies and market conditions vs traditional strategies and time-series baselines; claims "superior performance in maximizing returns." Establishes a crypto-trading benchmark. (Abstract gives no numbers, no cost treatment, no explicit limitations.)
- **Relevance to our system:** The most directly on-topic LLM-agent paper for us (crypto, on-chain + off-chain) — but it inherits every evidentiary gap of the LLM-agent genre: no transaction-cost modeling in the abstract, no clear post-cutoff/leakage control (a GPT-class model's cutoff likely overlaps the test window — exactly the "profit mirage" [30]), and "superior returns" stated without a regime-spanning, cost-aware, B&H-net comparison. The **on-chain-data** idea is the genuinely crypto-native contribution worth noting (on-chain flows are a real exogenous feature class), but the agent's trading claims are not gate-credible. For us: catalog CryptoTrade as the reference crypto-LLM-agent and a source of *feature ideas* (on-chain signals), not as evidence LLMs beat holding BTC.

### [32] An Adaptive Multi-Agent Bitcoin Trading System
- **Authors / Venue:** Aadi Singhi / arXiv
- **Year:** 2025
- **Source:** arXiv:2510.08068
- **% read:** 50% (abstract + design + results via abstract page)
- **Summary:** A multi-agent LLM Bitcoin trader with four roles (technical, sentiment, decision, reflection) and a **verbal-feedback** loop: a Reflect agent writes daily/weekly natural-language critiques that are injected into future prompts, so the system "learns" without any weight updates. Backtested on BTC **Jul-2024 → Apr-2025**, claiming >30% higher returns in bullish phases and **+15% overall vs buy-and-hold**, a sentiment agent turning sideways markets into +100%, and weekly feedback adding +31%.
- **Relevance to our system:** Crypto, single-coin (BTC) — superficially our exact setting — but a textbook of the failure modes our gate rejects: **transaction costs not addressed**, a **single ~10-month window** (which included a strong BTC bull run — so beating B&H "in bullish phases" is partly just exposure), and a **GPT-class cutoff that overlaps the 2024–25 test period** (leakage / "profit mirage" [30]). The "+100% from a sentiment agent in a sideways market" claim with no cost accounting is a red flag, not a result. The verbal-reflection-without-fine-tuning mechanism is a mildly interesting *agent-design* idea, but the trading numbers are not credible evidence. For us: a direct illustration that even *crypto single-coin* LLM systems collapse under cost-aware, regime-spanning, post-cutoff scrutiny — i.e. our gate's exact remit. Reinforces the thesis on our own asset class.

### [33] Large Language Model Agent in Financial Trading: A Survey
- **Authors / Venue:** Han Ding, Yinheng Li, Junhao Wang, Hang Chen, Doudou Guo, Yunbai Zhang / arXiv
- **Year:** 2024
- **Source:** arXiv:2408.06361
- **% read:** 40% (abstract + taxonomy framing via abstract page)
- **Summary:** A survey organizing the LLM-trading-agent field along three axes: **agent architectures**, **data inputs**, and **backtesting/performance evaluation.** It frames the open question as whether LLM agents "can outperform professional traders," reviews the common designs (single-agent, multi-agent, reflective/memory), and outlines challenges + future directions — but **declines to declare LLM agents mature or consistently profitable**, positioning the field as still developmental.
- **Relevance to our system:** A neutral anchor/citation for the LLM-trading-agent landscape (complements the broader DL→LLM survey [11]). Its refusal to claim profitability — despite surveying the very papers that *report* big returns — is itself telling: the field's own survey treats out-performance as an open question, not an established fact. Useful for our docs as the "here is the map of LLM-agent designs" reference, while our gate supplies the verdict the survey withholds. Reusable: its architecture/data taxonomy is a clean way to describe *where* an LLM could sit in our advisor (data ingestion / narration), explicitly not as a validated alpha source.

### [34] LLM-Powered Multi-Agent System for Automated Crypto Portfolio Management
- **Authors / Venue:** Yichen Luo, Yebo Feng, Jiahua Xu, Paolo Tasca, Yang Liu / arXiv
- **Year:** 2025
- **Source:** arXiv:2501.00826
- **% read:** 55% (abstract + design + results via abstract page)
- **Summary:** A multi-agent LLM system for **crypto portfolio** management: a Crypto Agent (market dynamics), News Agent (weekly news sentiment), and Trading Agent (fuses + executes), tested under hierarchical/collaborative/debate communication and zero-shot/CoT/RAG/skill-augmented configs. Inputs fuse **price + on-chain time series + news + technical indicators.** On a **52-week 2025 backtest** over the **top-15 L1 cryptos**, the best (Hierarchical-Skill) config reports **133.52% cumulative return, Sharpe 1.50**, beating single-agent, passive benchmarks, and DL baselines; ablating the Crypto Agent costs 42.6 pts.
- **Relevance to our system:** Closer to a *portfolio* than our single-coin advisor, but crypto-native and instructive. Same red flags: **no transaction-cost/slippage modeling**, a **single 52-week 2025 window** (and 2025 was a strong crypto year — a passive top-15 basket also did well, so "beats passive" needs the *net-of-cost, regime-spanning* test), and **cutoff/leakage** unaddressed ("profit mirage" [30]). The genuinely useful bits for us: (a) **on-chain + news + price fusion** as a feature-architecture sketch; (b) the finding that the *market-analysis* agent (numbers) matters far more than the news agent (the 42.6-pt ablation) — consistent with our thesis that *price/structure* dominates *text*. But 133% with no costs over one bull year is not gate-credible alpha. Catalog as design/feature inspiration, not evidence.

### [35] FinBERT: Financial Sentiment Analysis with Pre-trained Language Models
- **Authors / Venue:** Dogu Araci / arXiv (MSc thesis, Univ. of Amsterdam)
- **Year:** 2019
- **Source:** arXiv:1908.10063
- **% read:** 40% (abstract + method via abstract page + search summary)
- **Summary:** The foundational domain-adapted financial NLP model: take **BERT**, *further pre-train* it on a large financial corpus, then fine-tune for **sentiment classification.** Evaluated on Financial PhraseBank and FiQA; reports state-of-the-art on every metric, and shows domain pre-training lets it win **even with a small labeled set and only partial fine-tuning.** (Pure NLP; no trading application in the paper.) The ProsusAI/finbert checkpoint became a widely-used default.
- **Relevance to our system:** The canonical, cheap, *open* financial-sentiment substrate — the realistic baseline for any sentiment feature in our advisor (predates and is lighter than the LLM-era FinGPT [1]/FinLlama [10]). Its honest scope is the point: FinBERT proves you can *classify financial-text sentiment* well, full stop — it makes **no** trading claim. For us it's a ready, encoder-sized tool if we ever add a sentiment *input*, but exactly like the bigger models, its NLP accuracy says nothing about beating buy-and-hold; that requires our cost-aware, post-cutoff, bootstrap-vs-B&H gate. Good "minimal viable sentiment model" reference.

### [36] PIXIU: A Large Language Model, Instruction Data and Evaluation Benchmark for Finance (FinMA / FLARE)
- **Authors / Venue:** Qianqian Xie, Weiguang Han, Xiao Zhang, Yanzhao Lai, Min Peng, Alejandro Lopez-Lira, Jimin Huang / NeurIPS 2023 Datasets & Benchmarks
- **Year:** 2023
- **Source:** arXiv:2306.05443
- **% read:** 40% (abstract + framework via abstract page + search summary)
- **Summary:** A comprehensive open finance-LLM framework: (1) **FinMA**, a LLaMA fine-tuned on (2) **136K** multi-task financial **instruction** samples, evaluated on (3) **FLARE**, a benchmark of 5 financial NLP tasks + 1 financial *prediction* task across 9 datasets (incl. a stock-movement-prediction task). Spans multiple financial document types and modalities. Predecessor/sibling to FinBen [7] from an overlapping group.
- **Relevance to our system:** Important because it's an early benchmark that *includes a numeric financial-prediction (stock-movement) task alongside text tasks* — and, consistent with the later FinBen [7] and StockBench [13] findings, the pattern is that instruction-tuned finance LLMs do well on the *NLP* tasks while the *prediction* task is the weak spot (the broader literature, [7][8], pegs such directional prediction near coin-flip). For us PIXIU/FinMA is (a) a reusable *open* instruction-tuned finance model for text jobs (narration/extraction), and (b) more evidence that adding a "prediction" task to a finance-LLM benchmark exposes weakness, not strength, on the numeric side. NLP-capable, not alpha-capable.

### [37] StockTime: A Time Series Specialized Large Language Model Architecture for Stock Price Prediction
- **Authors / Venue:** Shengkun Wang, Taoran Ji, Linhan Wang, et al. / arXiv (CIKM 2024)
- **Year:** 2024
- **Source:** arXiv:2409.08281
- **% read:** 45% (abstract + architecture via abstract page)
- **Summary:** An LLM architecture specialized for **stock-price time series**: it treats **stock prices as consecutive tokens**, auto-derives textual descriptors (correlations, statistical trends, timestamps) from the price data itself, fuses text + numeric series in a shared embedding space, and predicts autoregressively. Claims to **outperform recent LLM-for-TS methods** with lower memory/runtime. Notably concedes models "produce less accurate predictions when faced with false and redundant information in financial markets" (i.e. noise hurts).
- **Relevance to our system:** Another data point on the operator's question — an LLM adapted to *financial* numeric series (prices-as-tokens), purpose-built for stocks. But the honesty checks: its claimed wins are *relative to other LLM-for-TS methods* (a low bar, given [25] shows the LLM often ablates out) and on prediction-error metrics, **not** tradeable PnL vs B&H net of costs; no crypto eval; and it explicitly admits noise degrades it — and crypto is noisier than equities. So StockTime joins FinCast [29]/StockLLM [8] as "yes, people specialize LLMs on financial series," while adding nothing that suggests crypto return-alpha over holding. The "noise hurts" admission is itself supportive of our thesis (the un-forecastable part dominates). Background/catalog.

### [38] A Review of Large Language Models for Stock Price Forecasting from a Hedge-Fund Perspective
- **Authors / Venue:** Olivia Zhang, Zhilin Zhang / arXiv (accepted IEEE Conf. on AI 2026)
- **Year:** 2026
- **Source:** arXiv:2605.05211
- **% read:** 45% (abstract + pitfalls + assessment via abstract page)
- **Summary:** A practitioner-oriented review of LLMs for stock-price forecasting that is refreshingly **cautionary**. It surveys the usual applications (news/social sentiment, report/earnings-call analysis, price-series tokenization, multi-agent systems) but foregrounds the **practical pitfalls the literature understates**: fragility in sentiment analysis, improper dataset/horizon design, flawed evaluation metrics, **data leakage**, **illiquidity premia**, and the fundamental **limits of stock-price predictability.** Its core recommendation: **stress-test robustness under realistic market frictions before deploying.**
- **Relevance to our system:** Almost a mission statement for our advisor's gate, from the buy-side. The recommended discipline — stress-test under realistic frictions, beware leakage, beware illiquidity, respect predictability limits — is precisely what our 1000-path moving-block bootstrap *with costs* against buy-and-hold operationalizes. It's the ideal neutral citation that the *field's own practitioner reviews* converge on skepticism + frictions-aware testing, not "LLMs forecast prices." Its pitfall list also maps cleanly onto our standing critiques across [2][3][4][30][31][32]. Strong anchor for "even the hedge-fund-perspective review says: test under frictions, don't assume edge."

### [39] Sentiment Trading with Large Language Models
- **Authors / Venue:** Kemal Kirtac, Guido Germano / Finance Research Letters 2024 (also arXiv)
- **Year:** 2024
- **Source:** arXiv:2412.19245
- **% read:** 55% (abstract + results via abstract page + search summary)
- **Summary:** A large, peer-reviewed sentiment-trading study on **965,375 US news articles (Jan-2010 → Jun-2023)**, comparing **OPT (GPT-3-class), BERT, FinBERT, and the Loughran-McDonald dictionary.** OPT wins on sentiment accuracy (74.4% vs ~72% BERT/FinBERT vs 50.1% L-M). A daily **long-short** portfolio on OPT scores reports a striking **Sharpe 3.05** (vs 2.11 BERT, 2.07 FinBERT, 1.23 L-M) and a **+355% gain Aug-2021 → Jul-2023.** Concludes LLM sentiment is significantly associated with subsequent daily returns.
- **Relevance to our system:** The strongest *peer-reviewed pro-LLM-sentiment* result in the ledger — and precisely because it's strong, the unstated caveats are what matter for us. The abstract **does not model transaction costs** and **does not address that OPT/GPT-3's training window overlaps the 2010–2021 sample** (look-ahead/leakage, exactly [6][30]). A long-short *cross-sectional* equity strategy with daily rebalancing is also turnover-heavy and **inapplicable to a single-coin advisor** (we can't go long-short across names). And a Sharpe of 3 from daily news sentiment is the kind of number that historically shrinks hard under realistic costs + post-cutoff testing (cf. Lopez-Lira [9]'s decay finding). So: real evidence that LLM sentiment *correlates* with returns, but not gate-credible tradeable alpha, and structurally not portable to our setup. Reinforces "sentiment = possible exogenous feature, tested net-of-cost post-cutoff; never standalone alpha."

### [40] Fin-R1: A Large Language Model for Financial Reasoning through Reinforcement Learning
- **Authors / Venue:** Zhaowei Liu, Xin Guo, Zhi Yang, et al. (SUFE / Fudan / Rice / LSE) / arXiv
- **Year:** 2025 (v5 2026)
- **Source:** arXiv:2503.16252
- **% read:** 45% (abstract + method + benchmarks via HTML)
- **Summary:** A compact **7B financial-reasoning LLM** built via a two-stage pipeline: SFT on 60K curated **chain-of-thought** financial samples, then **GRPO** RL with format+accuracy rewards. Targets financial *reasoning/QA* benchmarks (FinQA, ConvFinQA, Ant-Finance, TFNS, Finance-Instruct-500k); scores 75.2 avg, second overall, beating all similar-size models and within 3 pts of DeepSeek-R1. **Explicitly makes no trading/forecasting claim** — positioned for compliance checking and robo-advisory *consultation*.
- **Relevance to our system:** A clean, current example of the *right* LLM scope for finance — and a useful contrast to the trading-agent hype. Fin-R1 invests heavily in *reasoning* (CoT + RL) yet deliberately stays on **interpretable QA / advisory consultation**, not price prediction. That is exactly our posture: an LLM can *reason about and explain* financial questions (a strong fit for our "why this one" narration / operator-consultation seam) while the decision/alpha stays with the gate. Also relevant: it shows a small (7B), open-ish reasoning model can be competitive — a realistic substrate for a local narration/explanation layer, no frontier API required. Catalog as: best-practice LLM-for-finance scoping (reasoning/advisory, not trading).

### [41] FinRL-DeepSeek: LLM-Infused Risk-Sensitive Reinforcement Learning for Trading Agents
- **Authors / Venue:** Mostapha Benhenda (LAGA) / arXiv
- **Year:** 2025
- **Source:** arXiv:2502.07393
- **% read:** 45% (abstract + method via abstract page)
- **Summary:** A hybrid that **fuses an LLM signal into an RL trader**: it extends the **CVaR-PPO (CPPO)** risk-sensitive RL algorithm with **risk-assessment + trading-recommendation signals an LLM extracts from financial news.** Tested on the **Nasdaq-100** using the FNSPID news dataset, with three LLMs (DeepSeek-V3, Qwen-2.5, Llama-3.3). Open code/data/agents. (Abstract gives no concrete backtest numbers, cost treatment, or limitations.)
- **Relevance to our system:** A representative *LLM-as-feature-into-RL* design — the LLM produces a news-derived risk/recommendation signal that an RL policy consumes, rather than the LLM trading directly. Architecturally that's the safer pattern (LLM = exogenous signal, not decision-maker). But it stacks two of our least-trusted components (RL trader + LLM news signal), inherits leakage risk (news + cutoff overlap, [30]), and the abstract reports no costs or B&H-net comparison — so no gate-credible evidence. The genuinely useful idea for us is the *risk-sensitive* framing (CVaR objective): if we ever used an exogenous LLM/news signal, wiring it to **risk/sizing** (downside control) rather than directional alpha is the defensible target — consistent with the realized-vol finding [19] and the "B&H + strategy reduces drawdown" pattern. Catalog as design reference for LLM-signal-as-risk-input.

### [42] Market-Derived Financial Sentiment Analysis: Context-Aware Language Models for Crypto Forecasting
- **Authors / Venue:** Hamid Moradi-Kamali, Mohammad-Hossein Rajabi-Ghozlou, Mahdi Ghazavi, et al. / arXiv
- **Year:** 2025
- **Source:** arXiv:2502.14897
- **% read:** 55% (abstract + method + results via abstract page)
- **Summary:** A **crypto-specific** sentiment-forecasting model. Instead of human sentiment labels, it uses **market-derived labels — each tweet is labeled by the *ensuing short-term price move*** — then prompt-tunes a domain LM with market + temporal context. On Bitcoin tweets + 227 curated news events: +11% short-term trend-prediction accuracy over sentiment baselines, **89.6% accuracy** on the curated BTC news set, and **Sharpe up to 5.07** (trending) / 3.73 (neutral). Claims to overturn the assumption that price signals beat sentiment signals.
- **Relevance to our system:** Crypto + Bitcoin — our exact asset — but a near-textbook illustration of the traps our gate exists for. (1) **The labeling scheme — "label the tweet by what the price did next" — bakes the target into the features**; high accuracy then partly measures the label leak, not predictive power. (2) **No transaction costs**, and a Sharpe of 5 on short-term BTC signals is wildly implausible net of crypto fees/slippage. (3) "Curated impactful news events" is selection on outcome. (4) Single-regime-flavored evaluation. So despite being on-topic, its headline numbers are not gate-credible. The one *defensible* idea is *market-derived labeling* as a way to build a sentiment dataset — but it must then be tested on a *separate*, post-event, cost-aware, bootstrap-vs-B&H protocol, not on the same trend-defined labels. Strong example for the operator that "crypto + LLM + sentiment + big Sharpe" usually means leakage + no costs, not edge.

### [43] Time Series Foundation Models for Multivariate Financial Time Series Forecasting
- **Authors / Venue:** Ben A. Marconi / arXiv
- **Year:** 2025
- **Source:** arXiv:2507.07296
- **% read:** 55% (abstract + results + conclusion via abstract page)
- **Summary:** Evaluates two TSFMs — **Tiny Time Mixers (TTM)** and **Chronos [15]** — on three *financial* forecasting tasks: **US 10-yr Treasury yield changes, EUR/USD volatility, equity spread.** Findings are carefully mixed: pretrained TTM beats its un-pretrained version and needs **3–10 fewer years of data** for comparable performance, with **25–50% better** results when fine-tuned on limited data; zero-shot, TTM **beats naive benchmarks on volatility and equity-spread** prediction. **But traditional specialized models still matched or surpassed TTM on 2 of 3 tasks**, and the author concludes that competitive financial results will need **domain-specific pretraining + architecture tailored to financial series.**
- **Relevance to our system:** A balanced, finance-applied TSFM study that lands exactly where the operator's-question evidence converges: TSFMs help most in **noisy, data-constrained** settings and on **volatility/spread** (forecastable targets) — and even then **lose to specialized models on most tasks**, requiring finance-specific pretraining ([27]) to compete. Note the tasks where TSFM *did* beat naive are **volatility and a spread** — *not* directional price/return (echoing [19][28]: the forecastable financial quantity is vol, not return). For us: more support that (a) any TSFM value is in *risk/vol* estimation, not return-alpha; (b) off-the-shelf is not enough; (c) data-efficiency (fewer years needed) is the genuine practical upside — relevant if we ever had short crypto histories, but still gated.

### [44] LLM Agents Do Not Replicate Human Market Traders: Evidence from Experimental Finance
- **Authors / Venue:** Thomas Henning, Siddhartha M. Ojha, Ross Spoon, Jiatong Han, Colin F. Camerer (Caltech et al.) / arXiv
- **Year:** 2025
- **Source:** arXiv:2502.15800
- **% read:** 50% (abstract + setup + findings via abstract page)
- **Summary:** Puts LLM agents into a **classic experimental-finance asset-market paradigm** (traders buy/sell a risky asset with a *known* fundamental value), in single-model and mixed-model markets. Key finding: LLM agents are **"textbook-rational"** — they price near fundamental value and show only a **muted tendency to form bubbles**, with less strategy variance than humans. Conclusion: LLM-only populations **fail to reproduce human market phenomena (bubbles, crashes)** and cannot substitute for human traders in modeling real market dynamics.
- **Relevance to our system:** A distinctive, non-PnL caveat with two implications for our advisor. (1) **For market *simulation*:** if we ever considered LLM agents to generate synthetic market scenarios or stress tests, this says they'd produce *unrealistically calm, rational* dynamics — missing exactly the bubble/crash regimes our 1000-path moving-block bootstrap must span (so our resampling-from-real-data approach is the right one, not LLM-simulated paths). (2) **For trading behavior:** "textbook-rational, near fundamental value, muted bubbles" implies an LLM trader would tend toward... holding near fair value — i.e. it has no special crash-timing or momentum-riding edge, consistent with "no robust edge over buy-and-hold." Useful, original citation on the *limits* of LLMs as market participants/simulators.

### [45] Agentic Retrieval-Augmented Generation for Financial Document Question Answering (FinAgent-RAG)
- **Authors / Venue:** Yang Shu, Yingmin Liu, Zequn Xie / Expert Systems with Applications (submitted)
- **Year:** 2026
- **Source:** arXiv:2605.05409
- **% read:** 50% (abstract + method + results via abstract page)
- **Summary:** An *agentic* RAG framework for QA over financial filings that require multi-step numeric reasoning over tables, narrative text, and footnotes. Three parts: a **Contrastive Financial Retriever** (hard-negative mining), a **Program-of-Thought** module that emits executable Python for arithmetic (so the model computes rather than guesses numbers), and an **Adaptive Strategy Router** that scales compute to question difficulty. On FinQA / ConvFinQA / TAT-QA it reaches 76.8% / 78.5% / 75.0% execution accuracy (+5.6–9.3 pts over the strongest baseline) and cuts API cost 41.3% on FinQA. Strictly a **document-QA accuracy benchmark — no trading, no PnL, no costs/slippage in the market sense.**
- **Relevance to our system:** Adds RAG-for-finance *depth* (a flagged thin seam) and is squarely on the *language* side of our thesis split — it makes financial documents queryable, not markets predictable. Two genuinely reusable ideas for our narration/research-assistant seam: (1) **Program-of-Thought** — have the LLM emit *verifiable code* for any arithmetic in a "why this one" explanation rather than hallucinate figures (directly mitigates the hallucination risk flagged in [14]); (2) the **adaptive router** (cheap path for easy questions) is a concrete cost-control pattern for a local advisor. But this is firmly a context/extraction tool: it answers "what did the 10-K say," never "what will the price do." Catalog as RAG/extraction best-practice, not alpha.

### [46] Metadata-Driven Retrieval-Augmented Generation for Financial Question Answering
- **Authors / Venue:** Michail Dadopoulos, Anestis Ladas, Stratos Moschidis, Ioannis Negkakis / Int. J. of Accounting Information Systems (under revision)
- **Year:** 2025
- **Source:** arXiv:2510.24402
- **% read:** 45% (abstract + method + findings via abstract page)
- **Summary:** A systematic study of how to make RAG work on *long, structured* financial filings where the relevant evidence is **sparse and cross-referenced** — the regime where naive vector retrieval fails. Proposes a multi-stage architecture combining **LLM-generated metadata**, pre-retrieval filtering, post-retrieval reranking, and **contextual embeddings** (metadata embedded *alongside* the chunk text). Benchmarked on **FinanceBench**; the headline finding is that embedding chunk metadata directly with text ("contextual chunks") gives the largest gain, and a custom metadata reranker is more cost-effective than commercial rerankers. Pure document-QA accuracy; no trading.
- **Relevance to our system:** More RAG-for-finance depth, and a practical engineering lesson if we ever add a filing/news retrieval layer behind narration: **plain semantic chunking is insufficient for financial documents** — you need metadata-enriched chunks + reranking, because the evidence is sparse and cross-referenced. Reinforces FinSeer's [8] point from the document side (generic embeddings are weak for finance) and the cost-control theme. Strictly a context/extraction tool — background for the narration seam, not the decision rail.

### [47] Harnessing Earnings Reports for Stock Predictions: A QLoRA-Enhanced LLM Approach
- **Authors / Venue:** Haowei Ni, Shuchen Meng, Xupeng Chen, et al. / IEEE DOCS 2024
- **Year:** 2024
- **Source:** arXiv:2408.06634
- **% read:** 40% (abstract + method via abstract page + search summary)
- **Summary:** Fine-tunes an open LLM (Llama-3-8B-Instruct, 4-bit) via **QLoRA** instruction-tuning to predict post-earnings stock movement, fusing base factors (financial metrics + earnings-call transcripts) with external factors (market indices, analyst grades) into an ~8,556-row dataset (next-day move after earnings). Reports gains in accuracy / weighted-F1 / **Matthews correlation coefficient**, with the 4-bit Llama-3-8B beating GPT-4 on the task. Results are **classification metrics only — no trading PnL, no transaction costs, no stated walk-forward/post-cutoff protocol.**
- **Relevance to our system:** A representative earnings-call → movement-prediction study (fills the flagged earnings/10-K seam). Two honest reads for us: (a) it confirms a *small, 4-bit, QLoRA-tuned open model can outperform a frontier API* on a finance classification task — directly supportive of our "small local model is the realistic substrate" takeaway ([1][40]) for any narration/extraction component; (b) but it stops at classification metrics (MCC/F1) and never crosses into PnL-vs-B&H net of costs, so it says nothing about tradeable edge — and earnings-event trading is anyway inapplicable to a single-coin crypto advisor (no earnings). Catalog as evidence for the *local-model-is-enough* engineering point, not as alpha.

### [48] From Text to Alpha: Can LLMs Track Evolving Signals in Corporate Disclosures?
- **Authors / Venue:** Chanyeol Choi, Yoon Kim, Yu Yu, ..., Alejandro Lopez-Lira, Yongjae Lee / arXiv (cs.CE)
- **Year:** 2025 (rev. 2026)
- **Source:** arXiv:2510.03195
- **% read:** 45% (abstract + method + results via abstract page)
- **Summary:** Tries to extract *alpha* from the **narrative** of corporate disclosures using "LLM as extractor, embedding as ruler": the LLM pulls metric-focused text spans, and embedding-similarity measures **semantic drift** across reporting periods to detect "moving targets" (shifts in which metrics management emphasizes). Claims **>2× the risk-adjusted alpha** of a named-entity-recognition baseline with significantly stronger predictive power. The abstract does **not** address transaction costs and does **not** mention look-ahead / leakage safeguards.
- **Relevance to our system:** Co-authored by Lopez-Lira ([9]) and more methodologically interesting than most ("track *changes* in emphasis," not static sentiment) — but it inherits the genre's evidentiary gaps for our purposes: a "risk-adjusted alpha vs NER baseline" claim with no cost accounting and no leakage control isn't gate-credible (the very look-ahead concern Glasserman & Lin [6] raised for exactly this author's domain). The transferable *idea* — measuring semantic *drift* over time as a feature rather than point sentiment — is a genuinely novel input-construction trick, but it's disclosure-driven (no analogue for a single crypto coin) and would still need post-cutoff, cost-aware, bootstrap-vs-B&H validation. Catalog as a clever feature idea, not evidence of edge.

### [49] BizFinBench: A Business-Driven Real-World Financial Benchmark for Evaluating LLMs
- **Authors / Venue:** Guilong Lu, Xuntao Guo, Rongjunchen Zhang, Wenqiao Zhu, Ji Liu (HiThink Research) / arXiv (cs.AI)
- **Year:** 2025
- **Source:** arXiv:2505.19457
- **% read:** 45% (abstract + structure + findings via abstract page)
- **Summary:** A *business-grounded* financial LLM benchmark: 6,781 annotated queries across five dimensions (numerical calculation, reasoning, information extraction, prediction recognition, knowledge QA) in nine fine-grained categories, with an "IteraJudge" LLM-as-judge method to de-bias evaluation. Benchmarks **25 models** (Claude-3.5-Sonnet, o3, Gemini-2.0, DeepSeek-R1, Qwen). Headline: **no model dominates all tasks; LLMs handle routine finance queries competently but struggle with complex cross-concept reasoning**, and even leaders only reach ~63–64 on numerical calculation. No dedicated trading/stock-prediction task.
- **Relevance to our system:** Fills the "LLM agent/finance benchmarks" seam (a flagged thin area) and adds another independent, benchmark-grade confirmation of our thesis split: LLMs are decent at routine financial *language/knowledge* tasks and weak at the harder *reasoning/quantitative* end. The ~63 ceiling on *numerical calculation* (even for frontier models) is a quiet but important caution for any narration that does arithmetic — reinforcing the Program-of-Thought "make it compute, don't let it guess numbers" lesson from [45]. Background/anchor citation; no trading claim to weigh.

### [50] Golden Touchstone: A Comprehensive Bilingual Benchmark for Evaluating Financial Large Language Models
- **Authors / Venue:** Xiaojun Wu, Junxi Liu, Huanyi Su, ..., Jian Guo (IDEA-FinAI) / Findings of EMNLP 2025
- **Year:** 2024 (rev. 2025)
- **Source:** arXiv:2411.06272
- **% read:** 55% (abstract + task list + stock-prediction results via HTML)
- **Summary:** A bilingual (Chinese-English) financial-LLM benchmark over **eight tasks**: sentiment analysis, classification, entity recognition, relation extraction, multiple-choice, summarization, QA, and **stock-movement prediction.** Compares GPT-4o, Llama3, FinGPT, FinMA, and the authors' open Touchstone-GPT (continual-pretrain + instruction-tune). The decisive data point: **on stock-movement prediction every model is near chance** — GPT-4o weighted-F1 **0.4241**, FinMA-7B 0.3211, Touchstone-GPT 0.4396 (English DJIA) — and the authors state the best result "falls short of practical utility," that "sentiment of news items may not reliably predict stock movements," and that quantitative data would be essential.
- **Relevance to our system:** One of the cleanest, most quotable confirmations of our thesis: a careful bilingual benchmark shows LLMs do fine on the seven *language* tasks and **collapse to ~0.42 weighted-F1 on the one *prediction* task** — i.e. *worse than a coin flip is plausible after costs*, and the authors themselves say it's not practically useful and that text-sentiment doesn't reliably predict moves. This is exactly the "language ≠ alpha" wall ([7][8][36]) seen yet again, and the explicit "you'd need quantitative data" admission echoes our "numbers > text" finding ([34]). Strong anchor citation for "even purpose-built finance LLMs can't predict direction." Touchstone-GPT is also a usable *open* finance model for text jobs.

### [51] FinArena: A Human-Agent Collaboration Framework for Financial Market Analysis and Forecasting
- **Authors / Venue:** Congluo Xu, Zhaobin Liu, Ziyang Li / arXiv (cs.CE)
- **Year:** 2025
- **Source:** arXiv:2503.02692
- **% read:** 40% (abstract + architecture via abstract page)
- **Summary:** A **human-agent collaboration** framework with a mixture-of-experts-inspired machine module: a **time-series agent** forecasts prices from history, a **News Agent** uses **adaptive RAG** over unstructured news (to curb hallucination), and a universal expert agent fuses multimodal features with the *investor's risk preference* to produce decisions. Reports it "surpasses traditional and SOTA benchmarks in stock trend prediction and yields promising results in trading simulations across various risk profiles." The abstract gives **no transaction costs, no B&H-net comparison, no leakage/post-cutoff discipline, and no concrete numbers.**
- **Relevance to our system:** Two reusable design ideas align with our posture: (a) **risk-preference-conditioned output** (the same gated picks framed differently per investor risk appetite) is a clean UX/narration pattern for an advisor; (b) **adaptive RAG specifically to reduce hallucination** on the news leg echoes our grounding requirement [14]. But the *forecasting/trading* claims are qualitative-only with the genre's standard gaps (no costs, no B&H-net, no leakage control), so they're not gate-credible. Catalog as human-in-the-loop + risk-profile-narration design inspiration (consistent with Alpha-GPT 2.0 [12]), not as alpha evidence.

### [52] Coinvisor: An RL-Enhanced Chatbot Agent for Interactive Cryptocurrency Investment Analysis
- **Authors / Venue:** Chong Chen, Ze Liu, Lingfeng Bao, Yanlin Wang, Ting Chen, Daoyuan Wu, Jiachi Chen / arXiv (cs.AI)
- **Year:** 2025
- **Source:** arXiv:2510.17235
- **% read:** 45% (abstract + mechanism + results via abstract page)
- **Summary:** A **crypto-specific conversational analysis agent** whose core contribution is an **RL-based tool-selection mechanism**: it learns to plan multi-step and orchestrate diverse analytical tools / data sources for real-time, interactive crypto investment *analysis*. Crucially, it is explicitly positioned for "accurate and actionable investment **insights**" via a chat UI — **not** automated trading, and it makes **no buy-and-hold / PnL claim.** Evaluated on tool-orchestration quality (+40.7% recall, +26.6% F1 over the base model) and user satisfaction (4.64/5).
- **Relevance to our system:** The single most *posture-aligned* crypto LLM paper in the ledger — it is an LLM doing exactly the job we'd give one: **interactive crypto research/narration with tool orchestration, NOT signal generation**, and it deliberately makes no return claim. Two concrete reusable ideas: (a) **RL-learned tool selection** is a principled way to let a narration agent decide *which* of our vetted (gated) tools/indicators to surface for a given operator question — a more rigorous version of FinAgent's tool-augmentation [4]; (b) evaluating the LLM on *orchestration quality + user satisfaction* (not PnL) is precisely how we'd validate a narration layer. Strong supporting example that the defensible LLM role in crypto is the analysis/UX layer — confirms our architecture, nothing to gate.

### [53] A Time Series is Worth 64 Words: Long-term Forecasting with Transformers (PatchTST)
- **Authors / Venue:** Yuqi Nie, Nam H. Nguyen, Phanwadee Sinthong, Jayant Kalagnanam (IBM/Princeton) / ICLR 2023
- **Year:** 2022 (ICLR 2023)
- **Source:** arXiv:2211.14730
- **% read:** 45% (abstract + method + results framing via abstract page + search summary)
- **Summary:** The influential **patching + channel-independence** transformer for long-horizon forecasting. Two ideas: (1) segment each series into **subseries-level patches** used as tokens (retains local semantics, cuts attention cost quadratically, allows longer look-back); (2) **channel-independence** — every univariate channel shares one embedding + transformer weights. Reports ~**21% MSE / ~17% MAE** reductions over prior transformer SOTA on standard multivariate benchmarks, plus strong **self-supervised pretraining** that beats supervised training and transfers across datasets. Benchmarks are general (weather/traffic/electricity/ILI); **no financial or crypto data, no trading/PnL.**
- **Relevance to our system:** A *primary* (fills the flagged "more TSFM primaries" seam) and important context: PatchTST is one of the **deep baselines** that the financial-return study [28] showed TSFMs only sparsely beat — and the patching idea is the very machinery the ablation paper [25] found does the real work (vs the language prior). So PatchTST matters to us as (a) the architectural ancestor of the patched-decoder TSFMs (TimesFM [18]); (b) a reminder that a *small, dedicated* patch-transformer is the cheaper, often-as-good alternative to a giant TSFM ([26]). Its honest scope — general-forecasting MSE/MAE, never PnL — is the usual caution: good accuracy on weather ≠ edge on crypto returns. Background/architecture catalog; if we ever forecast (vol), a PatchTST-class small model is the realistic candidate, still gated.

### [54] iTransformer: Inverted Transformers Are Effective for Time Series Forecasting
- **Authors / Venue:** Yong Liu, Tengge Hu, Haoran Zhang, Haixu Wu, Shiyu Wang, Lintao Ma, Mingsheng Long (Tsinghua THUML) / ICLR 2024 (Spotlight)
- **Year:** 2023 (ICLR 2024)
- **Source:** arXiv:2310.06625
- **% read:** 45% (abstract + method + framing via abstract page + search summary)
- **Summary:** **Inverts** the transformer for forecasting: instead of one token per timestamp (mixing all variates), it embeds **each whole series as a "variate token,"** so attention captures *cross-variate* correlations and the FFN learns per-variable nonlinear representations. This fixes two standard-transformer failure modes — performance degradation + compute blow-up with longer look-back, and poor variate-centric representations — and yields SOTA on challenging real-world multivariate benchmarks with better generalization across variates. Positioned as "the fundamental backbone" of TS forecasting. General benchmarks; **no financial/crypto eval, no trading.**
- **Relevance to our system:** The other deep baseline in [28]'s financial-return comparison (where TSFMs barely beat it/random-walk), so it's load-bearing context for our "TSFMs aren't alpha engines" conclusion. The inverted/variate-token idea is the multivariate counterpart to Moirai [22] and theoretically attractive *if* we fed BTC alongside on-chain/funding covariates — but the same caveats bind: it's an accuracy method on calm general benchmarks, not a PnL method, and a single-coin advisor rarely benefits from multivariate machinery. Catalog as architecture/baseline; no evidence it helps beat holding on crypto.

### [55] Time-MoE: Billion-Scale Time Series Foundation Models with Mixture of Experts
- **Authors / Venue:** Xiaoming Shi, Shiyu Wang, Yuqi Nie, Dianqi Li, Zhou Ye, Qingsong Wen, Ming Jin / ICLR 2025 (Spotlight)
- **Year:** 2024 (ICLR 2025)
- **Source:** arXiv:2409.16040
- **% read:** 45% (abstract + architecture + results framing via abstract page + search summary)
- **Summary:** Scales TSFMs to **2.4B parameters** using a **sparse mixture-of-experts** decoder-only transformer (auto-regressive, flexible horizons/contexts) that activates only a subset of experts per prediction — so capacity grows while inference cost stays bounded. Pretrained on **Time-300B** (>300B time-points across 9 domains). Reports significantly improved zero-shot and full-shot forecasting, consistently beating dense baselines by large margins. The abstract names **no financial/crypto domain** and reports general forecasting accuracy, not trading.
- **Relevance to our system:** The current high-water-mark for *scale* among numeric-pretrained TSFMs (fills the "more primaries" seam, MoE branch). For our advisor it mostly underscores the cost/skeptic tension: a 2.4B model is the *opposite* of a lean local Rust advisor, and the same independent critiques apply — scale doesn't fix the domain-bound, shock-fragile limits ([20][26]) and the headline is general-accuracy, not crypto PnL. Its MoE efficiency (activate few experts) is an interesting deployment idea in the abstract, but [43]/[28] show *small* models (TTM/PatchTST) are the practical financial choice; a billion-param MoE is not. Background/architecture catalog; no alpha evidence.

### [56] Tiny Time Mixers (TTMs): Fast Pre-trained Models for Enhanced Zero/Few-Shot Forecasting of Multivariate Time Series
- **Authors / Venue:** Vijay Ekambaram, Arindam Jati, Pankaj Dayama, ..., Jayant Kalagnanam (IBM Research) / NeurIPS 2024
- **Year:** 2024
- **Source:** arXiv:2401.03955
- **% read:** 45% (abstract + method + results framing via abstract page + search summary)
- **Summary:** The **anti-scale** TSFM: a compact model **starting from ~1M parameters**, built on the lightweight **TSMixer** (MLP-mixer, not attention) architecture, pretrained in just 4–8 hrs on public data. Innovations: adaptive patching, diverse-resolution sampling, resolution-prefix tuning, multi-level modeling for **cross-channel correlations**, and **exogenous-signal infusion** at fine-tune time. Reports **4–40% zero/few-shot improvement** over popular benchmarks while **running on CPU-only machines.** No financial/crypto evaluation in the paper itself (though [43] later applied TTM to financial tasks).
- **Relevance to our system:** The most *deployment-relevant* TSFM for a lean local advisor — ~1M params, CPU-runnable, fast to fine-tune — and it's the exact model [43] applied to finance (where pretrained TTM beat its un-pretrained version and needed 3–10 fewer years of data, but still **lost to specialized models on 2 of 3 financial tasks** and helped on *volatility/spread*, not direction). So TTM is the realistic candidate *if* we ever add a forecasting input: small, cheap, exogenous-signal-aware. But [43]'s finance result is the binding evidence — useful for *vol/risk* in data-scarce settings, not return-alpha, and only after our gate. The CPU-runnable + data-efficiency angle is the genuine practical upside worth remembering for short crypto histories. Catalog as the "if we must forecast, use a tiny model like this" reference.

### [57] N-HiTS: Neural Hierarchical Interpolation for Time Series Forecasting
- **Authors / Venue:** Cristian Challu, Kin G. Olivares, Boris N. Oreshkin, Federico Garza, Max Mergenthaler-Canseco, Artur Dubrawski / AAAI 2023
- **Year:** 2022 (AAAI 2023)
- **Source:** arXiv:2201.12886
- **% read:** 45% (abstract + method + results framing via abstract page + search summary)
- **Summary:** An MLP-based (N-BEATS-descendant, **no attention**) long-horizon forecaster using **hierarchical interpolation + multi-rate data sampling**: blocks specialize on different frequency bands and assemble the forecast via doubly-residual stacking, controlling both prediction volatility and compute. Reports ~**20% accuracy improvement over the latest transformers while ~50× faster.** Pure general-forecasting benchmarks; **no financial/crypto evaluation, no trading.**
- **Relevance to our system:** Another of the deep baselines [28] used (and that TSFMs only sparsely beat on financial *returns*), and notable for *not* being a transformer — it's a stark reminder that a cheap MLP-stack can match/beat attention models on accuracy ([25]'s broader point: the fancy architecture often isn't what matters). For our advisor: if a forecasting input is ever justified (vol/risk), an N-HiTS/PatchTST-class small model is the pragmatic, fast, CPU-friendly choice over a giant TSFM — but accuracy on generic benchmarks still says nothing about beating B&H on crypto returns. Background/architecture catalog.

### [58] Sundial: A Family of Highly Capable Time Series Foundation Models
- **Authors / Venue:** Yong Liu, Guo Qin, Zhiyuan Shi, Zhi Chen, Caiyin Yang, Xiangdong Huang, Jianmin Wang, Mingsheng Long (Tsinghua THUML) / ICML 2025 (Oral, Top 1%)
- **Year:** 2025 (ICML 2025)
- **Source:** arXiv:2502.00816
- **% read:** 45% (abstract + method + results framing via abstract page + search summary)
- **Summary:** A **generative** TSFM that abandons discrete tokenization: its **TimeFlow Loss** (a flow-matching objective) lets a transformer be pretrained natively on **continuous-valued** series and emit **multiple plausible non-deterministic forecasts** (no parametric density assumed). Pretrained on **TimeBench (~1 trillion time points**, mostly real + some synthetic); 128M-param base. Reports SOTA on **both point and probabilistic** benchmarks with millisecond zero-shot inference. **No financial/crypto evaluation; general forecasting.**
- **Relevance to our system:** The newest, most capable open numeric-pretrained TSFM (fills "more primaries", generative branch) — and methodologically the cleanest contrast to Chronos's [15] *quantize-into-tokens* approach: Sundial models the continuous distribution directly via flow-matching. Its **probabilistic, multi-future** output is the *right shape* for honest crypto forecasting (a wide predictive distribution centered near no-change — which is what B&H already encodes), but exactly therefore unlikely to yield directional alpha. Same binding caveats as the whole cluster: SOTA on calm general benchmarks, no finance/crypto/PnL evidence, domain-bound + shock-fragile per [20][26][27]. A model to watch (open, fast, probabilistic); no basis yet to expect it beats holding crypto. If we ever experiment, its probabilistic output could feed a *risk/sizing* overlay (gated), per the vol-not-direction finding [19][43].

### [59] LiveTradeBench: Seeking Real-World Alpha with Large Language Models
- **Authors / Venue:** Haofei Yu, Fenghai Li, Jiaxuan You / arXiv (q-fin.TR)
- **Year:** 2025
- **Source:** arXiv:2511.03628
- **% read:** 50% (abstract + setup + findings via abstract page)
- **Summary:** A **live, leakage-proof** trading benchmark: instead of offline backtests, it streams *real-time* market prices + news and has 21 LLMs (across families) make sequential portfolio decisions over a **50-day live window** on **US stocks and Polymarket** prediction markets — so by construction there's no train/test overlap. Central finding: **"high LMArena scores do NOT imply superior trading outcomes"** — static chat-benchmark skill fails to predict real-world decision performance; models show distinct risk styles, some adapt to live signals. The abstract does **not** clearly establish that any model achieved *statistically significant* alpha, and does not detail transaction-cost modeling.
- **Relevance to our system:** Methodologically the **gold-standard design** for the operator's skepticism — *live/post-cutoff is the only honest LLM-trading test*, exactly StockBench's [13] and Profit Mirage's [30] logic taken to its limit (no backtest at all). The headline — **chatbot-leaderboard skill ≠ trading skill** — is a fresh, quotable confirmation that financial-QA/reasoning ability doesn't transfer to returns ([5][7][13]). For us it both (a) reinforces "keep the LLM off the alpha rail" and (b) validates our discipline that any LLM signal must be judged on genuinely out-of-sample, sequential, cost-aware performance — not benchmark accuracy. Note it doesn't claim a winner, which is itself telling. Strong anchor citation.

### [60] RiskLabs: Predicting Financial Risk Using Large Language Model based on Multimodal and Multi-Sources Data
- **Authors / Venue:** Yupeng Cao, Zhi Chen, Prashant Kumar, ..., K.P. Subbalakshmi, Papa Momar Ndiaye / arXiv (q-fin.RM)
- **Year:** 2024 (rev. 2025)
- **Source:** arXiv:2404.07452
- **% read:** 45% (abstract + method + results framing via abstract page)
- **Summary:** Applies LLMs to **financial *risk* forecasting** (deliberately, not return/trading), fusing three modalities: **earnings-call text + vocal/audio features**, market **time-series** data, and **news** context, to predict **market volatility and variance.** Reports the multimodal fusion is effective at forecasting volatility/variance and analyzes each source's contribution, finding the LLM plays a crucial role. (Abstract gives no specific numbers / explicit baseline table.)
- **Relevance to our system:** Strongly supportive of our most important nuance: **the forecastable financial target is *volatility/risk*, not direction/return** — RiskLabs deliberately aims LLMs at *risk* and reports success there, mirroring the realized-vol findings [19][43] and the CVaR-RL framing [41]. It is also a clean **multimodal (text + audio + price + news)** example (fills the multimodal seam). For our advisor the takeaway is consistent and twofold: (a) if an LLM/forecasting component is ever justified, point it at **risk/sizing** (a vol estimate to drive a gated overlay), never at return-alpha; (b) the audio/earnings-call modality has no crypto analogue, but the *principle* — fuse numeric + text for a *risk* estimate — is the defensible direction. Catalog as evidence for "LLMs/forecasting help risk, not return."
