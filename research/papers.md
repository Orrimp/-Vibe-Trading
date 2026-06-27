# Papers — Master Index

Aggregate view. **The per-topic ledgers are the source of truth** — open them for
full entries (Title, Authors, Year, Source, % read, Summary, Relevance).

**Program: 100 papers per topic × 9 topics (~900), resumable over rounds.** Batch 1
(117 across the original 5) is committed; the expansion + 4 new topics run in rounds.

- [strategies](strategies/papers.md) — quant/algo strategies, factors, microstructure, execution
- [backtesting](backtesting/papers.md) — backtest methodology (overfitting metrics, CV, data-snooping, costs)
- [ml-trading](ml-trading/papers.md) — classical ML, feature engineering, forecasting, financial-ML pipelines
- [deep-learning](deep-learning/papers.md) — deep learning + reinforcement learning for trading
- [data](data/papers.md) — test/train data, splits & leakage, overfitting, Monte-Carlo / synthetic data, PIT, labeling
- [evolution](evolution/papers.md) — evolutionary/genetic algorithms, optimization, strategy-updating, alpha search
- [llms](llms/papers.md) — LLMs (likelihood-computing nets) in finance; incl. LLMs-on-financial-time-series
- [risk-and-sizing](risk-and-sizing/papers.md) — Kelly, vol-targeting, drawdown control, risk parity, bet sizing
- [crypto-market-structure](crypto-market-structure/papers.md) — funding/basis/perp, crypto vol & liquidity, on-chain, regimes

**Counts:** see [PROGRESS.md](PROGRESS.md); cross-topic roadmap in [SYNTHESIS.md](SYNTHESIS.md).
(`llm-and-evolution/` is being split into `evolution/` + `llms/` and then retired.)

_This file is an index. The orchestrator regenerates the aggregate count here +
in PROGRESS.md after each batch; agents never write to this file._
