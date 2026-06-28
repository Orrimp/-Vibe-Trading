# Application — LLMs-on-TEXT for finance: narration, agents, sentiment, RAG, benchmarks

_Decision doc for analyst & architect. Bucket **(a)** of the `llms` topic split:
LLMs operating on **text** (and as multi-agent "traders"). Companion file —
bucket (b), LLMs/foundation-models trained ON numeric series —
[`application-llm-timeseries-foundation-models.md`](application-llm-timeseries-foundation-models.md).
Sources: `research/llms/knowledge.md`, `research/llms/papers.md` (cite `llms[N]`),
`research/SYNTHESIS.md`. No new papers; this distils what is already logged._

> **Our app:** Rust single-coin crypto **advisor** (paper/sim, not advice, not
> live). Pick ONE coin + budget → bake off EVERY strategy → rank under a FROZEN
> 1000-path moving-block-bootstrap gate (FRAGILE ⇒ can't crown; **buy-and-hold is
> always the benchmark + exempt**) → forward rule-based plan → watch it
> paper-trade. Validated thesis: **no active strategy robustly beats buy-and-hold
> net of costs.** **LLMs are NARRATION-ONLY** — they never enter the ranking
> (shipped F9 "why this one", ADR-0064; read-only reflection surface, ADR-0074).
> The product sells **measured honesty / "a framework for trading with traceable
> and plausible trading."**

---

## 1. Summary of the research

The text-LLM literature splits cleanly into **what holds up** (language jobs) and
**what does not** (trading alpha), and the evidence on each side is now
benchmark-grade and repeated.

**Language jobs hold up.** LLMs are genuinely strong at sentiment, NER,
extraction, QA, summarization, and explanation. This is the consistent finding of
the flagship finance model BloombergGPT `llms[5]` — which, tellingly, makes
**zero trading-PnL claims** — and of every holistic benchmark: FinBen `llms[7]`,
PIXIU/FLARE `llms[36]`, BizFinBench `llms[49]`, Golden Touchstone `llms[50]`. The
cheap open substrates are well-established: FinBERT `llms[35]` (encoder-sized
sentiment), FinGPT `llms[1]` and FinLlama `llms[10]` (LoRA-tuned open models),
Fin-R1 `llms[40]` (a 7B reasoning model that *deliberately* stays on advisory/QA,
no trading claim).

**Trading alpha does not hold up — and the refutations are now definitive.** The
headline LLM-trading-agent papers (FinMem `llms[2]`, TradingAgents `llms[3]` with
its Sharpe-8 / <1% drawdown red flag, FinAgent `llms[4]`) report eye-popping
returns under exactly the conditions our gate rejects: 3–6 cherry-picked names,
single short windows, missing/vague costs, and training cutoffs overlapping the
test period (leakage). Three classes of evidence dismantle this:

- **Contamination-clean evaluation deflates the edge.** StockBench `llms[13]`
  tests 14 frontier LLMs *after* their cutoffs (no leakage, no costs) and the win
  over buy-and-hold shrinks to ≈1–2% in a benign window and **inverts (all agents
  underperform) in downturns**. LiveTradeBench `llms[59]` runs *live* and finds
  **chatbot-leaderboard skill does not predict trading skill**. "Profit Mirage"
  `llms[30]` names and measures the mechanism: reported profits collapse past the
  training cutoff.
- **At scale, with bias controls + costs, B&H wins.** FINSABER `llms[86]` (20y
  S&P 500, survivorship/look-ahead/snooping controlled, commissions) finds **B&H
  beats FinMem and FinAgent on Sharpe on 3 of 4 stocks** (NFLX B&H 0.62 vs
  FinAgent **−0.42**) and on the best composite (B&H 0.70 vs 0.24 / −0.23); the
  agents are "overly conservative in bulls, overly aggressive in bears." KTD-Fin
  `llms[100]` (identifier-masking + Barra attribution + costs) shows
  **stock-selection alpha is *negative* for 9 of 10 models**; the big returns
  decompose into market + style beta, and CSI300 B&H matches/beats once selection
  is isolated. **LLM returns = knowing the story, not doing the analysis.**
- **On our own asset the careful tests say ≈ B&H.** FS-ReasoningAgent `llms[68]`
  (BTC/ETH/SOL, post-cutoff, with fees, explicit B&H) tracks B&H in bulls
  (slightly below) and only mildly cushions bears. CryptoBench `llms[98]`
  (monthly-refreshed, contamination-resistant) finds models **excel at retrieval
  but show "near-complete failure in predictive reasoning"** — and agentic
  wrapping makes it *worse*. The crypto trading-agent papers that *do* claim big
  returns (CryptoTrade `llms[31]`, the Bitcoin multi-agent system `llms[32]`, the
  crypto-portfolio system `llms[34]`, market-derived crypto sentiment `llms[42]`
  with its label-leak Sharpe of 5) all carry no/unstated costs, single bull-run
  windows, and cutoff overlap. Notably `llms[34]`'s own ablation shows the
  **price/market agent dwarfs the news agent** — numbers > text, our thesis from
  the inside.

**Sentiment is a real but fragile, cost-sensitive, decaying signal.**
Lopez-Lira & Tang `llms[9]` find genuine return-predictability from ChatGPT news
scoring — but the big hit-rate is on the *non-tradable* instant reaction, the
tradable drift lives in *illiquid small-caps*, and it **decays as adoption
rises**. Look-ahead bias `llms[6]` shows LLM sentiment is contaminated by the
model "remembering" outcomes (test post-cutoff or anonymize). The peer-reviewed
pro-sentiment results (Kirtac & Germano `llms[39]` Sharpe 3.05; the S&P 500 study
`llms[87]`; the LLM+RL study `llms[89]`) are all daily-rebalanced long-short with
**zero or tiny costs**; `llms[87]`'s authors *concede* zero-cost "overstates
returns," and `llms[89]` shows a rule-based sentiment variant **collapse 13.8% →
3.7% at just 5 bps**. Cross-sectional long-short is also structurally
inapplicable to a long-only single coin.

**RAG is a document/extraction tool, not a forecaster.** The RAG-for-finance
literature (agentic FinAgent-RAG `llms[45]`, metadata-driven RAG `llms[46]`,
FinSeer `llms[8]`) is about *answering questions over filings* (FinQA/ConvFinQA/
FinanceBench), where wins are real (76–78% execution accuracy `llms[45]`) and the
lessons concrete: financial docs need **Program-of-Thought** (emit verifiable
code for arithmetic `llms[45]`) and **metadata-enriched chunks + reranking**
`llms[46]`. The one RAG-for-*forecasting* attempt `llms[8]` still lands at ~54%
directional accuracy with no profit claim.

**The defensible posture is human-in-the-loop / analysis-only.** Alpha-GPT 2.0
`llms[12]` (research assistant), Coinvisor `llms[52]` (a crypto *analysis*
chatbot whose RL learns *tool selection* and makes **no PnL claim by design**),
FinRobot `llms[79]` (open platform, no alpha claim), SEP `llms[81]` (self-
reflective *explainable* predictions), FinSTaR `llms[88]` (deterministic
**assessment** >93% vs stochastic **prediction** plateauing at 65–80%,
explicitly attributed to market efficiency). The constrained "LLM-proposes /
immutable-gate-disposes" pattern `llms[69][77]` is the only way to let an LLM near
alpha — and it is **our exact posture**. Safety is a separate axis: auditing LLM
agents `llms[14]` warns accuracy/return benchmarks give an "illusion of
reliability" while hiding hallucination, stale data, and adversarial-prompt risk
— none of which a backtest gate catches.

---

## 2. Possible solutions / what can be done with this research

The text-LLM toolkit maps onto **explanation and operator-facing context**, never
the decision rail. Concretely:

1. **Grounded "why this one" narration** (already shipped as F9 / ADR-0064):
   an LLM speaks *only* the bake-off's machine values, with a deterministic
   faithfulness post-check and a templated fallback. The research says this is the
   one genuinely safe LLM job and gives the techniques to harden it (Program-of-
   Thought `llms[45]`, deterministic-compute-in-explanation `llms[88]`,
   self-reflective rationales `llms[81]`).
2. **Read-only reflection / decision-support surface** (already scoped as
   ADR-0074): at a decision point, factually surface prior gate verdicts for this
   coin/strategy — "you've paper-traded this before; here's what the gate said."
   The literature's "LLM = research assistant, human/gate is arbiter" posture
   `llms[12][52][79]` is exactly this.
3. **Honest benchmarking discipline as a product asset.** The contamination-clean
   / bias-controlled designs (StockBench `llms[13]`, FINSABER `llms[86]`, KTD-Fin
   `llms[100]`, LiveTradeBench `llms[59]`, CryptoBench `llms[98]`) are templates
   for *how to test any LLM-derived claim* and double as marketing copy for
   "measured honesty."
4. **(Gated, expected-null) exogenous sentiment feature.** *If* ever attempted, a
   FinBERT/FinGPT `llms[35][1]` sentiment score becomes a candidate **risk/de-risk
   input** (never alpha), tested post-cutoff or anonymized `llms[6]`, net of
   crypto-realistic costs, through the 1000-path bootstrap-vs-B&H gate. The honest
   prior is that it dies on costs `llms[9][87][89]`.
5. **LLM as pipeline/tooling engineer, not oracle.** TS-Agent `llms[70]` and
   AlphaSharpe `llms[71]` show an LLM can reliably *build/critique* analysis code
   and *design* robustness metrics — a developer-productivity aid for the
   gate-upgrade work, with the caveat that metric-search is itself a multiple-
   testing engine (deflate it).

---

## 3. Relevance for the project

This bucket is **directly load-bearing and largely already implemented** — the
research *validates* the constraints the codebase already enforces.

- **Narration-only is the correct architecture, independently confirmed.** Every
  serious, contamination-controlled study converges on "LLMs explain/retrieve,
  they don't predict/trade" `llms[5][7][13][86][100][98]`. F9 (ADR-0064) and the
  reflection surface (ADR-0074) are the *exact* role the literature endorses; the
  shipped faithfulness post-check + templated fallback in
  `crates/agent/src/narration.rs` is the concrete realization of the safety
  critique `llms[14]`.
- **"Traceable & plausible" is the literature's own conclusion.** FinSTaR's
  `llms[88]` assessment-vs-prediction split = our forecastable-vs-unforecastable
  split: *describing* what the series did is deterministic and >93% accurate (this
  is what our analytics and narration already do); *predicting* the future is
  capped by market efficiency. Narration that computes-then-states is exactly
  "traceable and plausible."
- **The thesis is externally proven on the LLM side.** FINSABER `llms[86]` and
  KTD-Fin `llms[100]` are, in effect, our advisor's thesis demonstrated at scale
  on equities by independent bias-controlled studies; FS-ReasoningAgent `llms[68]`
  demonstrates it correctly on BTC/ETH/SOL. These are the citations for "no, a
  frontier LLM can't just trade."
- **Honest negative on alpha.** There is **no gate-credible evidence** any text-
  LLM agent or sentiment strategy beats buy-and-hold net of costs on a liquid
  single coin. The apparent wins are leakage, no-cost backtests, single benign
  windows, illiquid-microcap concentration, or passive factor harvesting. The LLM
  must stay off the ranking — which the codebase already enforces.

---

## 4. Advantages for the project

- **Trust and explainability without alpha risk.** A grounded narration layer can
  measurably improve operator comprehension of *why the gate crowned (or refused
  to crown)* a strategy, with zero exposure to the overfitting that sinks LLM
  traders — because it speaks only already-gated numbers. SEP `llms[81]` and
  FinSTaR `llms[88]` show grounded explanation is a genuine LLM strength.
- **A differentiated, defensible product story.** "We use an LLM to *explain*, and
  a frozen statistical gate to *decide*" is precisely the posture the field's own
  audits `llms[14]` and bias-controlled studies `llms[86][100]` recommend — it
  turns measured honesty into a feature, not a limitation.
- **Cheap, local-capable substrates exist.** FinBERT `llms[35]`, FinGPT `llms[1]`,
  Fin-R1 `llms[40]` are small/open — compatible with the local-Ollama dev path and
  a hard token budget; no 50B from-scratch model (BloombergGPT `llms[5]`) needed.
- **Concrete hardening techniques are off-the-shelf.** Program-of-Thought
  `llms[45]` (emit verifiable code for any figure), metadata-enriched RAG `llms[46]`
  (if a filing/news retrieval layer is ever added), and deterministic-compute-in-
  CoT `llms[88]` directly reduce the hallucination risk the audit paper `llms[14]`
  flags.
- **Ready-made test designs.** The contamination-clean benchmarks give a
  drop-in protocol for vetting any future LLM-derived signal honestly.

---

## 5. Problems and challenges

- **Hallucination / faithfulness (the central risk).** A free-form "why this one"
  rationale will invent figures or causal stories `llms[14][3]`. *Mitigation in
  place:* the FROZEN deterministic faithfulness post-check + banned-phrase list +
  templated fallback in `crates/agent/src/narration.rs` (ADR-0064 § D2). Any
  change to that predicate set is an ADR amendment, not an edit.
- **Stale / adversarial news.** Any news/sentiment input introduces provenance,
  recency, and prompt-injection risk a backtest cannot catch `llms[14]`.
- **Leakage in any signal claim.** LLM sentiment is contaminated by outcome
  memorization `llms[6][30]`; any signal must be tested post-cutoff or anonymized,
  net of costs, vs B&H — directional accuracy is not edge `llms[8][9]`.
- **Cost-budget pressure.** Narration is an LLM consumer under a hard monthly
  token budget with 80%/100% auto-degrade. Free-form, retried, or multi-agent
  "debate" narration burns budget fast; templated/constrained generation +
  prompt caching + the adaptive-router idea `llms[45]` / Smart-Scheduler `llms[79]`
  are the cost controls.
- **`ui`-layering constraint.** `ui` must NOT depend on `strategy`/`exec`/`llm`/
  `models`; the LLM bootstrap lives in `agent`. The shipped seam already respects
  this — `NarrationOutcome { Ready | FellBack }` carries no `llm` type and crosses
  the agent→iced boundary like `ForwardPlan`; `ui` names `agent::NarrationOutcome`
  only in the `#[cfg(feature = "live")]` adapter. Do not leak `llm` types into
  `ui` when extending narration.
- **HARD CONSTRAINTS to respect:** USDT-denominated; Decimal not f64 (any sentiment
  score that touches sizing must not introduce f64 into the money path); anchored
  report SHAs byte-immutable (119/119) — narration/doc changes must not mutate
  anchored `spec/*/reports/`; gate/bands FROZEN (narration is additive, never a
  ranking input); paper-only; LLM via API with prompt caching + monthly budget,
  local Ollama for cost-free dev; `crates/llm` provides the provider trait +
  budgeted/cached/record-replay (use it; do not add a second LLM client path).
- **Scope-creep magnet.** The multi-agent "debate" pattern `llms[3][75][76]` is
  seductive and is exactly the configuration the refutations target — it belongs
  (if anywhere) as a *narration-structuring* device (bull-case/bear-case prose for
  the operator), never as a decision mechanism.

---

## 6. Concrete next steps / candidate work items

Named, with codebase location and priority. All additive; none touches the FROZEN
gate or the ranking.

- **P0 — Harden the F9 faithfulness check (highest value).** Extend the FROZEN
  predicate set in `crates/agent/src/narration.rs` `check_faithful` to require
  any *numeric figure* in the narration to be a verbatim match against
  `NarrationFacts` (Program-of-Thought spirit `llms[45]`; deterministic-compute
  `llms[88]`), and grow the banned-phrase list to cover prediction/causation verbs
  ("will rise", "because the market", "expect"). Add an adversarial test corpus
  (hallucinated figure, invented cause, stale-news claim) asserting `FellBack`.
  *Per the ADR-0064 § D2 contract, ship as an ADR-0064 amendment, not an ad-hoc
  edit.* Location: `crates/agent/src/narration.rs` + `crates/agent/tests/`.
- **P1 — Reflection-surface faithfulness parity (ADR-0074).** Apply the same
  "speak only stored, past-only facts; never re-rank" guard to the read-only
  reflection decision-support surface (`crates/reflection/` →
  `crates/trader` read helper → core-typed UI mirror). Add a test that the
  surface can never emit a forward/ranking statement `llms[68][88]`.
- **P1 — Budget-aware narration degradation test.** Assert that at 80%/100% token
  budget the narration path degrades to the templated fallback (no silent overspend),
  exercising `crates/llm` budgeted/cached/replay. Pair with a record-replay fixture
  so narration is testable offline (local-Ollama / replay), no live API in CI.
- **P2 — Honest-benchmark harness for any LLM-derived signal (gate, don't build
  alpha).** A standing test-data-discipline utility: given any candidate LLM signal,
  enforce post-cutoff or anonymized evaluation `llms[6]`, costs, and routing through
  the existing 1000-path bootstrap-vs-B&H gate before it can be surfaced. Encodes
  StockBench/FINSABER/KTD-Fin/CryptoBench discipline `llms[13][86][100][98]` as a
  reusable check. Location: a new `crates/audit`-side or `crates/backtest`-side
  test utility — **never a ranking input.**
- **P2 — (Only if operator demand) gated sentiment-as-risk experiment.** A
  FinBERT/FinGPT `llms[35][1]` crypto-news sentiment score wired *only* to a
  **de-risk** decision in the existing vol/risk overlay (`crates/forecast/overlay.rs`
  + `crates/risk`), with a day-1 baseline-equity-divergence e2e (the
  v3-vol-overlay-noop precedent), tested net of crypto costs post-cutoff. Honest
  expectation: it fails the gate (`llms[9][87][89]`). Frame as a *falsification
  experiment*, not a feature.

---

## 7. Open questions for analyst & architect

- Should the F9 faithfulness check escalate from phrase/predicate matching to a
  full **numeric-grounding** requirement (every number traced to `NarrationFacts`)?
  The research supports it `llms[45][88]`; cost is a richer post-check + ADR-0064
  amendment.
- For the reflection surface (ADR-0074), what is the exact contract that prevents
  an LLM gloss from *implying* a recommendation? Is templated-only safer than
  LLM-narrated for past-fact surfacing?
- Is a constrained **bull-case / bear-case** narration (the one defensible reuse
  of the multi-agent pattern `llms[3][76]`) worth the extra token budget over the
  current single grounded explanation — and does it improve operator decisions or
  just spend tokens?
- Do we ever want an LLM-assisted **research/tooling** seam (TS-Agent `llms[70]`,
  AlphaSharpe `llms[71]`) for the P0 gate-upgrade work — and if so, how do we keep
  metric-search from overfitting the gate (the SYNTHESIS P0 "frozen gate is safer"
  point)?
- What is the minimum viable **provenance + recency** contract for any future
  news input, given the adversarial-prompt risk `llms[14]`?

---

## 8. What NOT to do / out of scope

- **Do NOT let an LLM agent (single or multi-agent "debate") enter the ranking or
  emit a trade decision.** Every such result in the literature is leakage / no-cost
  / single-window / factor-harvesting `llms[2][3][4][30][31][32][86][100]`. This is
  the one bright line.
- **Do NOT trust any "LLM beats B&H" / "Sharpe > 2 from news" number at face
  value** — treat as leakage-or-no-cost until proven through our post-cutoff,
  cost-aware, bootstrap-vs-B&H gate `llms[39][42][87][89]`.
- **Do NOT add a second LLM client path** — all LLM traffic goes through
  `crates/llm` (budgeted/cached/record-replay/redact).
- **Do NOT let `ui` depend on `llm`** — narration crosses the agent→iced seam as a
  `core`-clean `NarrationOutcome`.
- **Do NOT use chat-leaderboard / QA-benchmark scores as evidence of trading
  capability** — explicitly refuted `llms[13][59][86]`.
