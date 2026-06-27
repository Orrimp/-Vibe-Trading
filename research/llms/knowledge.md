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
   dwarfs the *news* agent — numbers > text, consistent with our thesis.

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

---

## Methods / findings that hold up (and which don't)

**Hold up:**
- LLMs excel at financial *text* tasks (sentiment, NER, extraction, QA) — broad,
  benchmark-grade evidence [5][7].
- Properly contamination-controlled evaluation (post-cutoff test windows) is the
  right way to test any LLM signal [9][13][6]; without it results are inflated.
- News-sentiment carries *some* genuine signal, but fragile/decaying/illiquid-
  concentrated [9].
- Tool-augmentation + human-in-the-loop is the defensible architecture [4][12].
- Layered/decaying memory as an *input-construction* idea is interesting [2].

**Do NOT hold up (for our purposes):**
- "LLM agent beats buy-and-hold" claims on 3–6 assets, one window, no costs,
  with cutoff overlap [2][3][4] — textbook of what our gate rejects.
- LLMs as numeric *forecasters* — weak even in careful studies [7][8].
- NLP accuracy as a proxy for tradeable edge — repeatedly shown insufficient
  [5][7][8][10].

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
   [27], barely beat a random walk on *returns* [28], degrade under shocks [20], and
   lose to small specialized models on most tasks [43][26]. If any forecasting model
   is ever bolted on, target **volatility for risk/sizing** (the one forecastable
   financial quantity [19][43]) — and gate it. [27][28][20][19][43]
7. **Treat any "crypto + LLM + big Sharpe" result as leakage/no-cost until proven
   otherwise.** Crypto-specific agents [31][32][34] and sentiment models [42] report
   3–5+ Sharpe via overlapping cutoffs, outcome-based labels, and zero costs — the
   exact "profit mirage" [30]. Our post-cutoff, cost-aware, bootstrap-vs-B&H gate is
   the necessary filter. [30][31][32][34][42]

---

## SPECIAL FOCUS — Has anyone trained an LLM / foundation model directly on financial NUMERIC time series?

*(Operator's open question. This is the headline thread of the round.)*

**Short answer: YES — purpose-built "time-series foundation models" (TSFMs) that
train transformer / LLM-style architectures directly on NUMERIC series exist, are
open-source and mature, and at least three were trained on corpora that explicitly
include financial data. BUT the independent, skeptical evidence is that they are
*competitive-not-dominant*, *domain-bound*, and *shock-fragile* — and crucially
NONE has gate-credible evidence of beating buy-and-hold on crypto net of costs.
For our advisor the honest verdict is: a real tool, plausibly useful as a
*volatility / risk* input, but no basis to expect it beats holding on crypto
returns.**

### Two distinct families (don't conflate them)

1. **Numeric-pretrained TSFMs** — learn from *numbers*, transformer/LLM
   architecture, trained from scratch on time-series corpora:
   - **Chronos** [15] (Amazon): scales+**quantizes** values into a token
     vocabulary, trains T5 LMs with cross-entropy. Open.
   - **TimesFM** [18] (Google): patched **decoder-only**, ~200M params, ~100B
     time-points. Open (Apache-2.0). Most deployable.
   - **Lag-Llama** [17]: first open TSFM; **decoder-only**, uses *lags* as
     covariates; probabilistic (Student-t). Lists finance among domains.
   - **Moirai** [22] (Salesforce): **masked-encoder**, *any-variate*
     multivariate; LOTSA corpus (27B obs, 9 domains incl. finance). Open.
   - **MOMENT** [21] (CMU): general TS *analysis* (forecasting + classification +
     anomaly detection + imputation), "Time Series Pile". Open.
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

2. **Text-LLM repurposing** — keep a *language* model and adapt the interface:
   - **LLMTime** [16]: encode the series as digit strings, zero-shot a frozen
     GPT-3 / LLaMA-2. (Notably GPT-4 is *worse* — RLHF hurts number calibration.)
   - **Time-LLM** [23]: "reprogram" a frozen Llama-7B/GPT-2/BERT via text
     prototypes + Prompt-as-Prefix.

### The skeptical evidence (this is the load-bearing part)

- **The language model contributes ~nothing to numeric forecasting.** [25]
  (NeurIPS 2024 Spotlight) ablates three LLM-for-TS methods: **removing the LLM,
  or swapping it for a trivial attention layer, does not degrade — usually
  *improves* — accuracy**, at up to 3 orders of magnitude less compute. The
  text-LLM-repurposing branch (Time-LLM, LSTPrompt) is essentially debunked.
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

- **[Largely answered this round]** Does a purpose-built TSFM zero-shot-forecast
  crypto returns better than naive/B&H? The literature's strong prior is **no**
  (off-the-shelf doesn't transfer [27]; barely beats random walk on returns [28];
  shock-fragile [20]). The only *open* sub-question worth a small experiment:
  **can a TSFM (e.g. TimesFM/Chronos) forecast crypto *realized volatility* well
  enough to drive a risk/sizing overlay that passes our gate?** — vol is the one
  target with positive finance evidence [19][43].
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
  pretraining helps** → [27]; finance-specific TSFMs exist (claim, unproven on
  crypto PnL) → FinCast [29]
- **On financial *returns*, TSFMs barely beat a random walk** ("useful priors,
  not alpha engines") → [28]
- **Crypto-specific LLM agents beat B&H only in no-cost / single-window / bull-run
  / cutoff-overlapping setups; on-chain features are the real contribution** →
  CryptoTrade [31], [32], [34]
- **Reported LLM-agent profit is largely memorization ("profit mirage"),
  collapses post-cutoff** → [30]; field's own survey withholds a profitability
  claim → [33]
- **In multi-agent crypto systems the price/market agent dominates the news
  agent** (numbers > text) → [34]
