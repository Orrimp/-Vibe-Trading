---
slug: architecture
status: in-progress
owner: architect
updated: 2026-04-29
---

# Architecture — Crypto Trading Agent

> Draft scaffold. The architect agent fills this out after the analyst has
> produced initial product requirements and at least one feature brief.

## Workspace layout (proposed)

```
trading/
├── Cargo.toml            # virtual workspace
├── crates/
│   ├── core/             # package name: trading_core — domain types:
│   │                     # Symbol, Order, Position, Signal (see "Naming
│   │                     # conventions" for why the package is not `core`)
│   ├── data/             # market-data ingestion, storage, replay
│   ├── features/         # feature engineering, indicator library
│   ├── models/           # DL/ML models (candle/burn/tract) + training
│   ├── llm/              # LLM clients, prompt caching, tool schemas
│   ├── risk/             # risk engine, position sizing, kill switches
│   ├── strategy/         # strategy runtime, signal combination
│   ├── exec/             # exchange clients, order routing, paper-trade
│   ├── backtest/         # backtest engine (bin target: backtest)
│   ├── audit/            # double-entry ledger of decisions/orders/fills/PnL
│   ├── cost/             # cost telemetry — LLM tokens, infra, data feeds (Q4)
│   ├── ui/               # iced desktop app — ops cockpit + backtest viewer
│   │                     # (bin targets: cockpit, viewer)
│   └── agent/            # top-level orchestrator (bin target: trading)
```

## Naming conventions

### Crate package names vs stdlib collisions — confirmed 2026-04-17

The workspace's foundation crate carries the package name **`trading_core`**,
not `core`. Rust 2024's prelude imports resolve bare `core::` paths to the
stdlib `::core::` crate, but when a workspace member is *itself* named
`core` the two names collide inside any compilation unit that pulls it in
directly — most visibly in `rustdoc` doc-test harnesses and in macro-expanded
code that emits `::core::fmt` / `::core::write` (e.g. `thiserror::Error`).
A per-crate `doctest = false` escape hatch silences `cargo test -p <crate>`
but **does not** protect `cargo test --workspace --doc`, which bypasses the
flag and fails with `E0433` errors inside macro expansions.

**Rule:** no workspace member may take a package name that shadows a Rust
stdlib crate (`core`, `alloc`, `std`, `test`, `proc_macro`). The foundation
crate's package name is `trading_core`. Imports across the workspace read
`use trading_core::{Symbol, Order, …};`.

**Directory name:** the crate directory stays `crates/core/`. The package
name `trading_core` is what appears in `[package] name = ...` and in every
consumer's `[dependencies]`. The directory name does not affect imports or
collide with anything, and renaming it would force a touchy path update
across every `Cargo.toml` for a purely cosmetic gain. If later we want a
1:1 mapping between directory and package name we can rename the directory
in one PR — until then the single source-of-truth is the `[package] name`
field.

**Alternatives considered:**

- Keep `package = "core"` and rely on `trading_core = { package = "core" }`
  aliases in every consumer — the v0 developer tried this and it works for
  `--all-targets` but breaks `--doc`. It also fails open: a new Week 2 crate
  that forgets the alias compiles against the workspace `core` and then
  breaks in confusing ways only when `thiserror` or another macro emits a
  `::core::` path. Rejected.
- Keep `package = "core"` and add `doctest = false` workspace-wide — masks
  the failing gate instead of fixing it, and still leaves the alias trap.
  Rejected.
- Rename directory to `crates/trading_core/` in the same change — cleaner
  long-term, but the extra churn (git history, Cargo.lock, every consumer's
  `path = "../core"`) is not worth it against the single-knob rename in
  `[package] name`. Deferred; revisit only if the mismatch causes friction.


## Runtime

- `tokio` multi-thread.
- Actor-ish layout: each crate exposes an owned task with typed message
  channels (`tokio::sync::mpsc`). No shared mutable state across crates.

## Data flow

```mermaid
flowchart LR
  feed[Exchange feed] --> data
  data --> features
  features --> models
  features --> llm
  models --> strategy
  llm --> strategy
  strategy --> risk
  risk --> exec
  exec --> feed
```

## ML / DL

_Architect: pick `candle` vs `burn` vs `tract`+ONNX once the first model is
chosen. Default assumption: `candle` for prototyping, ONNX via `tract` for
serving production-trained models._

## LLM integration

_Architect: define when LLMs are called (per-bar? per-regime-change? on-demand
tool use?), token budget, caching strategy. Starting assumption: LLM called on
regime-change events only, cached system prompt, tool-use for structured
signals._

## Risk engine

- Hard limits encoded in Rust types (can't compile an order that violates them).
- Kill switch file (`.halt`) + heartbeat.
- Daily P&L stop, per-symbol exposure cap, max drawdown stop.

## Strategy registry & hot-loading

Strategies are first-class plug-ins. The runtime owns a typed registry of
active strategies and routes data/signals through each.

### v0 — clean trait shape, no hot-load (compiled-in)

```rust
pub trait Strategy: Send + Sync {
    fn id(&self) -> StrategyId;
    fn on_bar(&mut self, bar: &Bar) -> Vec<Signal>;
    fn on_tick(&mut self, tick: &Tick) -> Vec<Signal>;
    fn config_schema() -> serde_json::Value where Self: Sized;
}
```

Strategies are compiled in. The registry is a
`HashMap<StrategyId, Box<dyn Strategy>>` populated at startup from
`config/agent.toml`. **This shape is the contract** — it does not change to
support hot-loading later. v0 ships only `sma_crossover`.

### v0.5 — config-driven composition (hot-load A)

A `ComposedStrategy` implements the `Strategy` trait; its body is a tree of
indicator + rule nodes assembled at runtime from TOML. Example:

```toml
[strategies.btc_macd_rsi]
kind   = "composed"
signal = "macd_cross(12,26,9) AND rsi(14) < 35"
size   = "fixed_fraction(0.1)"
```

A file watcher on `config/strategies/` reloads on change; the registry
swaps the `Box<dyn Strategy>` atomically. No process restart. This pattern
covers ~70-80% of research iteration without leaving Rust.

### v1+ — WASM plugins (hot-load B)

For strategies that need genuinely custom logic (custom DL inference,
non-trivial state machines, off-the-shelf research code from Python via
Pyodide), compile to WASM and load via `wasmtime`.
- **Sandboxed** — a buggy strategy cannot crash the agent or leak memory.
- **Language-agnostic** — Rust, AssemblyScript, eventually Python.
- Tradeoff: slight per-tick perf overhead vs compiled-in; deploy step per
  strategy.

### Explicitly NOT chosen

- **Native `.so` / `.dylib` via Rust dynamic libs** — the Rust ABI is
  unstable; subtle ABI-mismatch crashes in production.
- **Embedded scripting** (Rhai, Lua, Rune) — loses Rust's type-safety story
  and adds a parallel error-handling vocabulary.

### Lifecycle integration

Every strategy registry change (load, swap, unload, demote) emits a journal
entry to the audit ledger. Combined with the [strategy lifecycle gates in
product.md](product.md#strategy-lifecycle--promotion-gates), this means the
ledger always answers "which strategies were active when this trade fired?".

### v0.5 — strategy-event journal schema (Q1) — confirmed 2026-04-19

**Decision:** dedicated `strategy_events` SQLite table, written by a new
`audit::journal::strategy_event(..)` function inside the same `sqlx`
transaction machinery used for fills. **Not** reused into `journal_entries`.

**Rationale:** `journal_entries` is the double-entry ledger — rows carry
debit / credit money amounts and reconcile to `Σ debits == Σ credits` per
transaction. Strategy lifecycle events (load / swap / unload / reject)
carry no money — they are operator / system events. Mixing them via a
`kind` discriminator column forces every consumer of `journal_entries`
to filter, complicates the reconciler's invariant proof, and muddies
future non-monetary events (kill-switch trips, mode changes, cost-budget
alerts). A sibling table with its own schema is the cleanest expression
of the two-kinds-of-thing distinction.

The v0 `registry_event` function currently writes zero-amount memo rows
into `journal_entries` against `equity:opening_balance`. v0.5 replaces
that path with `strategy_event` writes to the new table; the old memo
rows remain in the ledger as history. No schema migration is needed for
the existing table.

**Schema** (`migrations/0003_strategy_events.sql`, approximate):

```sql
CREATE TABLE strategy_events (
    id            TEXT PRIMARY KEY,       -- uuid v4
    ts            TEXT NOT NULL,          -- RFC3339
    kind          TEXT NOT NULL,          -- 'Load' | 'Swap' | 'Unload' | 'Reject'
    strategy_id   TEXT,                   -- nullable for Reject when id unparsable
    old_hash      TEXT,                   -- sha256 hex, 64 chars
    new_hash      TEXT,                   -- sha256 hex, 64 chars
    source_path   TEXT,                   -- repo-relative
    operator      TEXT NOT NULL DEFAULT 'system',
    error_code    TEXT,                   -- Reject only
    error_summary TEXT                    -- Reject only, short human message
);
CREATE INDEX strategy_events_ts_idx ON strategy_events(ts);
CREATE INDEX strategy_events_sid_idx ON strategy_events(strategy_id, ts);
```

**Writer** (in `audit::journal`):

```rust
pub enum StrategyEventKind { Load, Swap, Unload, Reject }

pub struct StrategyEventWrite<'a> {
    pub kind:          StrategyEventKind,
    pub strategy_id:   Option<&'a str>,
    pub old_hash:      Option<&'a str>,
    pub new_hash:      Option<&'a str>,
    pub source_path:   &'a str,
    pub operator:      &'a str,          // "system" in v0.5
    pub error_code:    Option<&'a str>,
    pub error_summary: Option<&'a str>,
}

pub async fn strategy_event(
    ledger: &Ledger,
    write: StrategyEventWrite<'_>,
) -> Result<(), LedgerError>;
```

**Reader** (in `audit::query`, `Decimal`/core-types-only contract):

```rust
pub async fn strategy_events_since(
    ledger: &Ledger,
    ts: Timestamp,
) -> Result<Vec<StrategyEventView>, LedgerError>;

pub async fn strategy_history(
    ledger: &Ledger,
    id: StrategyId,
) -> Result<Vec<StrategyEventView>, LedgerError>;
```

`StrategyEventView` is defined in `trading_core` alongside `FillView` /
`JournalEntryView` (no back-edge from `trading_core` to `audit`).

**Reconciliation invariant (unchanged):** the minute-boundary reconciler
walks `journal_entries` only. `strategy_events` rows do not affect
`Σ debits == Σ credits`. v0.5 test T214 (see
[spec/tasks/v05-composed-strategies.md](tasks/v05-composed-strategies.md))
asserts that running R7 + R8 integration cycles leaves the reconciler
at zero imbalance.

**Alternatives considered:**

- Reuse `journal_entries` with an `entry_kind` column plus a CHECK
  constraint — rejected; conflates balance-carrying and metadata rows,
  poisons the reconciler query, and turns a clean double-entry story
  into a filter discipline that a future developer will forget.
- Single-table polymorphism via sparse nullable metadata columns on
  `journal_entries` — rejected on the same grounds plus index bloat.

### v0.5 — registry concurrency (Q2) — confirmed 2026-04-19

**Decision:** `parking_lot::RwLock<HashMap<StrategyId, Box<dyn Strategy>>>`
for the v0.5 `StrategyRegistry`. No new dep (`parking_lot` is already
workspace-pulled). Hot path takes a read guard; the file-watcher task
takes a write guard only during swap.

**Rationale:** at 1m bar cadence the read frequency is ≤ a few per second
across all active strategies. Writes are rare (file edit → debounce →
parse → construct → swap; order of once per minute at worst during
research iteration). `parking_lot::RwLock` gives us sub-microsecond
acquire in the uncontended case, stdlib-shaped API, zero new
dependencies, and zero async-await inside the hot path — the agent's
`on_bar` fan-out stays blocking-free around the registry read.

Async-aware `tokio::sync::RwLock` is also acceptable if the hot path
later grows an `.await` point around the registry read. v0.5 does not —
`Strategy::on_bar` is a sync `&mut self` method, and the registry read
happens inside `StrategyRegistry::on_bar` which is itself sync — so
`parking_lot` is the correct instrument.

**Alternatives considered:**

- `arc-swap::ArcSwap<HashMap<..>>` lock-free hot-swap — overkill for 1m
  cadence; adds a dep and a cognitive overhead for readers familiar
  with lock semantics. Revisit in v1+ only if a tick-latency strategy
  pushes contention into the microsecond budget.
- `tokio::sync::RwLock` — introduces an unnecessary `.await` into the
  bar-close path at current trait shape. Stop-gap if we later migrate
  `Strategy::on_bar` to async.
- `std::sync::RwLock` — fine semantically; we pick `parking_lot` for
  the faster uncontended path and already-present workspace
  dependency.

### v0.5 — ComposedStrategy exit policy (Q3) — confirmed 2026-04-19

**Decision:** symmetric signal-flip only — when the rule transitions
`true → false` the strategy emits `Sell` to close. No drawdown-triggered
exit lives inside `ComposedStrategy`.

**Rationale:** a per-strategy drawdown clamp is a **risk** concern, not a
**strategy** concern. Putting it in each composed rule tree duplicates
state (last-high, drawdown counter) N-ways and makes the rule DSL less
orthogonal. The `risk` crate already owns `size_and_validate` (v0 R4.5)
and the portfolio-level `max_drawdown_stop_pct` floor; a per-strategy
drawdown limit is a natural extension — but it belongs on the
risk-limits struct, not inside the signal tree. Deferred to v1+ when
live-paper drawdowns surface as a real problem worth building for.

v0.5 `ComposedStrategy` therefore emits buy on `false → true` and sell
on `true → false`, matching v0 `sma_crossover` edge-triggered semantics.

**v1+ hook:** `risk::RiskLimits` grows an optional
`max_strategy_drawdown_pct: Option<Decimal>` field; `risk::size_and_validate`
clamps to zero (and emits a `StrategyRiskTripped` audit event) when a
specific strategy's cumulative drawdown passes the limit. Leave a
`// TODO(v1): max_strategy_drawdown` breadcrumb in the v0.5 Design section
so the developer does not invent the feature in v0.5.

**Alternatives considered:**

- Ship drawdown-triggered exit inside `ComposedStrategy` in v0.5 — bloats
  the rule tree, forces state into every node, and duplicates a risk
  concern. Rejected.
- Make it a first-class DSL node (`if drawdown(20) > 0.05 then close`) —
  possible but premature; commit after seeing real drawdown patterns in
  v1 paper trading.

### v0.5 — cockpit strategies panel layout (Q4) — confirmed 2026-04-19

**Decision:** **right column, above "Open positions"**, in a new
`StrategiesPanel` widget. The existing cockpit left column (P&L card,
latency badge, "Stop trading" button) is action-oriented; the right
column (Open positions, Live tape) is observation-oriented. Strategies
are observation-oriented (which strategies are running, what was the
last swap, how many signals in the last 60s) and pair naturally with
positions (what strategies produced the current book).

```
┌──────────────────────────────────┬─────────────────────────────────────┐
│  P&L                             │  Strategies (v0.5 new)              │
│  Feed latency                    │  Open positions                     │
│  Stop trading (destructive)      │  Live tape                          │
└──────────────────────────────────┴─────────────────────────────────────┘
```

**Rationale:** co-locating the passive strategies panel with a
destructive action (kill switch) crowds the decision surface and
creates visual competition between "look at this" and "do this".
Keeping the kill switch the biggest thing in the left column protects
the operator's muscle memory from v0.

Final widget composition (column widths, row heights, padding) is
ui-designer's call; architect only fixes the panel's position in the
column layout and the Model / Message surface (see
[v0.5 design in the feature file](features/v05-composed-strategies.md#design)).

**Alternatives considered:**

- Left column next to kill switch (analyst's initial suggestion) —
  rejected on visual-crowding grounds above.
- Full cockpit re-wireframe — rejected; v0 layout is stable and the
  operator has muscle memory on the four existing panels. Additive
  placement keeps cognitive load low.
- Separate top-level window — rejected; the strategies panel is part
  of the cockpit's live view, not a standalone tool (`viewer` binary
  is for offline reports).

### v0.5 — broadcast bus extensions (Q5) — confirmed 2026-04-19

**Decision:** the three new message types — `StrategyLoaded`,
`StrategySwapped`, `StrategyLoadError` — live in `trading_core`
alongside `Fill`, `Bar`, `PnlSnapshot`. The `agent::EventBus` (see
[dev-week2-broadcast-api-2026-04-18.md](reports/dev-week2-broadcast-api-2026-04-18.md))
gains three new `broadcast::Sender`/`Receiver` pairs, same pattern as
the existing `fills` / `positions` / `bars` / `ticks` / `pnl` / `mode`
channels.

**Rust sketch** (in `trading_core`):

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyLoaded {
    pub id:          StrategyId,
    pub hash:        [u8; 32],        // sha256 of canonicalized AST
    pub source_path: SmolStr,
    pub ts:          Timestamp,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategySwapped {
    pub id:          StrategyId,
    pub old_hash:    [u8; 32],
    pub new_hash:    [u8; 32],
    pub source_path: SmolStr,
    pub ts:          Timestamp,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyLoadError {
    pub source_path:   SmolStr,
    pub strategy_id:   Option<StrategyId>,   // None if filename-stem unparsable
    pub error_code:    SmolStr,              // e.g. "unknown_indicator"
    pub error_summary: SmolStr,              // one-line human message
    pub ts:            Timestamp,
}
```

**Bus extension** (in `agent::EventBus`):

| Channel | Type | Capacity | Description |
|---|---|---|---|
| `strategy_loaded`  | `StrategyLoaded`      | 32  | Emitted on registry `Load`. |
| `strategy_swapped` | `StrategySwapped`     | 32  | Emitted on registry `Swap`. |
| `strategy_error`   | `StrategyLoadError`   | 32  | Emitted on registry `Reject`. |

**Backpressure:** identical to the v0 pattern —
`RecvError::Lagged(n)` triggers log-and-continue in the UI subscriber;
`RecvError::Closed` surfaces as a `STRATEGIES_CONNECTION_CLOSED`
panel-error copy. Capacity is small (32) because publish rate is
bounded by file-edit cadence.

**Why `trading_core`, not `agent`:**

- `audit::journal::strategy_event` also needs these types when it
  persists the events (it converts `StrategyLoaded` into a
  `strategy_events` row). Placing them in `agent` would force `audit`
  to depend on `agent`, inverting the existing `audit ← agent`
  dependency edge.
- `ui` (cockpit strategies panel) subscribes to the bus; types in
  `trading_core` are already imported.
- `trading_core` is upstream of every other crate; no cycle.

**Alternatives considered:**

- Put them in `agent` — creates the cycle above. Rejected.
- Put them in `strategy` — forces `audit` → `strategy`, wrong
  direction (audit should be a pure sink). Rejected.
- Put them in `audit` — same cycle problem, and ties the UI's
  broadcast-bus subscriber to the audit crate just to get a type.
  Rejected.

### v1 — cross-sectional momentum resolutions (Q1–Q6) — confirmed 2026-04-29

Six open questions from
[v1-cross-sectional-momentum.md → Notes](features/v1-cross-sectional-momentum.md#notes--open-questions-for-architect).
All resolutions preserve the v0 `Strategy` trait shape and the v0.5
audit / broadcast / strategies-panel surfaces.

#### v1 Q1 — L2 book ingest: **deferred to v1.5**

**Decision:** v1 ships **klines + trades only**, identical fan-out shape
to v0/v0.5. No L2 book ingest path in v1. The
[product.md → Universe & data fidelity ladder](product.md#universe--data-fidelity-ladder)
v1 entry mentioning "L2 + funding context" is **walked back to v1.5** —
v1's two stretches (multi-symbol + first edge-candidate strategy) are
already enough; L2 adds a third memory-and-fan-out vector that the
momentum score does not consume.

**Rationale:** the v1 momentum score (R3) is close-to-close vol-adjusted
return; depth has no inputs that would exercise it. Adding L2 in v1
would force a new bus channel, a new persistence schema, and ~10× the
per-bar serde cost for zero strategy benefit. Pushed to v1.5 alongside
the other ladder stretches (Coinbase / Kraken multi-venue, 1s
aggregated trades).

**Alternatives considered:**

- L2 as observation-only at v1 (analogous to Q2 funding) — rejected;
  L2 ingest cost is dominated by serde + storage, not poll cadence,
  so "observation only" still pays the full bill.
- Ship L2 in v1 because the ladder says so — rejected; the ladder is a
  goal sketch, not a delivery contract; v1 must remain shippable.

#### v1 Q2 — Funding-rate ingest: **observation-only at v1**

**Decision:** v1 wires the Binance USDT-perpetual funding-rate REST
endpoint as a **once-per-hour poller** in a new
`crates/data/src/funding.rs`, persists to a small `funding_rates`
SQLite table (sibling to `journal_entries`), and broadcasts on a new
`funding_obs` channel. The v1 `MomentumStrategy` does **not** consume
funding — the channel exists so v1.5+ strategies and the operator
success report can read funding history without a follow-up
ingest-path build.

**Why ship the path now (not in v1.5):**

- One cheap REST poll per hour per symbol — the cost is bounded and
  pre-priced into the v1 hosting line ($40/month, unchanged).
- Validates the cross-stream-rate (per-bar bars vs per-hour funding)
  ingest pattern that v1.5 multi-venue / 1s aggregation will need.
- Operator success reports (per
  [product.md → Operator success reports](product.md#operator-success-reports))
  can show a funding column once a UI feature lands without waiting on
  a future ingest spike.
- Zero impact on the bar-close → signal hot path (the channel is
  observation-only and the strategy does not subscribe).

**Schema** (new file
`crates/audit/migrations/003_funding_rates.sql` or a sibling DB —
**architect's call to keep it inside the same SQLite file** as
`journal_entries` to preserve "ledger = single file" property):

```sql
CREATE TABLE IF NOT EXISTS funding_rates (
    id              TEXT PRIMARY KEY,        -- uuid v4
    symbol          TEXT NOT NULL,           -- e.g. "BTCUSDT" (perp symbol; spot mapping in metadata)
    funding_ts      TEXT NOT NULL,           -- RFC3339 — venue-published funding timestamp
    funding_rate    TEXT NOT NULL,           -- Decimal as TEXT (8h rate, e.g. "0.00010000")
    next_funding_ts TEXT NOT NULL,           -- RFC3339 — next scheduled funding settlement
    poll_ts         TEXT NOT NULL            -- RFC3339 — when we polled (audit)
);
CREATE INDEX IF NOT EXISTS funding_rates_symbol_ts_idx
    ON funding_rates(symbol, funding_ts);
```

**New broadcast type** in `trading_core`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FundingObs {
    pub symbol:          Symbol,           // perp symbol — operator should be aware of basis
    pub funding_rate:    Decimal,
    pub funding_ts:      Timestamp,
    pub next_funding_ts: Timestamp,
    pub poll_ts:         Timestamp,
}
```

`agent::EventBus` gains a `funding_obs` channel (capacity 32, same
backpressure semantics as `strategy_*`).

**Reader API** in `audit::query`:

```rust
pub async fn funding_rate_history(
    ledger: &Ledger,
    symbol: Symbol,
    since: Timestamp,
) -> Result<Vec<FundingObs>, LedgerError>;
```

**Alternatives considered:**

- Defer funding entirely to v1.5 alongside L2 — acceptable but punts on
  validating a cross-cadence ingest pattern that v1.5 will need
  regardless. The marginal cost of shipping the poller in v1
  (one tokio task, ~40 lines of REST client) is small.
- Build a perp-WS funding stream — rejected as overkill; funding rates
  publish at 8h cadence with a 1m forecast ticker; an hourly REST poll
  catches every settlement and the next-forecast value.

#### v1 Q3 — Long-only confirmed: **`K_long=3`, `K_short=0`** for v1

**Decision:** v1 ships **long-only spot momentum** explicitly in
`architecture.md` so future architect-rounds do not re-debate. The
analyst's `[ASSUMPTION]` (R4.3 in the v1 brief) is correct — spot
crypto on Binance / Coinbase / Kraken USDT pairs has **no native
short-sell mechanism**. Perp-based shorting belongs in v2 per the
[product.md → Universe & data fidelity ladder](product.md#universe--data-fidelity-ladder)
v2 row ("Top-25 perps (signal only, not exec)").

**Implication for the v1 strategy & risk surface:**

- `MomentumStrategy::on_bar` constructs `Vec<ProposedOrder>` with at
  most `K_long` `Side::Buy` legs, plus `Side::Sell` legs to close
  positions falling out of the top-K. **Never** `Side::Sell` to open a
  short.
- `risk::cross_sectional.k_short = 0` in the TOML schema; loader
  rejects `k_short > 0` with `unsupported_short_sizing` error code in
  v1. v1.5 may relax to "exclude these symbols from longs" semantics if
  the analyst pushes for it; perp-execution shorting waits for v2.

**Alternatives considered:**

- Ship `K_short` plumbing as "exclude from longs" in v1 — rejected;
  adds parser + sizer surface area for zero edge before the v1.5
  re-evaluation. Easier to add later than to remove.
- Allow synthetic shorts via inverse perps — out of scope per
  [product.md → Non-goals](product.md#non-goals) (no derivatives in v1).

#### v1 Q4 — Multi-venue: **single-venue (Binance) for v1**

**Decision:** v1 stays **Binance-only**. Multi-venue
(Coinbase + Kraken) is v1.5 scope. The
[universe ladder](product.md#universe--data-fidelity-ladder) v1 entry
("Top-10 USDT spot, 1m + L2 + funding context, +Kraken") is re-read as
the v1-series goal: v1 itself does the universe-size work,
v1.5 does venue multiplexing.

**Rationale:** each new venue is its own client + reconnect quirks +
fee schedule + symbol-name normalization (e.g. `BTCUSDT` on Binance
vs. `BTC-USD` on Coinbase vs. `XBT/USDT` on Kraken). v1's stretch is
multi-symbol determinism on a known venue; venue diversity is an
orthogonal stretch with its own reconciliation surface (per-venue
attribution in the ledger, cross-venue symbol mapping, latency
arbitrage controls).

**v1.5 trigger:** revisit when v1's cross-sectional momentum has
defensible Sharpe on Binance OOS. At that point the
`barter-data` v0.5 fallback (per
[Data / venues](#v0-decision--hand-rolled-binance-ws--marketdatasource-trait--confirmed-2026-04-17))
becomes the natural pick if it cleanly maps to `MarketDataSource`,
else a second hand-rolled adapter (Coinbase) lands first.

**Alternatives considered:**

- Ship Coinbase ingest in v1 alongside Binance — rejected; doubles the
  feed-test surface and forces premature decisions on cross-venue
  symbol normalization before the strategy has a defensible OOS number.
- Ship all three venues in v1 — rejected on the same grounds, harder.

#### v1 Q5 — Universe filtering: **strategy-side (pattern A)**

**Decision:** v1 strategies filter bars by symbol **internally** in
`Strategy::on_bar` rather than via a registry-level `interested_in()`
predicate. The registry's existing fan-out
(`StrategyRegistry::on_bar(&Bar) → Vec<Signal>`, see
[v0.5 strategy-event journal](#v05--strategy-event-journal-schema-q1--confirmed-2026-04-19))
is **unchanged** — every strategy sees every bar; out-of-universe bars
are a fast `match symbol { in_universe => …, _ => return Vec::new() }`
in the strategy.

**Rationale:** at v1 scale (10 symbols × 1m bars × ≤10 active
strategies), the constant cost of an `if !self.universe.contains(...)`
check is dominated by a single hash lookup per bar — sub-microsecond.
The trait stays minimal; no new method shape; v0 `sma_crossover` and
v0.5 `ComposedStrategy` continue to filter on `bar.symbol == self.symbol`
exactly as today.

**Trade-off:** registry-side filtering (pattern B) would save the
hash-lookup cost on out-of-universe bars at the cost of a new
trait method (`fn universe(&self) -> &[Symbol]`) and a two-stage
fan-out in the registry. The performance plan below
([Performance budget](#performance-budget) + R10.4) shows the v1 hot
path comfortably fits the 5ms p99 budget under pattern A; pattern B is
a future optimization if a tick-latency strategy ever stresses it.

**Implementation contract:** every multi-symbol `Strategy` impl
exposes `pub fn universe(&self) -> &SymbolSet` as an inherent method
(not a trait method) so the audit ledger and operator-success reports
can introspect it; the registry does not consume this method.

**Alternatives considered:**

- Pattern B (registry-side filtering with `fn universe()` trait method) —
  cleaner under heavy multi-strategy fan-out; rejected for v1 because
  the cost it saves is already inside budget. Promote in v2+ if a
  per-tick / per-microstructure strategy lands.
- Pattern C (hybrid: per-strategy `Vec<Symbol>` declared at registration,
  registry holds the index) — combines the worst of both (mutable
  registry index + strategy-side fallback for hot-loaded universe
  changes). Rejected.

#### v1 Q6 — `RebalanceRejected` ledger surface: **extend `strategy_events`**

**Decision:** add a new `kind = "rebalance_rejected"` variant to the
v0.5 `strategy_events` table (see
[v0.5 strategy-event journal](#v05--strategy-event-journal-schema-q1--confirmed-2026-04-19))
rather than create a parallel `decision_events` table.

**Rationale:** the existing schema already carries `error_code` /
`error_summary` columns and is operator-event-shaped. A rebalance
rejection is a **strategy-lifecycle event** (the strategy proposed an
action that the risk gate refused), not a money movement, so it
belongs alongside `Load` / `Swap` / `Unload` / `Reject`. A parallel
`decision_events` table would fork the schema for a single
non-monetary row type and complicate the operator success report's
"what happened to my strategies this week" query (it would need to
union two tables).

**No schema migration needed** — `strategy_events.kind` is a `TEXT`
column; v1 simply writes a new value. The reconciler invariant
(`Σ debits == Σ credits` over `journal_entries` only) is preserved
because `strategy_events` rows carry no money.

**v1 writer extension** (in `audit::journal`, additive):

```rust
pub enum StrategyEventKind {
    Load,
    Swap,
    Unload,
    Reject,
    RebalanceRejected,   // new in v1
}

// New helper, written from risk crate when vector validation rejects:
pub async fn rebalance_rejected(
    ledger: &Ledger,
    strategy_id: StrategyId,
    error_code: &str,           // e.g. "portfolio_exposure_breach"
    error_summary: &str,        // e.g. "proposed 0.55 > cap 0.50"
    ts: Timestamp,
) -> Result<(), LedgerError>;
```

**Reader extension** (in `audit::query`):

`strategy_history(id)` already returns all `strategy_events` rows for a
strategy. v1 callers use the `kind` field to filter to rebalance
rejections; no new reader method is required.

**Alternatives considered:**

- New `decision_events` table with one row per `Decision`
  (accepted + rejected) — rejected; doubles schema surface for one
  variant, and the v0 `Decision` model already lives in the audit log
  as the journaled fills' parent transaction (per v0
  `journal_transactions`). Rebalance rejections would be the only
  rows in `decision_events` that don't have a corresponding fill.
- Add a `kind` column with a CHECK constraint to `journal_entries` —
  rejected on the same grounds as v0.5 Q1: poisons the reconciler
  invariant and conflates monetary and metadata rows.

## Observability

- `tracing` with JSON output.
- Metrics via `metrics` + Prometheus exporter.
- Structured logs to `logs/` plus stdout.

## Disaster recovery & backups

v0 → v3 policy: **local snapshots only.** No cloud spend until the project
reaches terminal state. Confirmed 2026-04-19 ([product.md → Open decisions](product.md#open-decisions)).

- **SQLite ledger:** `sqlite3 <db> ".backup 'snapshot/<YYYY-MM-DD>-ledger.db'"`
  nightly via a tokio task in the `agent` binary. Retain 30 days; purge older.
- **Parquet archive:** historical market data lives in `data/binance/...`;
  treated as append-only. Weekly `rsync -a data/ data-snapshot/` rotation
  gives a 4-week rolling local backup.
- **Config + strategy TOML:** versioned in place under `config/`; backed up
  alongside the ledger snapshot.
- **RPO:** 24h (ledger + config). **RTO:** ~1h manual (copy snapshot,
  restart agent).

**Explicitly not in scope for this project:** off-site cloud sync (B2 / S3),
continuous WAL streaming (`litestream`), multi-region replication. Deferred
to a follow-up project triggered when real-money execution lands
([product.md → Project scope boundary](product.md#project-scope-boundary)).

Restore runbook lives at `spec/runbooks/disaster-recovery.md` (v0.5
deliverable; for v0 a section in `spec/runbooks/kill-switch.md` suffices).

## Performance budget

| Path                          | Budget      | Notes                           |
|-------------------------------|-------------|---------------------------------|
| Bar-close → signal (no LLM)   | < 5 ms p99  | Regression-tested in benches    |
| Bar-close → signal (with LLM) | < 500 ms p95| Only on regime-change triggers  |
| Backtest throughput           | > 100k bars/s | per symbol, single thread     |

## Foundation libraries

We pull in proven Rust crates rather than reinventing them. Default picks below;
the architect may override per-feature with rationale recorded here.

### Async, observability, errors
- `tokio` (multi-thread runtime), `tokio-stream`, `futures`
- `tracing`, `tracing-subscriber`, `metrics`, `metrics-exporter-prometheus`
- `thiserror` (libraries), `anyhow` (binaries only)
- `serde`, `serde_json`, `toml`, `config`

### Numerics & ML
- `rust_decimal` — **mandatory** for prices, sizes, balances, P&L. No `f64` for
  money, anywhere. (`6M+ downloads`, battle-tested).
- `ndarray`, `nalgebra` for general linear algebra.
- `polars` for in-memory DataFrame work and Parquet read/write.
- `candle` for DL prototyping; `tract` for ONNX serving.
- `linfa` for classical ML (regression, clustering) where heavier than RustQuant's `ml`.

### Money & currency types
Crypto doesn't fit ISO 4217 cleanly (BTC, USDT, stablecoins). We roll our own
`Money<C: Currency>` newtype around `rust_decimal::Decimal` in `core`, and use:
- `iso_currency` (or RustQuant's `iso`) for fiat sides only.
- Custom `Asset` enum for crypto symbols, sourced from venue metadata at startup.

Do **not** use generic money crates (`moneylib`, `moneta`, `cashmoney`) — they
assume ISO currency lists.

### Quant primitives — RustQuant ([avhz/RustQuant](https://github.com/avhz/RustQuant))
A free-time community crate; treated as a **helper, not a foundation**. Pin a
known-good version, vendor or fork if a module proves unstable. We adopt these
modules and ignore the rest:

| RustQuant module      | We use it for                                         | Lives in our crate |
|-----------------------|-------------------------------------------------------|--------------------|
| `math` (risk-reward)  | Sharpe, Sortino, Calmar, max drawdown, VaR, CVaR      | `risk`, `backtest` |
| `math` (distributions / FFT / quadrature) | Stats, characteristic functions, numerical integration | `features`, `models` |
| `math` (optimization) | Root finding, gradient descent for calibration tasks  | `models`           |
| `stochastics`         | Brownian / OU / CIR for synthetic data + Monte Carlo  | `backtest`, `data` (synthetic) |
| `time`                | Day counters, schedules, conventions for funding etc. | `core`             |
| `data`                | CSV / JSON / Parquet I/O helpers                      | `data`             |
| `iso`                 | Currency / MIC codes                                  | `core`             |
| `macros`              | `assert_approx_equal!` in tests                       | (dev-dep, all crates) |

**Explicitly NOT adopted from RustQuant:**

- `instruments` (bonds, options) — out of scope for spot crypto v0.
- `models` (rate / curve models) — fixed-income focus, not crypto.
- `ml` (linear/logistic regression, KNN) — too thin; use `linfa`/`candle`.
- `trading` (basic LOB) — we own our microstructure layer in `data` /
  `backtest`, and need it tuned for crypto venue quirks.
- `cashflows`, `portfolio` (RustQuant's) — replaced by our own typed
  primitives in `core` so risk limits are encoded in the type system.

**Risks of depending on RustQuant**
- Author flags it as a free-time project; API churn is likely.
- Mitigations: pin exact version, run their version through `cargo audit` /
  `cargo deny`, isolate behind thin adapter modules in our crates so a
  swap-out is a one-file change.

### Order book & matching engine

For paper-trade fills and high-fidelity backtests we need a credible LOB +
matching layer. A `MatchingEngine` trait in `backtest` isolates the choice so
the implementation can swap without touching `strategy` / `risk` / `exec`.

#### v0 decision — simple paper engine (no LOB) — confirmed 2026-04-17

v0's SMA emits only **bar-close market orders against 1m klines**. A full LOB
engine has no inputs that would exercise it and would consume the 2-week
budget. v0 ships a `PaperEngine` in `backtest` parameterized by:

- `slippage_bps` (default `2`) — buy fills at `bar.close * (1 + bps/10_000)`,
  sell fills at `bar.close * (1 − bps/10_000)`.
- `taker_fee_bps` (default `4`) — applied to notional, booked to
  `expense:fees:taker` per the ledger schema in `audit`.
- optional bar-VWAP fill price (toggle) for sensitivity runs.
- deterministic seeded RNG for any tie-break / jitter.

**Rationale:** scope fit — SMA has no limit orders, partial fills, or queue
position to model; shipping an LOB first would be speculative architecture.

**Alternatives considered:** `orderbook-rs` (lock-free, async), `matchcore`
(state-machine), `rust_ob` (minimal) — **deferred to v0.5** when limit orders,
IOC/FOK flags, and partial fills become real. Decision gate: v0.5 spike picks
one based on partial-fill fidelity, post-only / IOC / FOK support, and
slippage/fee hook cleanliness.

The trait is frozen now so swapping to an LOB implementation is an additive
change (`Box<dyn MatchingEngine>`), not a refactor.

### Technical analysis

Don't reinvent. Survey:

- `kand` — pure-Rust TA-Lib clone, breadth comparable to TA-Lib (default pick).
- `quantedge-ta` — streaming-first; important for live bars where we must update
  per-tick instead of per-bar. Likely complements `kand`.
- `rust_ti` — 70+ indicators if `kand` lacks any.
- `mantis-ta` — composable indicators + strategy primitives; evaluate as
  inspiration for our `features` crate API.

Plan: default `kand` (batch) + `quantedge-ta` (streaming), thin adapters in
`features`, no direct dependency from `strategy`.

### Pre-trade risk

- `openpit` — embeddable pre-trade risk SDK. Evaluate as an alternative to
  hand-rolling the risk crate. **Concern:** our risk engine relies on Rust
  *type-level* limits (illegal orders fail at construction). If `openpit` is
  runtime-checks-only, we use it as a second-line check, not the primary gate.

### Audit & ledger

The "every decision auditable" goal in [product.md](product.md) is implemented as
a **double-entry ledger** of decisions, intents, orders, fills, and P&L
attribution. Lives in the `audit` crate.

#### v0 decision — raw `sqlx` + `SQLite` + in-repo migrations — reconciled 2026-04-19

**Choice:** raw `sqlx` against an embedded `SQLite` file, with a small set of
schema migrations (`crates/audit/migrations/`) authored by us. The `audit`
crate exposes a `Ledger` handle, a `chart_of_accounts` bootstrap (13
accounts), and a balanced-double-entry `journal` module (`post_fill`,
`registry_event`, `post_cost`) that enforces `Σ debits == Σ credits` per
transaction. A read-only `audit::query` surface returns `Decimal` / `core`
types — no `sqlx` rows leak to consumers.

**Why not `sqlx-ledger`:** Week 1 wiring discovered `sqlx-ledger` v0.11.14
is **Postgres-only** — its `Cargo.toml` gates the store behind
`sqlx/postgres`, no SQLite path compiles. Taking it would have forced
Postgres as an ops dep and broken the single-binary deploy goal locked in
[product.md → Project scope boundary](product.md#project-scope-boundary).
We retained the **semantics** (double-entry, balanced-per-txn, append-only
journal, idempotent chart bootstrap) and dropped only the dependency — the
substitution is purely additive and keeps the `audit::query` public API
unchanged.

**Shape of the substitute** (actual code in `crates/audit/src/`):

- `ledger.rs` — `Ledger::open(db_path)` + `sqlx::migrate!` runs the
  in-repo migrations; `:memory:` path for tests.
- `bootstrap.rs` — `chart_of_accounts()` inserts the 13 v0 accounts
  idempotently (`INSERT OR IGNORE`).
- `journal.rs` — `post_fill(&Fill)` writes one `journal_transactions`
  row plus N `journal_entries` rows inside a single `sqlx` transaction;
  buy / sell / fee legs balance to the satoshi. `registry_event()` and
  `kill_switch_tripped()` write zero-amount memo rows against
  `equity:opening_balance` to preserve the balance invariant.
- `query.rs` — `cash_balance`, `realized_pnl_since`, `total_fees`,
  `account_list`, `recent_fills`, `recent_journal`,
  `all_transaction_ids`, `global_debit_credit_sum` — none return
  `sqlx` types.

**Rationale:** single-binary deploy, zero ops, fits the `$20/month`
hosting line in [product.md → Cost economics](product.md#cost-economics--monthly-ceiling);
embedded SQLite WAL handles the v0 write rate (≤ a few hundred journal
entries per minute at 1m bars) trivially; backup = copy the file.

**Alternatives considered:**

- `sqlx-ledger` on SQLite — the earlier pick; rejected at build time
  because the crate is **Postgres-only** in its shipped releases. A
  SQLite port would be a multi-week fork job, outside v0 budget.
- `cala-ledger` on Postgres — even more Postgres-locked; forces a DB
  process. Same reason, stronger.
- Leave the substrate open — rejected; the feature work needs a stable
  journal / query surface to build against.

**Pinning + isolation:** the backend (`sqlx::SqlitePool`) is
crate-private; no consumer imports `sqlx` types from `audit`'s public
API. A future swap (to Postgres, to a different embedded store, or to a
hypothetical `sqlx-ledger` with SQLite support) is a one-file change
inside `audit`. Consumers see only `Decimal`, `Money<C>`, `FillView`,
`JournalEntryView`, and the new `StrategyEventView` (v0.5 — see
[Strategy registry & hot-loading → v0.5 strategy-event journal](#v05--strategy-event-journal-schema-q1--confirmed-2026-04-19)
below).

#### v1+ migration path

If the hosted deployment moves off single-box, reconsider Postgres-backed
ledgers (`cala-ledger`, a revived `sqlx-ledger`, or a hand-rolled
Postgres adapter) — provided the migration preserves the public
`audit::query` shape. The Decimal-in / Decimal-out contract is the load-bearing
constraint; the storage engine is not.

### Tick aggregation

- `trade_aggregation` — proven candle aggregator from raw trades. Adopt for the
  tick → OHLCV path in `data`; avoids a class of off-by-one bugs.

### Cost telemetry — dedicated `cost` crate — confirmed 2026-04-17

A standalone `cost` crate owns cost measurement and budget enforcement. In
v0 it ships empty (no LLM calls, no events emitted) but the full surface is
wired so v0.5+ drops calls in without moving code.

Shape:

```rust
pub enum CostEvent {
    Llm {
        provider: LlmProvider,
        model: String,
        tier: LlmTier,           // deep_think | quick_think
        role: AgentRole,         // trader, sentiment_analyst, ...
        tokens_in: u64,
        tokens_out: u64,
        tokens_cached_in: u64,
        usd: Decimal,
        correlation_id: Uuid,
    },
    Infra  { line: InfraLine, usd: Decimal, period: Month },  // v1+
    Data   { feed: FeedId,    usd: Decimal, period: Month },  // v1+
    Storage{ bytes: u64,      usd: Decimal, period: Month },  // v1+
}

pub trait CostSink: Send + Sync {
    fn record(&self, event: CostEvent) -> Result<(), CostError>;
}

pub struct CostBudget { /* ceiling + spent; rollup queries audit ledger */ }
impl CostBudget {
    pub fn remaining(&self) -> Decimal;
    pub fn mode_override(&self) -> Option<LlmTier>;  // auto-degrade @ 80%
}
```

A default `LedgerCostSink` (lives in `cost`, depends on `audit`) writes each
`CostEvent` as a journal entry against `expense:llm:<tier>` and an accrued
`liabilities:llm_accrued` contra. v0 posts zero entries; the accounts exist
in the chart of accounts (R3.2).

**Rationale:** cost will grow — LLM tokens, infra, data feeds, storage, per
[product.md → Cost economics](product.md#cost-economics--monthly-ceiling).
Starting as a standalone crate avoids a later extraction refactor. `cost`
depends on `core` + `audit`; `llm` depends on `cost` (at v0.5).

**Alternatives considered:**

- Keep under `llm` — cheap now, but forces an extraction once non-LLM cost
  lines appear (`infra`, `data`, `storage` already named in the ladder).
- Fold into `audit` — inverts the dependency direction; `audit` is a generic
  double-entry substrate and shouldn't know about OpenAI vs Anthropic.

### Frontend — iced ([iced-rs/iced](https://github.com/iced-rs/iced))

Single UI stack across the project. No mixing with `egui`/`tauri`/`dioxus`.

- `iced` — Elm-architecture (`Model` / `Message` / `update` / `view` /
  `Subscription`); GPU-accelerated via wgpu; multi-window.
- `iced_aw` — community widgets (date pickers, modals, tabs, badges).
- `plotters` with `plotters-iced` backend — equity curves, indicator overlays,
  drawdown plots in the backtest viewer. Architect to spike `plotters` vs
  hand-rolled `iced::widget::Canvas` for the live candlestick view before
  locking in.

#### Why iced fits

- **Subscriptions** wrap our `tokio::sync::mpsc` and `BroadcastStream` feeds —
  the existing actor pattern composes directly. No bespoke glue code.
- **Pure `update` functions** make every state mutation reviewable; matches
  the auditability goal in [product.md](product.md).
- **Multi-window** lets ops cockpit (real-time) and backtest viewer (offline)
  run as separate top-level apps in the same crate, sharing widgets.

#### App layout

| Binary       | Window(s)                       | Data source               |
|--------------|---------------------------------|---------------------------|
| `cockpit`    | Live ops, kill-switch, P&L, log | `agent` over IPC / shared store |
| `viewer`     | Backtest report + equity curve  | `spec/reports/` markdown + artifacts |

Both binaries live in the `ui` crate and depend only on `core` (types) and
`audit` (read-only ledger queries) — never on `strategy`, `exec`, or `models`.
This keeps the UI swappable without touching trading logic.

#### `audit::query` — the read-only surface for `ui` — confirmed 2026-04-17

`ui`'s ledger dependency is limited to a dedicated `audit::query` module that
exposes `Decimal` aggregates and slice iterators. No SQL string, no `sqlx`
type, and no mutable handle leaks into `ui`. Minimum surface for v0 (signatures
indicative):

```rust
pub mod audit::query {
    pub fn cash_balance(asset: Asset) -> Result<Money<Usdt>, QueryError>;
    pub fn position(asset: Asset) -> Result<Position, QueryError>;
    pub fn equity() -> Result<Money<Usdt>, QueryError>;
    pub fn realized_pnl_since(ts: Timestamp) -> Result<Money<Usdt>, QueryError>;
    pub fn unrealized_pnl() -> Result<Money<Usdt>, QueryError>;
    pub fn recent_fills(limit: usize) -> Result<Vec<FillView>, QueryError>;
    pub fn recent_journal(limit: usize) -> Result<Vec<JournalEntryView>, QueryError>;
}
```

`FillView` / `JournalEntryView` are `core`-defined read-side types (no crate
back-edge from `core` to `audit`). The cockpit P&L card calls `equity()` /
`realized_pnl_since()` / `unrealized_pnl()`; the live tape calls
`recent_fills()`. This makes the ledger — not a cockpit accumulator — the
single source of truth for P&L (locks R3.6 in the v0-paper-sma feature brief).

**Rationale:** approved per existing UI constraint, now made explicit and
signed.

**Alternatives considered:** duplicate P&L computation in `ui` for speed —
rejected; violates R3.6 and creates the exact reconciliation gap the
differentiator argues against.

#### Constraints

- Pin a specific iced version per workspace; do not chase releases mid-feature
  (iced API churns between minors).
- All copy lives in a `ui::strings` module so it can be reviewed for clarity
  and later localized.
- Color, spacing, and typography flow from a single `ui::theme` module — no
  ad-hoc styles inside widgets.
- Destructive actions (kill switch, close all positions, cancel all orders)
  require a confirm dialog with a typed-input safety phrase.
- Empty, loading, and error states are first-class for every view — no blank
  screens.

### Data / venues

- `reqwest` + `tokio-tungstenite` for REST and WebSocket feeds (crypto venues).
- `yahoo_finance_api` or `yfinance-rs` for the optional macro overlay
  (DXY, SPX, US10Y) — evaluate `yfinance-rs` first (newer, async, has streaming).
- `clap` for CLI binaries (`backtest`, `agent`).
- `chrono` vs `time` — pick one workspace-wide. RustQuant uses `time`; we
  default to `time` to avoid two date libraries.

#### v0 decision — hand-rolled Binance WS + `MarketDataSource` trait — confirmed 2026-04-17

For v0 we roll our own thin adapter against the Binance spot WebSocket using
`tokio-tungstenite` + `serde` + `reqwest` (for the one-shot symbol metadata /
exchange-info fetch). Two streams only:
`btcusdt@kline_1m` and `btcusdt@trade`. Reconnect with exponential backoff,
heartbeat via ping/pong, optional testnet endpoint. Everything is isolated
behind a `MarketDataSource` trait in `data` so the implementation is
swappable.

```rust
#[async_trait]
pub trait MarketDataSource: Send + Sync {
    /// Symbol metadata + filters fetched once at startup.
    async fn exchange_info(&self, symbol: Symbol) -> Result<SymbolInfo, FeedError>;

    /// Bar stream (kline, venue-closed bars only).
    async fn subscribe_bars(&self, symbol: Symbol, tf: Timeframe)
        -> Result<BoxStream<'static, Result<Bar, FeedError>>, FeedError>;

    /// Raw trade stream (aggregated @trade channel).
    async fn subscribe_trades(&self, symbol: Symbol)
        -> Result<BoxStream<'static, Result<Tick, FeedError>>, FeedError>;
}
```

Implementations in v0: `BinanceFeed`, `ReplayFeed` (drives the same trait off a
Parquet fixture for backtests and UI smoke), `FakeFeed` (in-memory for unit
tests). The `MarketFeed` requirement in v0-paper-sma R1 is the consumer view
of this trait.

**Rationale:** the v0 need is two Binance streams — the surface is small
enough (~200 lines of serde + a reconnect loop) that an external dep trades
away more (version churn, feature creep, audit surface) than it saves. The
crate is cleanly deletable once v0.5 multi-venue drives a real adapter pick.

**Alternatives considered:**

- `binance-rs-async` — Binance-only, async, reasonable quality, but pulls
  REST + margin/futures endpoints we don't use and has historically slow
  release cadence on upstream venue changes. Good fallback if the hand-rolled
  adapter slips week 1.
- `barter-data` (from the `barter` ecosystem) — **strong v0.5 candidate**:
  normalized multi-venue streams (Binance/Coinbase/Kraken), converts into a
  single `MarketEvent` type, streaming-first. Overkill for v0's single venue,
  but revisit in v0.5 when ETHUSDT + Coinbase land per
  [product.md → Universe & data fidelity ladder](product.md#universe--data-fidelity-ladder).
- `ccxt-rs` — CCXT port, broad venue list, but historically thin surface
  and uneven async support. Rejected.
- Full hand-rolled with no trait — rejected; the trait is cheap insurance
  against venue lock-in and is the only thing the `strategy` / backtest
  code sees.

**Deletion criterion for v0.5:** when a second venue (Coinbase) enters the
universe, re-evaluate: pick `barter-data` if it still cleanly maps to
`MarketDataSource`, else write a second hand-rolled adapter.

**Explicitly NOT adopted from lib.rs/finance:**
- FIX engines (`easyfix`, `fixer`, `quickfix`) — crypto venues are REST/WS, not FIX.
- `databento` / `dbn` — institutional equities/futures data, paid, out of scope.
- `sec-fetcher`, equity-only data clients — not crypto.
- Personal finance ledgers (`rustledger`, `tackler`, `aledgr`, `hledger-fmt`).
- `async-stripe`, `mt940`, `ofx-rs` — payments / bank statements, unrelated.
- Options pricing crates (`black_scholes`, `volsurf`, `optionrs`, `implied-vol`,
  `stock-options`) — out of scope until/unless we trade derivatives.
- `simm-rs` — SIMM is for OTC bilateral margin, not us.
- `digifi`, `finquant`, `quantrs`, `alfars`, `finalytics` — unclear maturity;
  revisit if a specific need surfaces.

### LLM
- `anthropic-sdk` (or our own thin client around `reqwest` if the SDK lags)
- `async-openai` for OpenAI-compatible providers (covers OpenRouter, DeepSeek, LM Studio)
- Custom tool-use schema layer in `llm` crate

### Testing
- `proptest` for property tests on strategy invariants
- `criterion` for benchmarks
- `insta` for snapshot tests on prompt + report rendering

## Changelog

- 2026-04-17 (architect): initial scaffold.
- 2026-04-17 (architect): added Foundation libraries section; selected
  RustQuant modules `math` / `stochastics` / `time` / `data` / `iso` /
  `macros` as helpers, explicitly excluded fixed-income modules and the
  basic ML/portfolio/LOB modules; locked default crates for async, numerics,
  data, LLM, and testing.
- 2026-04-17 (architect): surveyed [lib.rs/finance](https://lib.rs/finance);
  added `rust_decimal` as mandatory, defined `Money<C: Currency>` strategy,
  added LOB/matching candidates (`orderbook-rs` / `matchcore` / `rust_ob`),
  TA picks (`kand` + `quantedge-ta`), `openpit` as second-line risk,
  an off-the-shelf double-entry ledger for the new `audit` crate (candidates
  then were `cala-ledger` and `sqlx-ledger`; see later changelog entry for
  the final pick), `trade_aggregation` for tick→bar,
  `yfinance-rs` for macro overlay, `time` chosen over `chrono` workspace-wide;
  flagged crypto exchange clients as a research gap; documented exclusions
  (FIX, equities data, options pricing, payments, personal-finance ledgers).
- 2026-04-17 (architect): selected [iced](https://github.com/iced-rs/iced)
  as the single UI stack; added `ui` crate with two binaries
  (`cockpit` live ops, `viewer` backtest); locked design constraints
  (`ui::strings`, `ui::theme`, confirm dialogs on destructive actions,
  first-class empty/loading/error states); UI depends only on `core` + read-only
  `audit`, never on trading logic.
- 2026-04-17 (architect): added Strategy registry & hot-loading section.
  v0 ships compiled-in `Strategy` trait with plug-in-shaped registry;
  v0.5 adds config-driven `ComposedStrategy` (file-watcher hot-swap);
  v1+ adds WASM plugins via `wasmtime`. Native dynamic libs and embedded
  scripting explicitly rejected. Registry mutations journal to audit ledger.
- 2026-04-17 (architect): resolved the five v0-paper-sma open questions from
  the analyst brief. Q1 matching engine: v0 ships a simple `PaperEngine`
  (bps slippage + taker fee + optional bar-VWAP) behind a `MatchingEngine`
  trait; full LOB deferred to v0.5. Q2 audit backing store: `sqlx-ledger` on
  SQLite for single-binary deploy (cala-ledger requires Postgres in current
  releases); migrate at v1+ if hosting shape changes. Q3 UI → audit: approved
  and made explicit via a new `audit::query` read-only module exposing
  `Decimal` aggregates / slice iterators; ledger is the single source of
  truth for P&L (no cockpit accumulator). Q4 cost location: dedicated `cost`
  crate added to the workspace map with `CostEvent` / `CostSink` /
  `CostBudget` surface; v0 wires the scaffold with zero emitters. Q5 crypto
  exchange client: hand-rolled Binance WS adapter on `tokio-tungstenite`,
  isolated behind a `MarketDataSource` trait with `BinanceFeed` / `ReplayFeed`
  / `FakeFeed` implementations; `barter-data` is the explicit v0.5 fallback
  once multi-venue lands.
- 2026-04-17 (architect): added Naming conventions section. Renamed the
  foundation crate package from `core` to `trading_core` workspace-wide to
  stop shadowing Rust stdlib `::core::` (Rust 2024); crate directory stays
  `crates/core/`. Replaces the per-consumer `trading_core = { package = "core" }`
  alias trap and unblocks `cargo test --workspace --doc`. See Week 1 test
  report [spec/reports/test-2026-04-17-1443-v0-paper-sma-week1.md](reports/test-2026-04-17-1443-v0-paper-sma-week1.md)
  section 7 (R-A).
- 2026-04-17 (architect): formally signed off `sqlx-ledger` on SQLite as the
  v0 audit-ledger substrate (supersedes the earlier candidate language).
  `cala-ledger` deferred to v1+ — reconsider only if hosted deployment moves
  off single-box *and* `cala-ledger` has gained a SQLite backend by then.
  Week 1 T05/T06 integration tests (5/5 green) confirm the pick.
- 2026-04-19 (architect): added **Disaster recovery & backups** section
  reflecting the operator's locked DR decision — local-only snapshots (daily
  `sqlite3 .backup`, weekly Parquet rsync), RPO 24h / RTO ~1h manual, zero
  cloud spend. Off-site sync and WAL streaming explicitly deferred to the
  follow-up project that lands real-money execution.
- 2026-04-17 (developer): repair pass — updated chart-of-accounts count from
  10 to 13 (added `expense:infra`, `expense:data` per cost-telemetry scaffold;
  LLM accounts were already present). `cala-ledger` count in the v0 decision
  prose updated from 10 to 13.
- 2026-04-19 (architect): reconciled Audit & ledger section to code reality
  and resolved the five v0.5 composed-strategies open questions
  ([feature brief](features/v05-composed-strategies.md)).
  **Doc-reality reconciliation:** `sqlx-ledger` v0.11.14 is Postgres-only
  in shipped releases; during v0 Week 1 the developer discovered this and
  substituted raw `sqlx` + `SQLite` + in-repo migrations preserving
  identical double-entry semantics. The Audit & ledger section now documents
  what the code actually does. Substitution is additive: the `audit::query`
  public API stays `Decimal`/core-types-only and is unchanged.
  **Q1 strategy-event journal:** new sibling `strategy_events` table in the
  SQLite ledger; keeps `journal_entries` monetary-only; exposed via
  `audit::query::strategy_events_since` /
  `audit::query::strategy_history`.
  **Q2 registry concurrency:** `parking_lot::RwLock<HashMap<..>>` — zero
  new deps, sub-microsecond uncontended read, fits the 1m bar cadence.
  `arc-swap` reconsidered in v1+ if tick-latency strategies arrive.
  **Q3 exit policy:** symmetric signal-flip in v0.5; per-strategy
  drawdown clamp deferred to v1+ and will live in `risk`, not inside
  each `ComposedStrategy`.
  **Q4 strategies panel layout:** right column above Open positions;
  keeps observation-oriented widgets together and protects the left
  column's destructive-action focus.
  **Q5 new broadcast message types:** `StrategyLoaded` /
  `StrategySwapped` / `StrategyLoadError` live in `trading_core`; three
  new `agent::EventBus` channels (capacity 32 each); lagged-drop + log
  backpressure matches the v0 pattern.
- 2026-04-29 (architect): resolved the six v1-cross-sectional-momentum
  open questions from
  [features/v1-cross-sectional-momentum.md](features/v1-cross-sectional-momentum.md#notes--open-questions-for-architect).
  **Q1 L2 ingest** deferred to v1.5 — keeps v1 shippable; momentum score
  is close-to-close, depth has no consumer. **Q2 funding-rate ingest**
  observation-only at v1: hourly REST poller + new `funding_rates`
  SQLite table + `funding_obs` broadcast channel + new
  `trading_core::FundingObs` type; `MomentumStrategy` does not consume
  it (validates the ingest path for v1.5 without expanding hot-path
  cost). **Q3 long-only** confirmed: `K_long=3`, `K_short=0` for v1;
  loader rejects `k_short > 0` with `unsupported_short_sizing` error
  code; perp-shorting waits for v2. **Q4 multi-venue** deferred to v1.5:
  v1 stays Binance-only; the universe-ladder `+Kraken` entry is re-read
  as a v1-series goal. **Q5 universe filtering** is strategy-side
  (pattern A) — strategies filter `Strategy::on_bar` internally; no
  trait change. Pattern B (registry-side via a new `fn universe()`
  trait method) deferred to v2+ if a tick-latency strategy ever
  stresses the budget. **Q6 `RebalanceRejected` ledger surface** —
  extend the v0.5 `strategy_events` table with a new
  `kind = "rebalance_rejected"` variant; no schema migration; new
  `audit::journal::rebalance_rejected` writer + the existing
  `strategy_history` reader. **v1 architectural deltas:** new
  `crates/strategy/src/cross_sectional/` module (`MomentumStrategy`,
  score, selector); vector-order shape in `risk` (`size_portfolio_target`
  alongside the existing scalar `size_and_validate`);
  `RiskLimits.portfolio_exposure_cap: Option<Decimal>` field added;
  `audit::query::pnl_by_symbol` reader + extended chart of accounts
  (the existing 13-account chart is additive — `assets:position:<asset>`
  is parameterized; v1 universe symbols seed nine new sub-accounts at
  startup, no migration); multi-symbol `ReplayFeed` interleave with
  `(venue_ts ASC, symbol ASC)` deterministic sort; `funding_obs`
  broadcast channel added to `agent::EventBus`. No change to the
  v0 `Strategy` trait shape, no change to the v0.5 audit/broadcast
  surfaces beyond the additive items above.
