# External code patterns — survey + borrowable shapes (2026-05-17)

Read three external projects in the project's adjacent space and captured
concrete patterns worth borrowing (or explicitly avoiding). Operator-
directed survey; not a feature spawn — this is the record so future
analyst/architect spawns can reference it.

## Why we read these

Web search confirmed the project's stated differentiator
([`spec/product.md` § Differentiator](../product.md#differentiator) —
*"persistent memory + audit"*) is unoccupied ground when you require
**Rust-native + double-entry audit ledger + multi-agent LLM + reflection
memory** in the same codebase. But adjacent projects each occupy
*part* of that intersection, so their patterns are worth borrowing
deliberately.

## The three projects

### 1. [Barter-rs](https://github.com/barter-rs/barter-rs) — Rust, event-driven, has AuditStream

Source files read:
- [`barter/src/engine/audit/mod.rs`](https://github.com/barter-rs/barter-rs/blob/main/barter/src/engine/audit/mod.rs)
- [`barter/src/engine/audit/state_replica.rs`](https://github.com/barter-rs/barter-rs/blob/main/barter/src/engine/audit/state_replica.rs)

Key types:

```rust
pub struct AuditTick<Kind, Context = EngineContext> {
    pub event: Kind,
    pub context: Context,
}

pub enum EngineAudit<Event, Output> {
    FeedEnded,
    Process(ProcessAudit<Event, Output>),
}

pub struct ProcessAudit<Event, Output> {
    pub event: Event,
    pub outputs: NoneOneOrMany<Output>,
    pub errors: NoneOneOrMany<UnrecoverableEngineError>,
}

StateReplicaManager::new(
    updates: impl Iterator<Item = AuditTick<...>>,
)
```

Pattern: **producer enqueues via channel; consumer wraps the channel
as `Iterator<Item = AuditTick<…>>`**. The replica is generic over the
Iterator, so the channel primitive (`mpsc`, `broadcast`, crossbeam) is
decoupled from the consumer logic. Events are mutably destructured —
no clone-heavy data path.

### 2. [TradingAgents (TauricResearch)](https://github.com/TauricResearch/TradingAgents) — Python, multi-agent LLM trading

Source file read:
- [`tradingagents/agents/researchers/bull_researcher.py`](https://github.com/TauricResearch/TradingAgents/blob/main/tradingagents/agents/researchers/bull_researcher.py)

Pipeline: 7 LLM-powered agent roles (Fundamentals, Sentiment, News,
Technical analysts → Bull/Bear researchers → Trader → Risk manager →
Portfolio manager). LangGraph orchestrates the DAG.

Bull/Bear researcher pattern:

| Element | Detail |
|---------|--------|
| State carrier | Python dict: `state["market_research_report"]`, `state["sentiment_report"]`, `state["news_report"]`, `state["fundamentals_report"]`, `state["debate_history"]`, `state["bull_history"]`, `state["count"]` |
| Output format | String prefix: `"Bull Analyst: {llm_response}"` |
| Loop control | `state["count"]` increments per debate round; outer LangGraph node decides when debate ends |
| Adversarial input | Bull researcher reads the **prior bear argument** so it can refute |
| Persistence | Decision log appends to `~/.tradingagents/memory/trading_memory.md`; per-ticker SQLite at `~/.tradingagents/cache/checkpoints/<TICKER>.db` |

### 3. [AgenticTrading (Open-Finance-Lab)](https://github.com/Open-Finance-Lab/AgenticTrading) — Python, memory-augmented DAG

Architecture (from README): Neo4j graph + secondary vector store. Central
`Memory Agent` hub owns both. Communication protocols layered: MCP
(lifecycle), ACP (feedback), A2A (peer coordination).

Execution loop (pseudocode):

```
loop {
  query = user_strategic_intent()
  dag = dag_planner.generate(query)
  for task in dag.topological_sort() {
    context = memory_agent.retrieve(task)
    agent = agent_pool.select(task.type)
    result = agent.execute(task, context)
    memory_agent.store(
      execution_trace = result,
      embeddings = vectorize(result),
      relationships = connect_to_prior_tasks,
    )
  }
  audit_agent.analyze_performance()
  memory_agent.update_strategy_params()
}
```

## Three concrete patterns worth borrowing

### Pattern A — `AuditTick<Event, Context>` consumer envelope

**Where it lands:** `crates/audit` external API.

Today `crates/reflection` reads audit state via tap-style write hooks
(`exec::on_trade_close` calls `ReflectionWriter::push(...)`). Future
consumers (Lab `Trail` screen at Phase D, v2.6 bake-off, v3
success-reports) would each need their own tap.

Barter's pattern flips this: producer (audit) emits `AuditTick`s into a
queue; consumers wrap the queue as `Iterator` and rebuild whatever
state they need. **Single source of truth, multiple replicas.**

For this project: an additive Rust enum
`AuditTick<EventKind, AuditContext>` (~50 LOC) over the existing
`journal_transactions` + `strategy_events` tables. Read-direction
broadcast. `crates/reflection` becomes a consumer instead of a write
target. UI `Trail` becomes another consumer. Hot path unaffected.

Formalised as **[ADR-0031](../architecture/adr/0031-audit-tick-consumer-envelope.md)**.

### Pattern B — `TradingState` struct (replace ad-hoc parameter threading)

**Where it lands:** future v2.x LLM enhancement (not v2 LLM ship; v2
already shipped 2026-05-13).

TradingAgents passes a single state dict through the agent pipeline.
In Rust idiom that becomes a `struct TradingState { fundamentals,
sentiment, news, technical, debate: Vec<Argument>, count: u32, … }`.
Each agent destructures, mutates its slice, returns the struct.

Maps cleanly to the existing v2 LLM `crates/llm` provider trait — the
trait already takes a request + returns a response; promoting to a
mutable state struct is incremental.

Worth queueing as **v2.x backlog entry** (not yet promoted).

### Pattern C — Adversarial researcher debate

**Where it lands:** v2.6 forecast bake-off + v2.x LLM debate.

TradingAgents has bull/bear researchers explicitly arguing against each
other. The Bull reads the Bear's last argument; the Bear reads the
Bull's last argument; iteration count tracked separately.

Two direct applications in this project:

1. **v2.6 bake-off** — TCN says Up, PatchTST says Down, vanilla
   Transformer says Flat. An LLM-arbiter reads all three forecasts +
   the operator's strategy params and produces a tie-break + a
   reasoning trace that lands in the audit ledger.
2. **v2 LLM debate** — same shape as TradingAgents bull/bear, but
   adapted to the project's existing `crates/llm` trait. Adversarial
   debate produces measurably higher-quality decisions than a single
   LLM call (TradingAgents' published evidence; not independently
   replicated here).

## Three things to NOT borrow

| Tempting | Why it's wrong for this project |
|----------|----------------------------------|
| AgenticTrading's Neo4j graph layer | Overkill at this scale. SQLite + `crates/reflection` lesson cards + vector embeddings (already on roadmap via `crates/reflection/src/embedding.rs`) cover the same ground at one-tenth the operational complexity. |
| TradingAgents' markdown-only decision log | The project's `crates/audit` double-entry ledger is structurally richer (balance enforcement + per-symbol attribution + venue split). Downgrading to markdown would lose load-bearing guarantees. |
| LangGraph checkpoint resume | Python crutch for a problem the project has already solved cleanly via `crates/replay-cache/` (SQLite WAL + canonical-JSON SHA-256 keys). Rust async + the replay cache handle determinism without DAG orchestration overhead. |

## Backlog implications (added to `spec/backlog.md` Queue)

- **`audit-tick-consumer-envelope` (architecture follow-up)** — formalised
  in ADR-0031; implementation queued.
- **`v2x-trading-state-bus` (v2 LLM evolution)** — state-struct refactor
  of the agent pipeline. Not yet promoted to feature.
- **`v26-bakeoff-llm-arbiter` (v2.6 enhancement)** — adversarial LLM-
  arbiter for the bake-off retirement decision.

## Validation of the differentiator

The original `spec/product.md ## Differentiator` line:

> *Tentative bet: persistent memory + audit.*

After reading the three projects:

- Rust + audit ledger: **Barter-rs** has audit but no double-entry; no LLM.
- LLM multi-agent: **TradingAgents** has the agent shape but no Rust + no
  audit ledger.
- Reflection memory: **AgenticTrading** has the loop but in Python with
  Neo4j.

**No project occupies the full intersection.** The bet is real; the
borrowable patterns above sharpen specific layers without changing the
overall bet.

## Sources

- [Barter-rs main repo](https://github.com/barter-rs/barter-rs) — audit module read directly.
- [TradingAgents main repo](https://github.com/TauricResearch/TradingAgents) — bull_researcher.py read directly.
- [AgenticTrading main repo](https://github.com/Open-Finance-Lab/AgenticTrading) — README + architecture diagram.
- [NautilusTrader](https://github.com/nautechsystems/nautilus_trader) — surveyed (closest Rust analog; no LLM/audit-ledger overlap).
- [LLM-TradeBot](https://github.com/EthanAlgoX/LLM-TradeBot) — surveyed.
- [TradingAgents-crypto](https://github.com/auronsun/TradingAgents-crypto) — surveyed (crypto fork of TradingAgents).
- Cryptocurrency Prices Forecasting Using LSTM, CNN, Transformer, TCN, and Hybrid Model — [IEEE](https://ieeexplore.ieee.org/document/11252474).
- TFT for multi-horizon crypto forecasting — [PMC](https://pmc.ncbi.nlm.nih.gov/articles/PMC11605417/).
