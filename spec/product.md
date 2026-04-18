---
slug: product
status: draft
owner: analyst
updated: 2026-04-17
---

# Product Requirements — Crypto Trading Agent

## Vision

A Rust-native autonomous trading agent for crypto markets that combines
classical ML, deep learning, and LLM reasoning to produce risk-aware trading
decisions across multiple exchanges.

## Goals

- Research, backtest, paper-trade, and eventually live-trade strategies.
- Combine numeric models (price/volume/order-book features, DL forecasters,
  RL policies) with LLM-driven reasoning (news, sentiment, macro).
- Be auditable: every decision, intent, order, fill, and P&L line traceable
  to its inputs, model versions, prompts, and config — backed by a
  double-entry ledger so cash and position accounts always reconcile.
- Be safe: hard risk limits enforced in Rust, independent of any model output.

## Non-goals (initial)

- Ultra-HFT sub-millisecond execution.
- Market making at scale.
- Regulated derivatives trading.

## Differentiator

What makes this worth building (and worth talking about) vs the existing crop:

1. **Rust-native end-to-end** — no Python orchestrator, no GIL, single binary
   per role. Performance and observability are first-class, not afterthoughts.
2. **Persistent reflection memory** — every closed trade produces a lesson
   card; the trader retrieves relevant lessons before composing the next
   order. (Gap in TradingAgents-style frameworks.)
3. **Type-encoded risk** — illegal orders fail at construction, never in the
   hot path. The risk engine is reviewable in isolation from strategy code.
4. **Auditable double-entry** — every decision and every cash/position move
   reconciles to a ledger. Closer to a treasury system than a PoC bot.

**Confirmed bet (2026-04-17):** long-term moat is **(2) + (4)** — persistent
reflection memory plus auditable double-entry ledger. Most projects can write
"more agents"; few persist real institutional memory or expose a defensible
audit trail. v0 priorities lean accordingly: every decision and every cash /
position movement reconciles to the ledger from day one, even with a trivial
strategy.

## Constraints

- Language: Rust (stable).
- Deployment: single-binary workloads plus optional Python sidecar only if
  unavoidable for a specific model.
- Models: prefer open-weights and local inference; LLMs via API with prompt
  caching and a strict monthly cost budget (tracked in `spec/architecture.md`).
- Data: start with spot markets on two majors (Binance/Coinbase), extend later.
- Lean on existing Rust crates rather than reinventing quant primitives —
  see [architecture.md → Foundation libraries](architecture.md#foundation-libraries)
  for the curated list (notably [RustQuant](https://github.com/avhz/RustQuant)
  for risk-reward metrics, stochastic processes, and time/calendar helpers).

## Stakeholders

- Vitaliy — product owner, operator.
- Claude agents — analyst, architect, developer, ui-designer, tester.

---

## Trading-time agent roster

> Inspired by [TauricResearch/TradingAgents](https://github.com/TauricResearch/TradingAgents).
> These are **runtime** agents inside the trading binary, distinct from the
> **dev-time** agents in [AGENT.md](../AGENT.md). They do not edit code — they
> produce signals, debate them, and pass them through risk gates.

Five-layer hierarchy mirroring a real trading desk:

### 1. Analyst layer (parallel, opinion-producing)

| Agent                   | Purpose                                                                | LLM tier      |
|-------------------------|------------------------------------------------------------------------|---------------|
| `fundamentals_analyst`  | On-chain metrics, tokenomics, supply schedules, protocol revenue       | quick-think   |
| `sentiment_analyst`     | Crypto-Twitter / Reddit / Discord sentiment, funding rates as proxy    | quick-think   |
| `news_analyst`          | Headlines, regulatory events, exchange listings, exploits              | quick-think   |
| `technical_analyst`     | Price/volume features, indicator suite (see below), DL forecasters     | quick-think + ML model |
| `macro_analyst` _(opt)_ | DXY, rates, equities correlation — gated by config                     | quick-think   |

All analysts run **in parallel** for a given symbol/timeframe and produce
typed, structured opinions (rating + confidence + evidence pointers), never
free text.

### 2. Researcher layer (debate)

- `bull_researcher` — argues the long case from the analyst evidence.
- `bear_researcher` — argues the short / stay-flat case.
- Bounded structured debate (`max_debate_rounds`, default 2). Output: a
  consolidated thesis with explicit counterpoints and remaining uncertainty.

### 3. Trader agent

Synthesizes the debate into a concrete proposed action: side, size, entry
condition, invalidation, time-in-force. **Deep-think tier** LLM; this is the
only agent that produces an `Order` candidate.

### 4. Risk team

- `volatility_risk` — checks proposed position against realized & implied vol budgets.
- `liquidity_risk` — checks venue depth, slippage estimate, exit feasibility.
- `correlation_risk` — checks portfolio-wide exposure and drawdown headroom.

Risk team can **veto** or **resize** but never **upsize** the trader's order.

### 5. Portfolio manager

Final approval gate. Holds the live book, enforces hard limits encoded in
Rust types (max leverage, daily loss stop, kill switch). Approves → emits to
`exec` crate. Rejects → records rationale into the decision audit log.

### Five-tier rating scale

All analysts and the trader output one of:
`STRONG_SELL | SELL | HOLD | BUY | STRONG_BUY` plus a `confidence: 0..1`
and a `horizon: short | medium | long`.

### Decision pipeline

```
analysts (parallel) → researcher debate → trader → risk team → portfolio manager → exec
                                ▲                       │
                                └───── feedback ────────┘
```

---

## Strategy library — roadmap

The `strategy` crate is a registry of named strategies that share
data/feature/risk/exec scaffolding. Rough order of arrival:

| Tier  | Strategy family                                  | Why now                                  |
|-------|--------------------------------------------------|------------------------------------------|
| v0    | SMA crossover (single pair)                      | Tracer bullet; validates the harness     |
| v0.5  | Multi-indicator rules (MACD + RSI + Bollinger)   | Exercises feature pipeline + tester      |
| v1    | Cross-sectional momentum (top-N)                 | First real edge candidate; multi-symbol  |
| v1.5  | Mean-reversion on z-scored pairs                 | Tests pairs / portfolio plumbing         |
| v2    | LLM-augmented news/sentiment overlay             | First LLM-in-the-loop strategy           |
| v2.5  | DL forecaster (TCN or small Transformer)         | First DL model in production             |
| v3    | RL policy on constrained action space            | Learning agent                           |
| v4+   | Event-driven (listings, exploits, regime shifts) | Higher-skill territory                   |

Each strategy lifts through the lifecycle gates (see below) — promotion is
never automatic.

---

## Data sources & feature pipeline

### Market data
- Spot OHLCV + trades + L2 book snapshots from Binance and Coinbase.
- Funding rates and open interest (perp-aware, even if perps are non-goal v0,
  funding is signal for spot).
- Historical bulk via venue dumps (Binance Vision); live via WebSocket.

### News & sentiment
- Crypto-native news: CoinDesk, The Block, Decrypt RSS.
- Social: X/Twitter API or scraper, Reddit (r/CryptoCurrency, r/Bitcoin).
- Optional: CryptoPanic aggregator.

### On-chain
- Glassnode / Coin Metrics free tier or self-hosted node for BTC/ETH basics.
- Etherscan API for token-specific metrics.

### Macro (optional)
- DXY, US 10Y, SPX from a free provider (Stooq, Yahoo).

### Technical indicator suite (v0)
- Trend: SMA, EMA, MACD.
- Momentum: RSI, Stochastic.
- Volatility: ATR, Bollinger Bands, realized vol.
- Volume/flow: OBV, VWAP, CVD.
- Microstructure: spread, depth-imbalance, trade-aggressor ratio.

### DL/RL models (planned)
- Forecaster: lightweight Transformer or TCN on multivariate features.
- Regime classifier: HMM or small LSTM for trend/range/crash regimes.
- RL policy: PPO agent on a constrained action space (size in {0, ¼, ½, full}).

### Universe & data fidelity ladder

Order of expansion. Each step requires the previous to be stable in paper mode
for ≥ 14 days before the next one is enabled.

| Tier | Universe                                  | Granularity              | Venues       |
|------|-------------------------------------------|--------------------------|--------------|
| v0   | `BTCUSDT` spot                            | 1m bars + trades         | Binance      |
| v0.5 | + `ETHUSDT` spot                          | + L2 snapshots           | + Coinbase   |
| v1   | Top-10 USDT spot                          | 1m + L2 + funding context| + Kraken     |
| v1.5 | + Stablecoin pairs (BTC/USDC, ETH/USDC)   | 1s aggregated trades     |              |
| v2   | + Top-25 perps (signal only, not exec)    | 1m + open interest       |              |
| v3+  | DEX (Uniswap v3, Hyperliquid)             | event stream + mempool   | RPC nodes    |

---

## LLM strategy

### Dual-tier model use (adapted from TradingAgents)
- `deep_think_llm` — Claude Opus / GPT-class model for trader, researcher
  debates, and post-trade analysis.
- `quick_think_llm` — Claude Haiku / Sonnet for analyst summaries, news
  classification, repeated structured calls.

### Provider abstraction
First-class providers behind a single Rust trait:
- Anthropic (default, with prompt caching).
- OpenAI compatible (covers OpenAI, OpenRouter, DeepSeek, local LM Studio).
- Local via Ollama for cost-free dev / sentiment classification.

### Cost controls
- Hard monthly token budget per role (configured in `architecture.md`).
- Mandatory prompt caching on stable system prompts.
- Tool-use schemas instead of free-text parsing.
- Cheaper tier auto-selected when confidence of the input is already high.

---

## Memory & continual learning

> Identified as a gap in TradingAgents — explicit differentiator for us.

- **Episodic memory**: every decision (analysts → debate → trader → risk →
  PM) persisted with inputs, model versions, prompts, and resulting P&L.
- **Reflection loop**: after each closed trade, a `post_mortem_analyst`
  writes a lesson card into a vector store (qdrant or SQLite + sqlite-vss).
- **Retrieval at decision time**: trader retrieves top-K relevant past
  lessons before composing the order.
- **Periodic distillation**: weekly job clusters lesson cards into rules
  the user can review and promote into the prompt library.

---

## Risk management (hard requirements)

- Risk limits enforced as Rust types — illegal orders fail to compile or
  fail at construction, never at runtime in hot path.
- Kill switch: presence of `.halt` file or missed heartbeat → flatten and
  stop.
- Daily loss stop, per-symbol exposure cap, max leverage, max drawdown
  trigger.
- Portfolio Manager veto is the only path to live exchange.
- Full audit log: every decision and every order is reproducible from logs.

---

## Operating modes

1. **Research** — backtest only, deterministic seeds, no LLM cost (cached
   responses replay).
2. **Paper** — live data feed, full pipeline, simulated fills, real LLM cost.
3. **Live** — paper mode + real venue. Requires explicit human approval to
   enable; defaults off.

---

## Strategy lifecycle — promotion gates

A strategy lives in **one stage at a time**. Promotion between stages is
explicit, criteria-driven, and human-approved (single-operator, but the gate
prevents drift):

| Stage         | Entry criteria                                                                | Approver                  |
|---------------|-------------------------------------------------------------------------------|---------------------------|
| `research`    | Hypothesis + backtest scenario file in `spec/features/<slug>.md`              | analyst                   |
| `paper`       | Backtest Sharpe > 1.0 on 2y OOS data; no fatal regressions in tester report   | tester verdict + operator |
| `live`        | 30 days paper without risk-limit breach; live cost ≤ projected; PM signoff    | operator                  |
| `deprecated`  | Live drawdown > 1.5× backtest, or operator opt-out                            | operator                  |

Promotion writes a stage-change row to the audit ledger. Demotion is allowed
at any time (PM-flatten + remove from active strategies).

---

## Configuration surface

Single TOML config (`config/agent.toml`) controls:
- `llm.deep_think` / `llm.quick_think` — provider + model id per tier
- `llm.budget_usd_month` — hard monthly cap
- `agents.enabled` — which analysts/researchers to run
- `agents.max_debate_rounds` — debate depth (default 2)
- `data.sources.*` — toggles per feed
- `risk.*` — limits (leverage, drawdown, exposure, daily loss)
- `mode` — `research | paper | live`
- `debug` — verbose logs and prompt dumps

---

## Cost economics — monthly ceiling

Real product, real opex. A monthly `costs.md` report (auto-generated by the
tester) reconciles actual spend against this ladder:

| Cost line                  |  v0 ceiling | v1 ceiling | v2 ceiling |
|----------------------------|------------:|-----------:|-----------:|
| LLM tokens (deep + quick)  |        $20  |       $80  |      $200  |
| Market data (paid feeds)   |         $0  |        $0  |       $50  |
| Hosting (single VM)        |        $20  |       $40  |       $80  |
| Storage (Parquet + ledger) |         $5  |       $15  |       $30  |
| **Total / month**          |     **$45** |   **$135** |   **$360** |

**Hard rule:** LLM cost auto-degrades to `quick_think` only when 80% of the
monthly budget is spent. At 100%, agent reverts to deterministic-only mode
and posts a cockpit alert. **Ladder confirmed (2026-04-17).**

---

## v0 — first step (proposed: paper-trading SMA in 2 weeks)

> **Candidate C** from the brainstorm. Deliberately boring strategy; the
> win is proving the harness end-to-end so every later feature drops in
> cleanly. Foundation > demo.

### Scope

**Week 1 — foundation**
- `core` types: `Symbol`, `Asset`, `Money<C>`, `Order`, `Fill`, `Position`,
  `Signal`, `Decision` (typed, no `unwrap`).
- `data` crate: Binance WS feed for `BTCUSDT@kline_1m` + historical Parquet
  loader + `trade_aggregation` for tick→bar.
- `audit` crate: `sqlx-ledger` (SQLite) with chart-of-accounts for cash / positions / P&L.
- `ui::cockpit`: live tape (last 200 trades), position panel, P&L card,
  kill-switch button. Reads from `agent` over an in-process bus.

**Week 2 — trading**
- `strategy::sma_crossover` deterministic rule (e.g. SMA(20) vs SMA(50)).
- `backtest` engine: paper-fill `MatchingEngine` with `bps: 2` slippage and
  `0.04%` taker fee.
- First end-to-end run: 2023 `BTCUSDT` 1m, full backtest report from tester
  agent.
- First paper-trading run: 24 h dry run with kill switch verified.

### Acceptance

- Tester report committed to `spec/reports/`.
- Cockpit screenshot in PR description showing live tape + paper position.
- Audit ledger reconciles: `start_cash + Σ(fills) + Σ(P&L) = end_equity`.
- Kill switch verified manually (toggle `.halt`, observe flatten + halt).

**Confirmed (2026-04-17):** Candidate C is v0. Analyst now drafts the
matching feature brief in `spec/features/v0-paper-sma.md`.

---

## Success metrics (long-run)

- **v0 (paper SMA)** — see scope above.
- **v0.5 (multi-indicator + first LLM analyst)** — `sentiment_analyst`
  writes opinions to the ledger; agent debate happens (no money on it yet).
- **v1 (multi-agent backtest)** — full analyst → debate → trader → risk
  pipeline produces positive Sharpe (> 1.0) net of fees on out-of-sample
  2024 H1.
- **v2 (paper at scale)** — 30 consecutive days of paper trading on top-10
  USDT spot without a risk-limit breach; LLM cost stays inside monthly
  budget.
- **v3 (live, optional)** — 90 days live with realized Sharpe within ±0.3
  of backtest, max drawdown ≤ 15%.

---

## Open decisions

Tracked here until the operator answers; then they migrate into the body or
into [architecture.md](architecture.md).

- [x] **Differentiator (2026-04-17):** persistent memory + double-entry
  audit. See [Differentiator](#differentiator).
- [x] **v0 scope (2026-04-17):** Candidate C — paper-trading SMA in 2 weeks.
  See [v0 — first step](#v0--first-step-proposed-paper-trading-sma-in-2-weeks).
- [x] **Cost ceilings (2026-04-17):** $45 / $135 / $360 monthly ladder
  with 80%/100% auto-degrade rules.
- [x] **Strategy registry shape (2026-04-17):** hot-loadable, in two phases.
  v0 ships a clean `Strategy` trait + compiled-in registry that is plug-in
  shaped (no API change later). v0.5 adds **(A) config-driven composition**
  (TOML assemblies of indicators + rules, atomic swap on file change). v1+
  adds **(B) WASM plugins** via `wasmtime` for genuinely custom logic.
  Native dynamic libs and embedded scripting explicitly rejected. Detail
  in [architecture.md → Strategy registry & hot-loading](architecture.md#strategy-registry--hot-loading).
- [ ] **Live trading horizon** — is going live ever in scope, or is this a
  research/paper-only product? (Drives whether we ever need exchange API
  keys, KYC, withdrawal flows.)
- [ ] **Second operator** — will anyone else ever use this UI? (Defaults
  to "no" — drives auth, multi-user concerns.)
- [ ] **Tax / reporting** — annual P&L report needed? FIFO/LIFO accounting?
  Jurisdiction? (Affects audit-ledger schema.)
- [ ] **DR / backups** — acceptable RPO/RTO if the box dies? snapshot
  cadence for ledger + Parquet archive?

---

## Explicitly NOT adopted from TradingAgents

- **LangGraph / Python runtime** — replaced by Rust + tokio actor pattern.
- **Equity-market focus** — we are crypto-only; tickers are exchange/symbol
  pairs, not stock symbols.
- **Single-shot session model** — we add persistent memory and reflection.
- **Synchronous debate orchestration** — analyst layer fans out in parallel.

## Changelog

- 2026-04-17 (analyst): initial scaffold.
- 2026-04-17 (analyst): added trading-time agent roster, decision pipeline,
  data/indicator suite, dual-tier LLM strategy, memory loop, risk
  requirements, operating modes, config surface, and revised success
  metrics — adapted from TradingAgents
  ([source](https://github.com/TauricResearch/TradingAgents)).
- 2026-04-17 (analyst): added constraint pointing to curated foundation
  libraries — adopting RustQuant
  ([source](https://github.com/avhz/RustQuant)) for risk metrics, stochastic
  processes, and time helpers; details in
  [architecture.md](architecture.md#foundation-libraries).
- 2026-04-17 (analyst): strengthened auditability goal to call out the
  double-entry ledger requirement (enabled by `sqlx-ledger` on SQLite, see
  [architecture.md → Audit & ledger](architecture.md#audit--ledger)) — every
  cash/position movement must reconcile.
- 2026-04-17 (analyst): brainstorm pass — added Differentiator (tentative
  bet: persistent memory + audit), Strategy library roadmap, Universe & data
  fidelity ladder, Strategy lifecycle gates, Cost economics ladder, concrete
  v0 scope (Candidate C — paper-trading SMA in 2 weeks), and Open decisions
  list. Updated Stakeholders to include `ui-designer`. Items marked
  `[DECIDE]` need operator signoff before they harden.
- 2026-04-17 (operator): all three `[DECIDE]` markers resolved — moat bet,
  v0 scope, and cost ladder confirmed. Strategy registry decided: hot-loadable
  via two-phase plan (config-driven A in v0.5, WASM B in v1+); v0 ships a
  plug-in-shaped trait. Analyst kicked off to draft `spec/features/v0-paper-sma.md`.
- 2026-04-17 (developer): updated stale `cala-ledger` references to `sqlx-ledger`
  per architect decision in `spec/architecture.md`. `sqlx-ledger` on SQLite is
  the confirmed v0 audit-ledger substrate.
