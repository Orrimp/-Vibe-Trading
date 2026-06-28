# Knowledge — LLMs (likelihood-computing neural nets) in Finance

Synthesis of the `llms` ledger (`papers.md`). Our app: a Rust single-coin crypto
**advisor** (paper/sim only) — bake off strategies → rank under a FROZEN 1000-path
moving-block bootstrap gate (weakest-link verdict; buy-and-hold always the
benchmark) → forward paper-trade. Validated thesis: **no active strategy robustly
beats holding, net of costs.** We read the LLM literature for (a) where LLMs
*genuinely* help our advisor vs. hype, and (b) the operator's open question:
**has anyone trained an LLM/foundation model directly on financial numeric time
series?**

---

## Key themes

1. **The thesis split: language ≠ alpha.** The strongest, most independent
   evidence (FinBen [7], BloombergGPT [5], the DL→LLM survey [11]) converges on
   one finding: LLMs are genuinely good at *language-shaped* financial tasks
   (sentiment, extraction/NER, QA, summarization, narration) and **weak at
   numeric forecasting / quantitative prediction**. BloombergGPT — the flagship
   "finance LLM" — makes *zero* trading-PnL claims.

2. **LLM trading-agent papers report eye-popping returns under exactly the
   conditions our gate rejects.** FinMem [2], TradingAgents [3], FinAgent [4]
   all show big out-performance, but share: tiny asset universes (3–6 names,
   often news-cherry-picked), single short test windows, missing/vague
   transaction costs, and training cutoffs that overlap the test period
   (leakage). A reported Sharpe of 8 with <1% drawdown ([3]) is a red flag, not
   a result.

3. **The contamination-clean reality check deflates the hype.** StockBench [13]
   tests 14 frontier LLMs *after* their knowledge cutoffs (no leakage), and the
   "win" over buy-and-hold shrinks to ≈1–2% in a benign window *with no costs* —
   and **inverts (all agents underperform) during downturns.** This is the
   single best capstone for our thesis on the LLM side.

4. **LLM news-sentiment is a real but fragile, cost-sensitive, decaying signal.**
   Lopez-Lira & Tang [9] find genuine return-predictability from ChatGPT news
   scoring — but the big hit-rate is on the *non-tradable* instant reaction, the
   tradable drift lives in *illiquid small-caps*, and it *decays as adoption
   rises*. Look-ahead bias [6] further shows LLM sentiment is contaminated by
   the model "remembering" outcomes — must be tested post-cutoff or anonymized.

5. **RAG helps retrieval/context, not forecasting profit.** FinSeer/StockLLM [8]
   builds an honest RAG-for-time-series pipeline and still lands at ~54%
   directional accuracy with *no* profitability claim — and finds generic text
   embeddings are the *wrong* substrate for numeric series.

6. **Safety/governance is a separate axis from PnL.** Auditing LLM agents [14]
   warns accuracy/return benchmarks give an "illusion of reliability" while
   hiding hallucination, stale data, and adversarial-prompt risk — none of which
   our backtest gate catches.

7. **The most aligned posture is human-in-the-loop / tool-augmented.** Alpha-GPT
   2.0 [12] and FinAgent's tool-augmentation [4] frame the LLM as a research
   assistant/orchestrator that *calls vetted tools* and *retrieves context*,
   with a human (or, for us, the robustness gate) as the arbiter — not an
   autonomous alpha generator.

8. **Crypto-specific LLM agents inherit every failure mode — on our own asset.**
   CryptoTrade [31], the Adaptive Multi-Agent Bitcoin system [32], and the LLM
   crypto-portfolio system [34] all report beating B&H, but with no/unstated
   transaction costs, single short windows (often inside the 2024–25 crypto bull
   run, so "beats B&H" is partly just exposure), and training-cutoff overlap.
   Their genuinely useful contribution is *feature ideas* (on-chain flows), not
   gate-credible alpha. Tellingly, [34]'s ablation shows the *market/price* agent
   dwarfs the *news* agent — numbers > text, consistent with our thesis. **The
   round-3 well-controlled crypto study confirms the verdict:** FS-ReasoningAgent
   [68] — BTC/ETH/SOL, **post-cutoff, with fees, explicit B&H** — comes out
   **≈ buy-and-hold** (tracks it in bulls, slightly below; mildly cushions bears).
   The careful crypto test gives the thesis answer on our own coins.

9. **"Profit mirage" — reported LLM-agent profits are largely memorization.**
   Profit Mirage [30] formalizes and quantifies it: impressive backtest returns
   collapse once you test beyond the model's training cutoff. This is the named,
   measured version of the leakage critique we apply to [2][3][4][31][32], and the
   methodological complement to StockBench's [13] post-cutoff design.

10. **Every finance-LLM benchmark that adds a *prediction* task exposes the same
    wall.** FinBen [7], PIXIU/FLARE [36], BizFinBench [49], and Golden Touchstone
    [50] all show LLMs competent on language tasks (sentiment/extraction/QA) and
    near-chance on the numeric *prediction/forecasting* task. Golden Touchstone is
    the most quotable: **stock-movement weighted-F1 ≈ 0.42** even for GPT-4o, with
    the authors stating it "falls short of practical utility" and that text
    sentiment "may not reliably predict stock movements" — you'd need quantitative
    data. Benchmark-grade, independent, repeated confirmation of "language ≠ alpha."

11. **RAG for finance is a *document/extraction* tool, not a forecasting one.** The
    RAG-for-finance literature ([45] agentic FinAgent-RAG, [46] metadata-driven RAG,
    [8] FinSeer) is overwhelmingly about *answering questions over filings* (FinQA /
    ConvFinQA / TAT-QA / FinanceBench), where the wins are real (76–78% execution
    accuracy [45]) and the practical lessons concrete: financial docs need
    **Program-of-Thought** (emit verifiable code for arithmetic, [45]) and
    **metadata-enriched chunks + reranking** (plain semantic chunking fails on sparse,
    cross-referenced filings, [46]). The one RAG-for-*forecasting* attempt [8] still
    lands at ~54% directional accuracy with no profit claim. RAG = grounding/context
    for narration, not a price predictor.

12. **The most posture-aligned crypto LLM is an *analysis/narration* chatbot, not a
    trader.** Coinvisor [52] is a crypto investment-*analysis* chatbot whose RL learns
    *tool selection* (which vetted tool to surface) and is evaluated on orchestration
    quality + user satisfaction — **it makes no PnL/B&H claim by design.** This is
    exactly the LLM role our architecture reserves: research/UX layer over a gated
    tool library, never the decision rail. Contrast with the crypto *trading*-agent
    papers [31][32][34] that do claim returns and inherit the leakage/no-cost gaps.

13. **When you control for leakage AND decompose returns, LLM trading "skill"
    vanishes — it's passive factor harvesting.** The two landmark studies settle it,
    now with full-text numbers (deep-read): **FINSABER** [86] (20 yrs, S&P 500,
    bias-controlled, commission costs) finds the headline FinMem/FinAgent advantages
    **"deteriorate significantly under broader cross-section and longer-term
    evaluation"** — **buy-and-hold beats both agents on Sharpe on 3 of 4 stocks**
    (TSLA B&H 0.63 vs FinAgent 0.21; NFLX B&H 0.62 vs FinAgent **−0.42**) **and on the
    best composite (B&H 0.70 vs FinAgent 0.24, FinMem −0.23, even ARIMA 0.33)**, with
    agents "overly conservative in bulls (FinAgent 0.12 vs B&H 0.61), overly
    aggressive in bears." (Correction: the prior "all p > 0.34" line was unsupported —
    FINSABER reports no per-strategy alpha p-values; the refutation is the Sharpe
    tables.) **KTD-Fin** [100] (4-level identifier masking certified by a 10-attacker
    probe at ≤3% recovery + Barra attribution + 5/15 bps costs) shows **stock-selection
    alpha is *negative* for 9 of 10 models** (only Claude Opus 4.7 at +0.2%; worst
    −77.8%) — the big cumulative returns decompose into market + style beta (e.g.
    top-return Qwen3-Plus's +70.3% = +41.8% market + 29.2% style − 0.7% selection),
    and **CSI300 buy-and-hold (+36.9%)** beats most agents once selection is isolated.
    These independently rebuild *our two gate pillars*: leakage control +
    benchmark-relative attribution (= "B&H is always the benchmark"). LLM returns =
    knowing the story, not doing the analysis.

14. **The LLM "language prior" ablates out of numeric forecasting — now shown three
    ways.** Beyond [25] (NeurIPS-24 Spotlight: remove the LLM, accuracy is unchanged
    or better), round 3 adds: [73] (three identical architectures — text-pretrained,
    TS-pretrained, **random-init** — perform alike; a from-scratch transformer on
    ~50M samples matches frozen GPT-2; small-data overfitting masked prior wins), [80]
    (zero-shot LLM forecasters are noise-sensitive and underperform simple models),
    and [78] (LLMs hover near *random* on time-series *reasoning*; context-aided
    forecasting stays near/below a median baseline). The original "freeze a pretrained
    LM" idea [61] (GPT4TS) and its successors (Time-LLM [23], TEMPO [74], UniTime
    [62]) are the family these debunk. Even the pro-LLM datapoint [97] (LLMFew) wins
    only on *structured-signal, non-financial* classification and lacks the
    from-scratch control to prove the language prior matters.

15. **Crypto-native, contamination-resistant benchmarks repeat the wall — retrieve
    yes, predict no.** CryptoBench [98] (monthly-refreshed, on-chain/DeFi/DEX/MEV
    tasks) finds leading models **excel at *retrieval* but show "near-complete failure
    in *predictive* reasoning,"** functioning "better as search engines than
    analysts" — and agentic wrapping makes it *worse*. FinSTaR [88] makes the split
    explicit: deterministic **assessment** (compute drawdown/vol from prices) hits
    >93%, but stochastic **prediction** plateaus at 65–80%, which the authors
    attribute to **market efficiency** ("info not contained in price history alone")
    — a reasoning-model-era restatement of our thesis.

---

## Methods / findings that hold up (and which don't)

**Hold up:**
- LLMs excel at financial *text* tasks (sentiment, NER, extraction, QA) — broad,
  benchmark-grade evidence [5][7]; and at *retrieval* over crypto data [98].
- Properly contamination-controlled evaluation (post-cutoff windows / identifier
  masking) is the right way to test any LLM signal [9][13][6][86][100]; without it
  results are inflated.
- Benchmark-relative attribution (strip passive market/style exposure) is essential
  — LLM "returns" are largely factor harvesting [100]. = our "B&H is the benchmark."
- News-sentiment carries *some* genuine signal, but fragile/decaying/illiquid-
  concentrated [9]; turnover-driven sentiment edge collapses under costs [89] (5 bps
  cuts 13.8% → 3.7%).
- Tool-augmentation + human-in-the-loop is the defensible architecture [4][12][52].
- The LLM's lane on numbers is **assessment/explanation** (deterministic, >93% [88]),
  not prediction.
- Layered/decaying memory as an *input-construction* idea is interesting [2].

**Do NOT hold up (for our purposes):**
- "LLM agent beats buy-and-hold" claims on 3–6 assets, one window, no costs,
  with cutoff overlap [2][3][4] — textbook of what our gate rejects; **refuted at
  scale** by the long-run bias-controlled [86] (B&H beats the agents on Sharpe on
  3/4 stocks + best composite) and memory-controlled [100] studies (negative
  selection alpha for 9/10; returns = market+style beta; B&H matches/beats).
- LLMs as numeric *forecasters* — weak even in careful studies [7][8]; the language
  prior **ablates out** [25][73][80] and LLMs barely reason about series [78].
- TSFMs as crypto *return* forecasters — barely beat random walk [28], don't
  transfer off-the-shelf [27], shock-fragile [20], miscalibrated [83]; on crypto the
  one peer-reviewed economic test [64] gives BTC Sharpe ~1.0 (≈ B&H).
- NLP/classification accuracy or low forecasting RMSE as a proxy for tradeable edge —
  repeatedly shown insufficient [5][7][8][10]; FinTSB [82] proves **error and profit
  decouple** ("lower MSE ≠ more profit"). The **price-level trap** is the recurring
  on-crypto offender: ETH frozen-LLM [63], FinCast's crypto MSE [29], the FinBERT-
  BiLSTM "98% accuracy / 0.019% MAPE" on BTC/ETH [72], and TimesFM's Bitcoin Monash
  point [18] all post great *level* error vs no random-walk/B&H baseline — meaningless
  for return direction (tomorrow's price ≈ today's, which B&H already captures).
- Synthetic/generated paths as a stress-test substitute — statistical fidelity ≠
  profitability [67]; resample real data instead.

---

## Actionable takeaways for our advisor

1. **Keep the LLM strictly off the alpha/decision rail.** Use it for narration
   ("why this one"), context, and operator research help only. The robustness
   gate — never the LLM — decides what to trade. [5][7][11][13][14]
2. **Ground all LLM narration in the gated numbers** (templated/constrained, not
   free-form), with provenance + recency checks on any news input, to contain
   hallucination/staleness/adversarial risk. [14]
3. **If we ever test an LLM-derived signal (e.g. sentiment), test it
   post-cutoff or on anonymized text, net of costs, through the 1000-path
   bootstrap vs B&H** — directional accuracy is not edge. [6][8][9]
4. **A small LoRA-tuned open model (FinGPT/FinLlama style) is the realistic
   substrate** for any local narration/sentiment component — a 50B from-scratch
   model (BloombergGPT) is irrelevant to a retail advisor. [1][10][5]
5. **Tool-augmentation is the right pattern**: an LLM that *calls* our vetted,
   gated strategy library + explains results — not one that invents signals. [4][12]
6. **Do NOT adopt a time-series foundation model to forecast crypto returns.** The
   finance-applied evidence is clear: off-the-shelf TSFMs don't transfer to finance
   [27], barely beat a random walk on *returns* [28], degrade under shocks [20], lose
   to small specialized models on most tasks [43][26][57], are **miscalibrated** [83],
   and the *only* peer-reviewed crypto economic test gives BTC Sharpe ~1.0 ≈ B&H [64].
   Deep-read sharpening: the famous *generic* TSFMs aren't even finance-trained
   (Chronos/TimesFM have **no finance** in-corpus; Moirai's LOTSA is **0.10%** finance;
   Lag-Llama's is one daily FX set) — and Chronos is *architecturally* "infeasible for
   strong-trend series" [15], the worst case for crypto. The one model genuinely
   trained on crypto (FinCast [29], 8.69% crypto) only wins on **price-level MSE with
   no PnL/Sharpe/B&H — the level trap.** FinTSB [82] shows forecasting error and
   trading profit **decouple**, and [64] is a live example (the best-accuracy config ≠
   the big-Sharpe config). If any forecasting model is ever bolted on, target
   **volatility for risk/sizing** (the one forecastable financial quantity [19][43]) —
   prefer a *small* model ([56] TTM, [53] PatchTST, [57] N-HiTS, [66] complexity-
   router), and gate it. [27][28][20][19][43][82][64][83][15][29]
7. **Treat any "crypto + LLM + big Sharpe" result as leakage/no-cost until proven
   otherwise.** Crypto-specific agents [31][32][34] and sentiment models [42] report
   3–5+ Sharpe via overlapping cutoffs, outcome-based labels, and zero costs — the
   exact "profit mirage" [30]. The careful crypto study comes out ≈ B&H [68]; the
   long-run bias-controlled [86] (**B&H beats the agents on Sharpe**) and
   memory-controlled [100] (**negative selection alpha for 9/10; returns are
   market+style beta**) equity studies show the edge is artifact/factor-exposure once
   leakage + factor exposure are removed. Our post-cutoff, cost-aware,
   bootstrap-vs-B&H gate is the necessary filter.
   [30][31][32][34][42][68][86][100]
8. **Watch metric-overfitting in the gate-improvement work (Deflated-Sharpe/PBO).**
   An LLM can help *design* robustness metrics [71], but searching over many candidate
   metrics is itself a multiple-testing engine that inflates the winner — our
   **frozen** (no per-run metric search) gate is the safer design; deflate any
   selected-from-many statistic. [71]

---

## SPECIAL FOCUS — Has anyone trained an LLM / foundation model directly on financial NUMERIC time series?

*(Operator's open question. This is the headline thread of the round.)*

**Short answer: YES — purpose-built "time-series foundation models" (TSFMs) that
train transformer / LLM-style architectures directly on NUMERIC series exist, are
open-source and mature. Deep-read refinement (this pass): the *famous generic* TSFMs
are barely trained on finance — Chronos [15] and TimesFM [18] have NO finance/crypto
in their corpora (web-traffic + synthetic), Moirai's [22] LOTSA is only 0.10%
finance, Lag-Llama's [17] is one daily FX set. The genuine "trained on crypto numeric
series" answer is the *purpose-built* finance model FinCast [29]: a 1B-param MoE with
8.69% crypto (1.78B points) in pretraining, evaluated on crypto_1day/1hour/1min and
beating TimesFM ~2× on crypto MSE (crypto_1day h=60: 0.2774 vs 0.5730). BUT FinCast's
win is price-level point-MSE with NO return/PnL/Sharpe/B&H metric — the level-
persistence trap — so even the strongest constructive case carries zero gate-credible
evidence of crypto return-alpha. Across the board the independent evidence is that
TSFMs are *competitive-not-dominant*, *domain-bound*, *shock-fragile*, and
*miscalibrated*; NONE has gate-credible evidence of beating buy-and-hold on crypto
net of costs. For our advisor the honest verdict is unchanged: a real tool, plausibly
useful only as a *volatility / risk* input, with no basis to expect it beats holding
on crypto returns.**

### Two distinct families (don't conflate them)

1. **Numeric-pretrained TSFMs** — learn from *numbers*, transformer/LLM
   architecture, trained from scratch on time-series corpora.
   **Full-text deep-read finding: the famous *generic* TSFMs are barely trained on
   finance at all** — so "trained on financial series" is overstated for them, and
   the constructive case rests on the *purpose-built* finance models ([29][27]):
   - **Chronos** [15] (Amazon): mean-scales + **quantizes into 4,094 bins over
     [−15,+15]**, trains T5 LMs (20M–710M) with cross-entropy. Open. **Corpus has
     NO finance/crypto** (finance named only aspirationally). Zero-shot only **~13%
     better than Seasonal-Naive** (never vs a random walk). **Stated limitation:
     "theoretically infeasible to model time series with a strong trend"** (fixed
     bin range) — a direct architectural strike against trend-dominated crypto.
   - **TimesFM** [18] (Google): patched **decoder-only**, 200M params, patch 32→128;
     **corpus is ~100B Wikipedia pageviews + Google-Trends + 60% synthetic — NO
     finance/crypto.** A lone **Bitcoin** point exists in the Monash table (MAE
     1.97e18 vs naive 5.32e18) but it's *price-level* MAE vs last-value, not return/PnL.
     Open (Apache-2.0). Most deployable; univariate point-forecast only, no covariates.
   - **Lag-Llama** [17]: first open TSFM; **decoder-only (8 layers)**, uses *lags* +
     date-time covariates; probabilistic (Student-t). **Its "finance" is ONE daily
     FX set (8 currencies) — no crypto, no equities.** Full read: **zero-shot avg
     rank 6.7 is WORSE than a supervised TFT (5.0)**; only fine-tuning makes it lead.
   - **Moirai** [22] (Salesforce): **masked-encoder**, *any-variate* multivariate;
     LOTSA = 27.6B obs / 9 domains — but **finance is only 0.10% (~24.9M obs)**;
     "Bitcoin" appears in the Monash table with no separate result. Zero-shot
     competitive-not-dominant (loses to PatchTST on retail). Open.
   - **MOMENT** [21] (CMU): general TS *analysis* (forecasting + classification +
     anomaly detection + imputation), "Time Series Pile" (100K+ series; finance =
     daily Exchange rates only). Open. Full read: **anomaly detection adj-F1 0.679**
     (beats GPT4TS 0.444) but a **trivial k-NN ties/beats it on VUS-ROC**, and its
     **zero-shot *forecasting* loses to ARIMA/ETS/Theta**.
   - **TimeGPT-1** [24] (Nixtla): first *commercial* TSFM, encoder-decoder, >100B
     points **explicitly including a finance domain.** Closed/SaaS.
   - **Time-MoE** [55]: scale leader — **2.4B-param sparse MoE** decoder-only,
     Time-300B corpus (300B points, 9 domains). Open (ICLR 2025 Spotlight). No
     finance eval; the *opposite* of a lean local advisor.
   - **TTM (Tiny Time Mixers)** [56] (IBM): the **anti-scale** model — ~1M params,
     TSMixer/MLP-mixer, **CPU-runnable**, exogenous-signal-aware. The realistic
     deployable choice; [43] applied it to finance (helps on vol/spread, not direction).
   - *Architectural primaries / deep baselines* (not foundation models themselves
     but the building blocks + the baselines TSFMs are measured against): **PatchTST**
     [53] (patching + channel-independence, ancestor of patched-decoder TSFMs) and
     **iTransformer** [54] (inverted "variate tokens" for multivariate). Both are
     among the deep baselines [28] showed TSFMs only sparsely beat on financial returns.

   - **Sundial** [58] (Tsinghua): newest, *generative* (flow-matching, continuous,
     no tokenization), ~1T points, probabilistic multi-future. Open. No finance eval.
   - **N-HiTS** [57]: MLP-stack (no attention) deep baseline; ~50× faster than
     transformers at comparable accuracy — among [28]'s financial baselines.

2. **Text-LLM repurposing** — keep a *language* model and adapt the interface.
   **Deep-read finding — the ablation conflict is the whole story:** the *method
   papers' own* in-house ablations claim the language backbone helps (Time-LLM [23]:
   Llama vs GPT-2 = 14.7% MSE win, "reprogramming-layer-alone = baseline"; GPT4TS [61]
   Table 7: pretrained 0.427 vs random-init 1.326 on ETTh1) — but the **controlled
   independent ablations [25][73] refute exactly this**, because the in-house
   "random-init"/"no-LLM" baselines are left under-powered on small data (the
   overfitting [73] diagnoses). When you train a from-scratch transformer of *equal
   capacity*, the language prior ablates out.
   - **LLMTime** [16]: encode the series as **per-digit strings**, zero-shot a frozen
     GPT-3 / LLaMA-2-70B; beats ARIMA/TCN/N-HiTS on CRPS. Full read: the win
     mechanism is an **Occam/repetition/seasonality bias** — *exactly what crypto
     returns lack*. **GPT-4 is *worse* than GPT-3 (RLHF degrades number calibration;
     chat < base)**; authors concede "text patterns don't connect to numeric
     extrapolation." Validated on TSMC stock (post-cutoff), not crypto.
   - **Time-LLM** [23]: "reprogram" a **frozen Llama-7B** (<6.6M trainable, ~0.2%)
     via text prototypes + Prompt-as-Prefix. No finance data.
   - **GPT4TS / "One Fits All"** [61]: freeze GPT-2's attention+FFN, train only
     embed/norm/output (~5%). Claims frozen attention "behaves like PCA" (Theorem 1)
     — which itself implies a *cheap linear method* would capture it. Debunked by
     [25][73] (the LM ablates out). No finance data.
   - **UniTime** [62]: one cross-domain model using *domain-instruction* text + a
     Language-TS Transformer. Partial wins (37/80), no finance.
   - *Applied to crypto:* [63] freezes Llama-3/Llama-2/GPT-2 on **ETH** numeric
     series (the GPT4TS recipe on our asset — but only beats LSTM/PatchTST on
     price-*level* MSE, with no random-walk baseline; the price-level trap).

### The skeptical evidence (this is the load-bearing part)

- **The language model contributes ~nothing to numeric forecasting.** [25]
  (NeurIPS 2024 Spotlight) ablates three LLM-for-TS methods: **removing the LLM,
  or swapping it for a trivial attention layer, does not degrade — usually
  *improves* — accuracy**, at up to 3 orders of magnitude less compute. The
  text-LLM-repurposing branch (Time-LLM, LSTPrompt) is essentially debunked.
- **The method papers' OWN ablations claim the opposite — and that's the tell**
  (deep-read finding). Time-LLM [23] reports its frozen Llama backbone is worth
  14.7% MSE (bigger backbone = better); GPT4TS [61] Table 7 shows pretrained 0.427
  vs random-init 1.326 on ETTh1. But these in-house "random-init"/"no-LLM" baselines
  are deliberately under-powered on small data — the exact small-dataset
  encoder/decoder overfitting [73] diagnoses. The **controlled** ablations win:
  [73] shows a from-scratch transformer on ~50M samples matches frozen GPT-2 and a
  random-init model matches a text-pretrained one; [25] removes the LM with no loss.
  Lesson that mirrors our gate: **small-data adaptation masquerades as capability**
  unless the baseline is capacity-matched — the same trap our bootstrap + planned
  Deflated-Sharpe/PBO exist to expose.
- **TSFMs are domain-bound and not cost-justified.** [26] finds zero-shot ability
  is **tightly tied to pretraining domains**, and even *fine-tuned* TSFMs don't
  consistently beat small dedicated models given their size.
- **They degrade exactly when it matters — during shocks/regime breaks.** [20]
  (Chronos + TimeGPT + Moirai on economic data): match/exceed classical models in
  *stable* periods, **degrade under rapid shocks.** Crypto is shock-dominated, so
  this is the worst case for us — the model is accurate when you don't need it and
  fails when you do (crash avoidance).
- **On an actual financial task, zero-shot is only "a reasonable baseline."** [19]
  applies TimesFM to **realized-volatility** forecasting: zero-shot is mediocre;
  you must **incrementally fine-tune** to beat HAR/GARCH — and even then the win is
  on *volatility* (forecastable, persistent), NOT on directional return / PnL.
- **Even fine-tuned numeric LLMs barely clear a coin flip on direction.**
  FinSeer/StockLLM [8] = ~54% next-day up/down, no profit claim; FinBen [7] = LLMs
  "fundamentally constrained in quantitative prediction."

### Now applied DIRECTLY to crypto (round-3 additions — the on-asset evidence)

This round added the crypto-specific tests the earlier (mostly equity/economic)
evidence was missing. The verdict is unchanged — and now shown on BTC/ETH/SOL:

- **A purpose-built foundation model trained ON crypto numeric series — and it's the
  level trap** [29] (FinCast, deep-read): a 1B-param MoE pretrained with **8.69%
  crypto (1.78B points)**, evaluated on crypto_1day/1hour/1min, **beats generic
  TimesFM ~2× on crypto MSE** (crypto_1day h=60: 0.2774 vs 0.5730). This is the
  cleanest "yes, someone trained a foundation model on crypto numbers." BUT it is
  **price-level point-MSE with NO return/PnL/Sharpe/B&H** — the level-persistence
  trap, where a random walk scores similarly and 2×-lower MSE says nothing about
  return direction. The strongest constructive answer still yields zero gate-credible
  crypto return-alpha (and it's a GPU-trained 1B model, wrong for a lean advisor).
- **TSFMs on 21 cryptos, with an economic (Sharpe) metric** [64] (peer-reviewed,
  MDPI; full text 403-blocked, cross-checked via two searches): fine-tuned TimeGPT
  *without* variables leads on *accuracy* (DM-confirmed); but the big economic number
  (**ETH Sharpe 4.29**) comes from a **different config** (TimeGPT *with* variables)
  under a **long/short** strategy — a textbook **error/profit decoupling** (the best
  forecaster is not the best trader, cf. [82]). On **BTC** (most liquid, closest to
  our default) the best Sharpe is only **~1.03**, i.e. ≈ buy-and-hold in a bull-tilted
  sample. Closed/SaaS anyway.
- **A careful, cost-aware, post-cutoff crypto LLM agent comes out ≈ buy-and-hold**
  [68] (FS-ReasoningAgent): on **BTC/ETH/SOL, Nov-2023→Jul-2024 (post-cutoff),
  with fees and an explicit B&H baseline**, returns **track B&H in bulls (slightly
  below: BTC 76.19% vs 79.63%)** and only mildly cushion bears (BTC −15.91% vs
  −19.15%). This is our thesis, demonstrated correctly on our own coins.
- **Frozen LLM on ETH numeric series** [63]: beats LSTM/PatchTST on price-*level*
  MSE but **never benchmarks a random walk** and never measures return/PnL — the
  price-level trap (level persistence is what B&H already captures).
- **LLM breakout-detection on crypto fails on the metric that matters** [65]
  (BreakGPT on Solana): uptrend **F1 ≈ 0.16**, beaten by a small ConvTransformer;
  "0.95 accuracy" is just the majority (no-surge) class.
- **The one rigorous-discipline crypto LLM result that *does* show alpha** [69]
  (constrained LLM factor search, strict OOS, costs) gets **Sharpe 1.55 — but it's
  cross-sectional long-short concentrated in *small illiquid tokens*** (cap-weighted
  fails; no B&H baseline). Structurally inapplicable to a long-only single coin, and
  the illiquidity-premium mirage [38] under realistic microcap costs.
- **TSFM operational viability in finance is hedged** [66]: newer TSFMs are "closing
  the gap" in finance but carry latency/drift/cost trade-offs; a "complexity router"
  beats deploying one universal TSFM — i.e. small models usually win on cost.
- **Synthetic-crypto generators: statistical fidelity ≠ profitability** [67]
  (CTBench): high-quality synthetic crypto series still fail to support profitable
  trading — supports our **bootstrap-from-real-data** design over generative paths
  (cf. [44]: LLM-simulated markets are unrealistically calm).

### What this means for our advisor

- **Returns/alpha:** no credible evidence any TSFM/LLM beats buy-and-hold on
  crypto returns net of costs. The forecastable part of price is the part B&H
  already captures (drift); the un-forecastable part is where active timing would
  need to win, and that's exactly where these models are weakest.
- **Volatility/risk:** the *one* genuinely forecastable financial quantity where a
  TSFM showed a (fine-tuned) edge is **realized volatility** [19]. If we ever bolt
  on a forecasting model, vol-for-sizing/de-risking is the defensible target — and
  it must still pass our cost-aware bootstrap-vs-B&H gate.
- **Substrate:** if experimented with, prefer the **open** TimesFM [18] / Chronos
  [15] over the closed TimeGPT [24] and over text-LLM-reprogramming [23] (debunked
  by [25]). A small dedicated model is likely cheaper and as good.
- **Don't reach for a chat LLM to forecast numbers** — [16] (GPT-4 worse than
  GPT-3) and [25] (LLM ablates out) both say the language prior doesn't help.

---

## Open questions / things worth testing in our app

- **[Answered — strongly negative now, on our own asset]** Does a purpose-built
  TSFM/LLM zero-shot-forecast crypto returns better than naive/B&H? The prior was
  already **no** (off-the-shelf doesn't transfer [27]; barely beats random walk on
  returns [28]; shock-fragile [20]); round 3 closes it with **crypto-specific** tests:
  peer-reviewed TSFMs give BTC Sharpe ~1.0 ≈ B&H [64]; a careful crypto LLM agent
  comes out ≈ B&H [68]; LLM crypto breakout-detection fails [65]; the frozen-LLM-on-ETH
  "win" is the price-level trap [63]; and contamination-clean crypto benchmarks show
  retrieve-yes-predict-no [98]. The only *open* sub-question worth a small experiment:
  **can a TSFM (e.g. TimesFM/Chronos/TTM) forecast crypto *realized volatility* well
  enough to drive a risk/sizing overlay that passes our gate?** — vol is the one
  target with positive finance evidence [19][43], **but** mind the calibration
  warning [83] (a miscalibrated vol forecast on the risk rail can *increase* drawdown).
- Could a constrained, grounded LLM narration layer measurably improve operator
  trust/decisions without ever touching the decision path? (UX experiment.)
- Is there *any* regime (e.g. a sharp, news-driven crypto crash) where an LLM
  news-sentiment overlay reduces drawdown enough to matter as a *risk* feature
  (not alpha)? Cf. the realized-vol [19] and CVaR-RL [41] framings.
- Could *on-chain* features (the genuinely crypto-native idea from CryptoTrade
  [31] / [34]) carry any gate-surviving signal — as a *feature*, tested net of
  costs post-cutoff, not as an LLM-agent claim?
- For synthetic/stress paths, our resample-from-real-data bootstrap is preferable
  to LLM-simulated markets — LLM agents are "textbook-rational" and miss
  bubbles/crashes [44]. (Confirms current design; nothing to change.)

---

## Paper map (claim → supporting [N])

- LLMs strong at financial *text*, weak at *numbers/forecasting* → [5][7][8][11]
- LLM trading-agent out-performance is an artifact of small-N / single-window /
  no-cost / cutoff-overlap setups → [2][3][4]
- Contamination-clean (post-cutoff) eval shrinks LLM edge to ~1–2%, inverts in
  downturns → [13]; post-cutoff/anonymization needed → [6][9]
- LLM news-sentiment = real but fragile/decaying/illiquid-concentrated signal →
  [9]
- RAG helps context/retrieval, not forecasting profit; text embeddings wrong for
  numeric series → [8]
- LLM-in-finance carries non-PnL risks (hallucination/stale/adversarial) → [14]
- Defensible LLM role = human-in-the-loop / tool-augmented research assistant →
  [4][12]
- Realistic substrate = small LoRA-tuned open model, not 50B from-scratch →
  [1][10] vs [5]
- **Numeric-pretrained TSFMs exist & are open** (numbers, not text) → Chronos [15],
  Lag-Llama [17], TimesFM [18], MOMENT [21], Moirai [22]; commercial+finance-trained
  → TimeGPT [24]
- **Text-LLMs can be made to "forecast" numbers** but the language prior is
  inessential → LLMTime [16], Time-LLM [23]; ablation debunk → [25]
- **TSFM zero-shot is competitive-not-dominant, domain-bound, shock-fragile** →
  [20][26]; on real financial vol, zero-shot only "reasonable", needs fine-tune →
  [19]
- **No gate-credible evidence any TSFM/LLM beats B&H on crypto returns net of
  costs**; forecastable edge (if any) is volatility not direction → [19][20][8][7]
- **Off-the-shelf TSFMs don't transfer to finance; only from-scratch financial
  pretraining helps** → [27]; **a foundation model trained ON crypto numeric series
  exists (FinCast [29]: 1B MoE, 8.69% crypto, beats TimesFM ~2× on crypto MSE) — but
  the win is price-level MSE with NO return/PnL/Sharpe/B&H (the level trap), so still
  unproven on crypto alpha** → FinCast [29]
- **On financial *returns*, TSFMs barely beat a random walk** ("useful priors,
  not alpha engines") → [28]
- **Crypto-specific LLM agents beat B&H only in no-cost / single-window / bull-run
  / cutoff-overlapping setups; on-chain features are the real contribution** →
  CryptoTrade [31], [32], [34]; a careful cost-aware post-cutoff crypto agent comes
  out **≈ B&H** → [68]; live crypto BTC/ETH benchmark = mixed/short/fragile → [90];
  live cost-aware crypto-futures agent shows no B&H baseline, modest Sharpe → [95]
- **Reported LLM-agent profit is largely memorization ("profit mirage"),
  collapses post-cutoff** → [30]; field's own survey withholds a profitability
  claim → [33]
- **In multi-agent crypto systems the price/market agent dominates the news
  agent** (numbers > text) → [34]; price-only multi-agent drops text because it
  "lags price discovery" → [76]

### Round-3 additions (claim → [N])

- **At scale, with leakage control + costs, LLM agents fail to beat B&H on Sharpe**
  → FINSABER [86] (20y S&P 500; B&H Sharpe beats FinMem/FinAgent on 3/4 stocks +
  best composite 0.70 vs 0.24/−0.23). (No alpha p-values reported — prior "p>0.34"
  was unsupported; refutation is the Sharpe tables.)
- **With identifier-masking + Barra attribution, LLM stock-selection alpha is
  *negative* for 9/10 (worst −77.8%); returns = market+style beta (top model +70.3%
  = +41.8% market + 29.2% style − 0.7% selection); CSI300 B&H (+36.9%) matches/beats**
  → KTD-Fin [100]
- **The LLM language prior ablates out of numeric forecasting** → [25] (orig.);
  random-init = pretrained [73]; noise-sensitive, loses to simple models [80];
  near-random on TS *reasoning*, context doesn't help [78]
- **Frozen-LM-for-TS family** (the methods those ablations target) → GPT4TS [61],
  Time-LLM [23], TEMPO [74], UniTime [62], TS-Reasoner [94]; pro-LLM claim but
  domain-mismatched + ablation-incomplete → LLMFew [97]
- **Peer-reviewed TSFMs on crypto: BTC Sharpe ≈ 1.0 (≈ B&H); the ETH 4.29 is
  long/short + no costs** → [64]; frozen LLM on ETH = price-level trap → [63];
  LLM crypto breakout-detection fails (F1 ≈ 0.16) → [65]
- **Forecasting error and trading profit DECOUPLE; small models (XGBoost/LGBM)
  beat deep/foundation models on financial TS, costs included** → FinTSB [82];
  finance-TS eval suites are proliferating because MSE misleads → [85]
- **TSFMs are miscalibrated (confidently wrong) — bad for a risk/vol overlay** →
  [83]; operationally a giant TSFM rarely pays vs a small/routed model → [66]
- **Newest numeric-pretrained TSFMs** (catalog) → Chronos-2 [84], Sundial [58],
  Time-MoE [55], TTM [56], N-HiTS [57], PatchTST [53], iTransformer [54]
- **Crypto contamination-resistant benchmark: retrieve yes, predict no; agentic
  wrapping makes it worse** → CryptoBench [98]
- **Assessment (deterministic, >93%) vs prediction (stochastic, plateaus 65–80% by
  market efficiency)** → FinSTaR [88]
- **Turnover-driven sentiment edge collapses under costs (13.8%→3.7% at 5 bps);
  one RL long-only variant marginally beats B&H on a single OOS year** → [89];
  zero-cost sentiment "beats B&H" caveat stated by authors → [87]
- **Multimodal text+numeric forecasting evidence is "genuinely mixed" / narrow /
  contamination-plagued; safe use is Time2Text explanation** → [99]
- **Synthetic crypto generators: statistical fidelity ≠ profitability** → CTBench
  [67] (supports bootstrap-from-real-data over generated paths)
- **LLM-discovered robustness *metrics* (Deflated-Sharpe-adjacent) — useful but
  metric-search itself overfits; frozen gate is safer** → AlphaSharpe [71]
- **Defensible LLM roles confirmed:** open finance *platform* (no PnL claim) →
  FinRobot [79]; explainable self-reflective prediction → SEP [81]; LLM-as-pipeline
  -engineer (reliable tooling, not alpha) → TS-Agent [70]; constrained
  LLM-proposes-immutable-gate-disposes → [69][77]
- **Every serious survey hedges on LLM trading alpha** → [11][33][38][91][93][96]
