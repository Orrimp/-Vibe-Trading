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
├── backtesting/       ← backtest methodology, overfitting, test-data discipline, costs
├── ml-trading/        ← classical ML, feature engineering, forecasting, financial-ML pipelines
├── deep-learning/     ← deep learning + reinforcement learning for trading
└── llm-and-evolution/ ← LLM trading agents + evolutionary/genetic strategy discovery
```

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

- **Batch 1:** ≥ 100 papers total (~20–25 per topic). Later batches extend coverage.
- Prefer **open access** (arXiv, ar5iv, papers-with-code, open journals). Paywalled →
  abstract-only (25%) is acceptable **if noted**.

## Agent contract

Defined in [`.claude/agents/researcher.md`](../.claude/agents/researcher.md). Each
agent owns **ONE** topic folder and writes **ONLY** there — no cross-agent file
collisions. `PROGRESS.md` + the master `papers.md` are orchestrator-maintained.
