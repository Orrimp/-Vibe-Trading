# Trading Research Knowledge Base

A **durable, resumable** literature review on how to build a real-world trading
system — **strategies, backtesting, test data, learning, evolution, and LLM
applications**. Findings here will later feed concrete improvements to **our app**.

> **Our app (context for "relevance" notes):** a Rust single-coin crypto
> investment **advisor** — *paper/sim only, not live, not financial advice*.
> Flow: pick a coin + budget → **bake off** many strategies on `(coin, window)`
> → **rank** under a **FROZEN robustness gate** (1000-path moving-block
> bootstrap; a weakest-link verdict; **buy-and-hold is always the benchmark**) →
> emit a **forward plan** → **watch it paper-trade** the simulated budget.
> Validated thesis so far: **no active strategy robustly beats just holding,
> net of costs.** We care about: honest backtesting/robustness, strategy ideas
> worth testing, test-data discipline, and where ML/DL/LLM/evolution genuinely
> help vs. where they overfit.

## Layout

```
research/
├── README.md          ← this file (contract + resume protocol)
├── PROGRESS.md        ← resume state: per-topic counts, targets, status (orchestrator-owned)
├── papers.md          ← MASTER index (links + aggregate count; orchestrator-owned)
├── strategies/        ← quant/algo strategies, factors, microstructure, execution
├── backtesting/       ← backtest METHODOLOGY (overfitting metrics, CV, data-snooping, costs)
├── ml-trading/        ← classical ML, feature engineering, forecasting, financial-ML pipelines
├── deep-learning/     ← deep learning + reinforcement learning for trading
├── data/              ← test/train data, splits & leakage, overfitting, Monte-Carlo / synthetic data, point-in-time, labeling
├── evolution/         ← evolutionary/genetic algorithms, optimization, strategy-updating, alpha search
├── llms/              ← LLMs (likelihood-computing neural nets) in finance; incl. LLMs-trained-on-financial-time-series
├── risk-and-sizing/   ← Kelly, vol-targeting, drawdown control, risk parity, bet sizing
└── crypto-market-structure/ ← funding/basis/perp, crypto volatility & liquidity, on-chain, crypto regimes
```

> `llm-and-evolution/` (batch-1, 23 papers) was **SPLIT** (2026-06-27) into
> `evolution/` + `llms/` — they are distinct fields (LLMs are likelihood-computing
> neural nets; evolution is optimization / strategy-search). Its entries migrate
> into the two; the old folder is then retired.

Each topic folder contains (created + owned by its researcher agent):
- **`papers.md`** — the LEDGER. One entry per paper. **This is the source of truth.**
- **`knowledge.md`** — aggregated, organized findings + actionable takeaways for our app.

## Ledger entry format (`research/<topic>/papers.md`)

Appended **immediately after each paper is read** (so a crash loses ≤ 1 paper):

```
### [N] <Title>
- **Authors / Venue:** ...
- **Year:** YYYY
- **Source:** arXiv:XXXX.XXXXX / DOI / URL   ← real, verifiable identifier
- **% read:** NN%   (abstract ≈ 25 · +method ≈ 50 · +results ≈ 75 · full ≈ 100)
- **Summary:** 3–6 sentences — problem, method, key result.
- **Relevance to our system:** how it informs strategies / backtest / test-data /
  learning / evolution / LLM in OUR advisor (or "background only").
```

**Honesty rule:** only log papers you actually retrieved and read. Never invent a
paper, author, or result. Be truthful about `% read`. 15 real papers beat 25 fabricated ones.

## Resume protocol (multi-day; assume crashes)

The **ledgers ARE the state.** To resume in any fresh session:

1. `grep -rc '^### ' research/*/papers.md` → per-topic counts.
2. For each topic **under target**, spawn a researcher (`subagent_type: researcher`,
   or `general-purpose` carrying the §Agent contract below + the topic's seeds).
3. Tell it to **read its ledger first and SKIP titles already listed**, then
   continue to target, **writing incrementally** after every paper.
4. After agents return: regenerate `PROGRESS.md` + `papers.md` aggregate, then commit.

## Targets

- **100 papers PER TOPIC** (9 topics → ~900 total), reached over multiple resumable
  rounds. Batch 1 (~22 each for the original 5) is committed; the expansion to 100 +
  the 4 new topics (`data`, `evolution`, `llms`, `risk-and-sizing`,
  `crypto-market-structure`) run in rounds (resume + skip dupes; each run adds a batch).
- Prefer **open access** (arXiv, ar5iv, papers-with-code, open journals). Paywalled →
  abstract-only (25%) is acceptable **if noted**. Never fabricate to hit the count —
  an honest 60 beats 100 invented.

## Agent contract

Defined in [`.claude/agents/researcher.md`](../.claude/agents/researcher.md). Each
agent owns **ONE** topic folder and writes **ONLY** there — no cross-agent file
collisions. `PROGRESS.md` + the master `papers.md` are orchestrator-maintained.
