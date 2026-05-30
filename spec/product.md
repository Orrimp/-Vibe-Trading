---
slug: product
status: draft
owner: analyst
updated: 2026-05-30
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

## Pillar stack — core vs support (ratified 2026-05-30)

> **The reframe.** Three "alpha-engine-by-prediction" bets have now
> under-delivered on real crypto OHLCV (empirical basis below). The product is
> re-anchored: **alpha + safety are the quantitative core; the LLM is a support
> pillar — explanation and narration over a quantitative core, NOT the alpha
> source.** This stops future sessions re-proposing LLM-as-alpha-engine.

### CORE (alpha + safety — all deterministic)

1. **Quantitative strategy** (momentum / pairs / composed) — the edge candidate.
2. **Monte-Carlo robustness layer** — quantifies whether the edge is real:
   resamples real returns into an ensemble of plausible paths and measures the
   *distribution* of a strategy's outcome (Sharpe percentiles, max-drawdown tail,
   probability of loss) rather than a single point estimate. First slice in
   flight: [`monte-carlo-bootstrap-path-generator`](monte-carlo-bootstrap-path-generator/feature.md)
   (C1, stationary block bootstrap of real Binance returns) +
   [`strategy-robustness-harness`](strategy-robustness-harness/feature.md)
   (C2, the distribution-summary report). Direction:
   [`spec/dev-notes/strategy-robustness-monte-carlo-direction-2026-05-29.md`](dev-notes/strategy-robustness-monte-carlo-direction-2026-05-29.md).
   **This is uncertainty quantification, NOT prediction** — categorically distinct
   from the retired forecasting bets (it resamples, it does not forecast).
3. **Deterministic learning loop** (future) — adapts param / route selection from
   past outcomes via the reflection store, through a sanctioned seam *outside* the
   strategy crate (the strategy crate stays consumer-free per the ADR-0041
   layering rule). Queue follow-on (C4), deliberately sequenced **after** the
   robustness harness so the loop consumes a real outcome distribution, not a
   hypothetical one. Requires no LLM.
4. **Risk envelope + auditable double-entry ledger** — the operationally-proven
   moat (see Differentiator). Hard limits in Rust types, independent of any model.

### SUPPORT (the LLM pillar — explanation, not decision)

The LLM is the **explanation and narration layer over the quantitative core**. Its
sanctioned roles, none of which is an alpha source:

- **Regime narration** — turn a detected regime + a robustness distribution into a
  human sentence ("median Sharpe 1.40 holds across 500 resamples but the p5 of
  0.31 and an 18% probability of net loss indicate the 2023 result is
  path-favourable").
- **Lesson summarization** — distill clusters of reflection `LessonCard`s into
  review-ready rules (the § Memory "periodic distillation" job).
- **Human-readable robustness-report explanation** — narrate the distribution
  summary so the quantitative story is legible to the operator.
- **Tie-break ONLY** — when two strategies / parameter sets are statistically
  **indistinguishable** on the robustness distribution, the LLM may break the tie
  with a narrated rationale. Bounded, auditable, **never** the primary gate.

**Explicitly NOT the alpha source.** The LLM does not generate the trading edge.
The edge is the quantitative strategy; the robustness layer says whether it is
real; the ledger proves it reconciles; the LLM makes it legible.

### Empirical basis for the demotion

Three alpha-engine-by-prediction bets, all on disk, all under-delivered on hourly
crypto OHLCV:

| Bet | Role attempted | Verdict | Reference |
|---|---|---|---|
| v2.5 DL forecaster programme (TCN + PatchTST + planned Transformer + v2.6 bake-off) | Predict next-period return → trade the prediction | **RETIRED 2026-05-22** — terminal F4 across two model families; no +0.10 Sharpe-delta; bake-off + Transformer phases deprecated without shipping (joint F4-F4 prior exhausted EV) | [`v25-dl-journey-retrospective-2026-05-22.md`](dev-notes/v25-dl-journey-retrospective-2026-05-22.md); [`v26-forecast-bakeoff`](v26-forecast-bakeoff/feature.md) |
| v3 volatility forecaster (GARCH) | Forecast σ → size positions by it | **RETIRED 2026-05-22** — MODEL-BROKEN / NO-ALPHA / negative net-delta after the noop-fix | [`v3-volatility-forecaster`](v3-volatility-forecaster/feature.md); [`v3-vol-overlay-noop-discovery-2026-05-22.md`](dev-notes/v3-vol-overlay-noop-discovery-2026-05-22.md) |
| v3 LLM forecaster | LLM as the alpha engine | **shipped-partial** — load-bearing Wave D alpha verdict deferred (no `ANTHROPIC_API_KEY`); the `strategic-reset` § 4.5 prior rates it LOW-MEDIUM to clear the +0.10 gate | [`v3-llm-forecaster`](v3-llm-forecaster/feature.md); [`strategic-reset-2026-05-23.md`](dev-notes/strategic-reset-2026-05-23.md) |

The honest read: **alpha-engine-by-prediction has not paid off, regardless of
whether the predictor is a TCN, a GARCH, or an LLM.** The retirements were of the
*prediction-as-alpha* role. The Monte-Carlo robustness bet (core pillar 2) is a
*different epistemic act* — it measures the variance of an already-shipped
strategy's outcome under input perturbation. The GARCH/regime techniques are not
relitigated as generators; that distinction is owned by the architecture readiness
audit and is out of scope here. This section ratifies only the **LLM = support,
not alpha** reframe.

## Non-goals

- Ultra-HFT sub-millisecond execution.
- Market making at scale.
- Regulated derivatives trading.
- **Real-money execution, KYC, exchange API keys, withdrawals.** Out of scope
  for this project. Terminal mode is continuous paper-trading on **real
  market data** with **simulated** fills. A follow-up project would integrate
  multi-venue real-money execution and multi-platform data APIs.
- **Tax reports / lot accounting.** Out of scope. Operator reports focus on
  performance visibility, not tax compliance.

## Project scope boundary

This project has an explicit finish line. What it ships:

- Rust-native trading agent running locally on real Binance (and later
  additional-venue) market data.
- Paper trading with **simulated fills**, end-to-end, on a 24/7 continuous
  basis in v3.
- Multi-agent decision pipeline (analysts → debate → trader → risk → PM)
  with full audit ledger and persistent reflection memory.
- Operator success reports proving "is this working?" — equity curve,
  Sharpe / Sortino / drawdown, strategy attribution, system health.

What it does **not** ship, and what becomes a **follow-up project** once v3
is stable:

- Real-money order execution on any venue.
- KYC, exchange API key management, withdrawal flows.
- Multi-platform real-money integration (CEXes + DEXes + custody).
- Tax lot accounting, FIFO / specific-lot, jurisdictional reports.

The reasoning: real money introduces a large surface of compliance, key
management, and operational risk that is best tackled **after** the core
trading intelligence is proven on real data in paper. Splitting the work
across two projects keeps both scopes honest.

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
| v2.5  | DL forecaster portfolio (**RETIRED 2026-05-22**) — 4-phase programme: phase 1 [v2.5 TCN](v25-tcn-overlay/feature.md) shipped with F4 verdict @ 1h horizon; phase 2 [v2.5a PatchTST](v25a-patchtst-overlay/feature.md) shipped 2026-05-22 with F4 verdict @ 24h horizon (Sharpe-delta +0.006 vs v1 baseline, LOWER than retired TCN); phases 3+4 ([v2.5b vanilla Transformer](v25b-transformer-overlay/feature.md) + [v2.6 bake-off](v26-forecast-bakeoff/feature.md)) deprecated without shipping (joint F4-F4 prior exhausted EV). Umbrella: [`v25-dl-forecast-overlay`](v25-dl-forecast-overlay/feature.md) deprecated. Retrospective: [`spec/dev-notes/v25-dl-journey-retrospective-2026-05-22.md`](dev-notes/v25-dl-journey-retrospective-2026-05-22.md). | **TERMINAL — no production deployment**; v2.5-era DL approaches do not extract +0.10 Sharpe-delta on hourly crypto OHLCV at 1h/24h log-return horizon. Research budget pivoted away. |
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

### DL/RL models

- ~~Forecaster: lightweight Transformer or TCN on multivariate features.~~
  **Retired 2026-05-22** — the v2.5 4-phase DL forecaster programme (TCN +
  PatchTST + planned Transformer + bake-off) reached terminal F4 verdict
  across two model families with no Sharpe-delta unlock (see § Strategy
  ladder v2.5 row). Research budget pivoted to **σ-prediction**
  ([`v3-volatility-forecaster`](v3-volatility-forecaster/feature.md) —
  also retired 2026-05-22 after MODEL-BROKEN / NO-ALPHA verdict) and
  **LLM-as-forecaster** ([`v3-llm-forecaster`](v3-llm-forecaster/feature.md),
  shipped-partial 2026-05-22; Wave D paused on `ANTHROPIC_API_KEY`).
- Regime classifier: HMM or small LSTM for trend/range/crash regimes.
  Cross-link to draft lane [`v3-regime-classifier`](v3-regime-classifier/feature.md)
  (analyst-only spec; activation-gated).
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

> **Reframed 2026-05-30 — the LLM is a SUPPORT pillar, not the alpha source.**
> See [§ Pillar stack — core vs support](#pillar-stack--core-vs-support-ratified-2026-05-30).
> The dual-tier model use below describes the LLM's *runtime mechanics*
> (providers, tiers, cost controls); its *role* is bounded to regime narration,
> lesson summarization, robustness-report explanation, and statistical-tie-break —
> never the primary trading-edge generator. The empirical basis is three retired /
> deferred alpha-engine-by-prediction bets (TCN/PatchTST, GARCH-σ, LLM-forecaster).

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
| `research`    | Hypothesis + backtest scenario file in `spec/<slug>/feature.md`              | analyst                   |
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
matching feature brief in `spec/v0-paper-sma/feature.md`.

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
- **v3 (continuous paper + success reports)** — 90 days continuous
  paper-trading on real Binance market data with **simulated** fills,
  weekly auto-generated operator success reports, lesson-card memory
  demonstrably accumulating, uptime > 99%, zero risk-limit breaches,
  LLM cost inside the v2 monthly budget. **This is the terminal state
  for this project.** A follow-up project picks up from here for
  multi-platform real-money integration.

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
- [x] **Live trading horizon (2026-04-19):** paper-trading on real data
  is the terminal mode for THIS project. No real-money execution, no KYC,
  no withdrawal flows. Real-money + multi-platform integration is a
  **separate follow-up project** kicked off only after v3 continuous-paper
  is stable. See [Non-goals](#non-goals) and [Project scope boundary](#project-scope-boundary).
- [x] **Second operator (2026-04-19):** single-operator forever. No auth,
  no RBAC. Anyone else can look over the operator's shoulder.
- [x] **Tax / reporting (2026-04-19):** no tax reports, no lot accounting.
  Operator reports focus instead on **program success visibility** —
  equity, Sharpe / Sortino / drawdown, strategy attribution, system
  health. See [Operator success reports](#operator-success-reports).
- [x] **DR / backups (2026-04-19):** local snapshots only (daily
  `sqlite3 .backup` + weekly Parquet rsync). Zero monthly cloud spend
  until the project is complete. RPO 24h, RTO ~1h manual. Off-site sync
  and continuous WAL streaming are follow-up-project concerns.

---

## Cockpit information architecture

The cockpit (live + fixtures bins) is the operator's one-screen view of
the system. From v0 it has been a single-page layout: tape + positions
+ P&L + kill switch. The
[`lumen-design-adoption`](lumen-design-adoption/feature.md) initiative
splits it into a **navigated multi-screen shell** so trading data and
operations data don't share the primary scan.

### Navigation surfaces (terminal)

The product target — what the cockpit looks like at the end of the
adoption initiative — is a left-sidebar shell with these screens:

- **Home** — primary trading view. Active strategies (summary), open
  positions, recent fills (tape / AgentFeed), realised + unrealised
  P&L. The view the operator looks at by default. (Phase 2.)
- **Charts** — per-symbol price chart with buy/sell markers from the
  audit ledger. Symbol selector at the top. (Phase 2.)
- **Strategies** — per-strategy detail: read-only params, recent
  signal events, equity-since-deploy sparkline. Reachable via the
  sidebar or by clicking a strategy row on Home. (Phase 3.)
- **Risk / Limits** — current per-venue exposure vs caps, daily loss
  limit consumed, kill threshold proximity gauge. (Phase 3.)
- **Audit / Journal** — full ledger browser with filter row and
  pagination; per-row click opens the existing transaction modal.
  (Phase 3.)
- **Debug** — operations chrome the operator only checks
  occasionally: kill switch, latency badge, market-health detail
  per venue, server-time detail, version, logs/metrics output. The
  operator can ignore this screen during normal trading. (Phase 2.)
- **Backtest** (separate `viewer` binary) — KPI strip + equity curve
  + drawdown band over the existing markdown body. Single-binary
  scope; not in the cockpit's sidebar. (Phase 4.)
- **Right-rail Assistant** — opt-in collapsible panel for the v2
  LLM strategy. Reserved as a column-track in the shell grid;
  hidden when v2 LLM is not enabled. (Phase 6, gated on v2 LLM.)

### Why this IA

The single-page v0 cockpit conflates three operator modes — "is the
system trading sensibly", "is the system healthy", and "what
specifically just happened" — onto one scan. Splitting them lets:

- The trading view stay free of operations chrome (no kill button
  next to the P&L card) so the operator's normal scan is uncluttered.
- The operations view live in one place (Debug) so reconnect events,
  latency, and market-health are findable when something feels off.
- The detail screens (Strategies / Risk / Audit) surface backend
  data that already exists but had no UI surface in v0/v0.5/v1/v1.5b.
- The chart surface answer the natural cross-check — "did my strategy
  buy at a sensible price?" — that the audit ledger has the data
  for but no visual surface today.

### What stays out of the cockpit IA

These are covered by other surfaces and **do not** become cockpit
screens:

- **Order entry** — paper-trading product; no order ticket / order
  book / watchlist. Universe is config-driven, not UI-driven.
- **Configuration editor** — `config/agent.toml` is hand-edited; the
  cockpit never writes config. (Risk and execution-mode toggles in
  Phase 5 are exceptions ratified there.)
- **Multi-account / multi-tenant views** — single-operator product
  forever (per the v3 success-metric scope boundary).

### Phasing reference

The IA above lands across Phases 2 / 3 / 4 / 5 / 6 of the
[lumen-design-adoption](lumen-design-adoption/feature.md) master
roadmap. Phase 1 (shipped 2026-05-04) ships the design tokens, Tier 1
chrome, and the always-visible status bar that anchors the bottom of
every screen above. The product spec doesn't pin per-phase scope
boundaries — those live in the master roadmap and the per-phase briefs.

---

## Operator success reports

The operator's question is always "is this working?" — and the reports are
the answer. Auto-generated at a regular cadence (weekly default,
configurable) and written as dated markdown under `spec/operator-success-reports/reports/`
with linked plots in `spec/operator-success-reports/reports/artifacts/`.

### What every report contains

- **Headline** — one number: cumulative return since inception vs a
  BTC buy-and-hold baseline.
- **Equity curve** — since-inception + last-7-days.
- **Risk metrics** — Sharpe, Sortino, Calmar, max drawdown + recovery
  time.
- **Strategy attribution** — per-strategy P&L, trade count, win rate,
  avg trade P&L. (v0.5+, once multiple strategies exist.)
- **Memory highlights** — top lesson cards the trader retrieved this
  week; which correlated with wins vs losses. (v1+, once the memory
  loop runs.)
- **System health** — uptime, kill-switch trips, clock-skew events,
  feed reconnects, LLM spend vs budget.
- **What changed** — any strategy swaps, config changes, stage
  promotions (research → paper) during the period.
- **Open risks** — drawdown approaching limit? LLM budget approaching
  cap? A strategy showing decay? Surface these at the top.

### Generation cadence

- v1+: weekly on Monday 00:00 UTC.
- On-demand via `cargo run --bin reports -- --period 7d`.
- Triggered immediately on kill-switch trip with incident context
  attached.

### Consumer

The single operator (you). No distribution list, no email pipeline.
Reports are browsable markdown that the cockpit's `viewer` binary (v0.5
scope) can render inline.

---

## Explicitly NOT adopted from TradingAgents

- **LangGraph / Python runtime** — replaced by Rust + tokio actor pattern.
- **Equity-market focus** — we are crypto-only; tickers are exchange/symbol
  pairs, not stock symbols.
- **Single-shot session model** — we add persistent memory and reflection.
- **Synchronous debate orchestration** — analyst layer fans out in parallel.

## Changelog

- 2026-05-30 (analyst, monte-carlo-robustness-lane M0): ratified operator Q4 —
  added top-level **§ Pillar stack — core vs support**. CORE = quantitative
  strategy + Monte-Carlo robustness (uncertainty quantification, resampling not
  prediction) + (future) deterministic learning loop + risk/ledger moat. SUPPORT
  = the LLM pillar (regime narration, lesson summarization, robustness-report
  explanation, statistical tie-break), explicitly **NOT the alpha source**.
  Empirical basis tabled: three retired/deferred alpha-engine-by-prediction bets
  (v2.5 TCN+PatchTST F4×2 + deprecated v2.6 bake-off; v3 GARCH-σ NO-ALPHA; v3
  LLM-forecaster Wave-D-deferred LOW-MEDIUM prior). Cross-linked the in-flight
  first slice (`monte-carlo-bootstrap-path-generator` C1 +
  `strategy-robustness-harness` C2) and the direction note. Added a reframe
  banner to § LLM strategy so the dual-tier mechanics no longer read as
  alpha-engine framing. GARCH/regime-as-generator distinction deliberately NOT
  relitigated here (owned by the architecture readiness audit).
- 2026-05-04 (analyst, post-Phase-1 ship): added new top-level
  section **"Cockpit information architecture"** capturing the
  terminal product IA — left-sidebar shell with Home / Charts /
  Strategies / Risk / Audit / Debug screens, separate viewer Backtest
  surface, reserved right-rail Assistant for v2 LLM. The IA lands
  across Phases 2 / 3 / 4 / 5 / 6 of the
  [`lumen-design-adoption`](lumen-design-adoption/feature.md) master
  roadmap (revised same day from 4 to 6 phases at operator request).
  Phase 1 (shipped) provides the design tokens + Tier 1 chrome +
  status-bar anchor that every screen sits on.
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
  plug-in-shaped trait. Analyst kicked off to draft `spec/v0-paper-sma/feature.md`.
- 2026-04-19 (operator + analyst): v0 delivered and verified PASS (all 35
  tasks, 124 tests green, deterministic backtests, Prometheus live). Final
  four Open decisions resolved — live/KYC out of scope (paper on real data
  is terminal state; real-money is a follow-up project), single-operator
  forever, no tax/reporting (replaced by operator success reports focused
  on "is this working?"), local-only DR with zero cloud spend. Added
  `## Non-goals` expansion, `## Project scope boundary`, `## Operator
  success reports` sections; revised v3 success metric to continuous-paper
  terminus.
- 2026-04-17 (developer): updated stale `cala-ledger` references to `sqlx-ledger`
  per architect decision in `spec/architecture.md`. `sqlx-ledger` on SQLite is
  the confirmed v0 audit-ledger substrate.
