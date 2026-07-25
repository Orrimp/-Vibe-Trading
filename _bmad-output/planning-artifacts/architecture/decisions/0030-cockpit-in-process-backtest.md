---
adr: 0030
title: Cockpit calls the backtest engine in-process via a tightened library API
status: accepted
date: 2026-05-17
supersedes: none
superseded-by: none
---

# ADR-0030: Cockpit calls the backtest engine in-process via a tightened library API

## Context

The chart-centric Lab introduced by `ui-rethink-phase-a-lab` ships a Run
button from day 1 (operator-decision Q-A2, locked 2026-05-17). The UI
must therefore invoke the backtest engine on a `(strategy, pair, range,
params)` tuple and surface the result in the same process — there is no
out-of-process IPC and there is no `cockpit_live` agent loop in the
read-only Phase A path. Until today, every backtest invocation in this
repository went through the standalone `cargo run -p backtest --bin
backtest -- …` binary, which means there is **no public Rust API** on
the `backtest` crate that a UI thread can call. `crates/backtest/src/lib.rs`
exposes only `report_body_hash` / `extract_report_body` and re-exports
`MatchingEngine` / `PaperEngine`. The whole orchestration (config load,
data load, strategy build, engine step loop, report write) lives in
`crates/backtest/src/main.rs` behind `async fn main()`.

The `agent::runtime::run` surface that `cockpit_live` consumes is
explicitly the **live** dependency edge — the architecture's "agent owns
strategy/exec/models/llm bootstrap" rule lives there (see
[`06-ui-and-cockpit.md` § UI isolation rule](../../../../docs/archive/pre-bmad-spec/architecture/06-ui-and-cockpit.md)).
There is no equivalent edge for backtest. This ADR opens that edge in
the smallest possible shape: a single function on `backtest::engine`
that takes the tuple and returns a `Result<RunReport, RunError>`. The
standalone bin stays the canonical CLI invocation path — it just calls
the same library function internally.

The pattern this ADR establishes is reused by Phase B's full Run-button
wiring (param sheet, cache-miss path), Phase E's Compare matrix (fan-out
N tuples), and eventually v3's continuous paper-loop invocation from
the cockpit during operator-driven what-if exploration. Locking the
shape now — once, with the right async story — avoids three rounds of
churn later.

## Decision

Tighten `crates/backtest/src/lib.rs` to expose:

```rust
pub mod engine {
    pub async fn run_scenario(
        cfg: ScenarioConfig,
    ) -> Result<RunReport, RunError>;
}

pub struct ScenarioConfig {
    pub strategy: StrategyId,        // e.g. "v1.momentum"
    pub pair: (Venue, Symbol),        // e.g. (Binance, XRPUSDT)
    pub range: DateRange,             // start + end UTC
    pub params: Option<ParamSheet>,   // None → strategy defaults
    pub seed: [u8; 32],               // ChaCha20 RNG seed
    pub write_report: bool,           // true → also persists the .md report
}

pub struct RunReport {
    pub equity_series: Vec<(Timestamp, Money<Usdt>)>,
    pub fills: Vec<FillView>,
    pub kpis: BacktestKpis,
    pub report_path: Option<PathBuf>, // Some(...) iff cfg.write_report
}
```

The function takes the configuration as a single struct (not positional
args) to make the API additive over time without breaking call sites.
It returns the full in-memory `RunReport` so the UI can render
immediately **without** re-parsing the Markdown report it just wrote;
the report file is written as a side-effect when
`cfg.write_report = true` (Phase A turns it on so the cached-report
loader at `lab/equity_loader.rs` picks up subsequent Lab opens).

The standalone bin (`crates/backtest/src/main.rs`) is refactored to
build a `ScenarioConfig` from its CLI args and call `engine::run_scenario`,
then print the report path. **CLI behaviour is byte-identical**:
existing `cargo run -p backtest --bin backtest -- --scenario …`
invocations produce the same files in the same locations. The 11
locked body-SHA-256 anchors in `evidence/anchors.toml` stay unchanged
(determinism contract: same seed → same body bytes).

The UI invokes the function on a tokio task spawned via the cockpit's
side-thread runtime handle (the same `tokio::runtime::Handle` already
captured for `KillSwitch::trip`). The iced `update` loop dispatches
`Message::LabRunRequested(...)`; the task posts back via a `oneshot`
+ `iced::Task::perform` glue to `Message::LabRunCompleted(Result<...>)`.
The iced render thread is never blocked. Per-paint reads of cached
reports (M2 equity loader) remain synchronous on the iced thread —
they are file reads under 50 KB and are cached in a per-`Cockpit`
memo.

## Alternatives considered

- **Keep `cargo run -p backtest …` as the only invocation path and
  have the UI shell-out to it.** Rejected: process spawn cost on the
  operator's machine is ~150 ms cold, the JSON-stdout interchange is
  reinvented Markdown parsing, and the cockpit's `tracing` subscriber
  loses the engine's spans. The whole reason we have a Rust UI is to
  keep the data path in-process.
- **Expose `MatchingEngine::step` to the UI directly and have the UI
  drive the loop.** Rejected: violates the UI isolation rule
  (`ui` would need to construct a strategy, which lives in `strategy/`,
  which is on the `ui` deny-list per `06-ui-and-cockpit.md`). The
  orchestration must live in `backtest`.
- **Defer the in-process API to Phase B and ship Phase A with the
  CLI-hint empty state only (Q-A2 original default).** Rejected by
  operator decision 2026-05-17. The chart-as-door framing requires
  that the Lab Run button works on day 1.
- **Run the engine synchronously on the iced thread.** Rejected: a
  v1.momentum × 90d backtest is 800-1200 ms; blocking the iced
  thread during a chip click would freeze the cockpit for the
  entire run.
- **Make the function sync (not `async`) and let callers wrap in
  `tokio::task::spawn_blocking`.** Rejected: `MatchingEngine::step`
  is already `async`, and the data loader path uses `tokio::fs`.
  Keeping the function `async` keeps the engine internals
  composable with the bus / data feed for Phase B.

## Consequences

**Enforced by:**
- `cargo test -p backtest` — existing 11 body-SHA-256 anchor checks
  fail if the standalone bin's report bytes drift. The refactored
  bin **must** produce identical bytes; this is a hard gate.
- `cargo check -p ui --no-default-features` — ensures the `ui` crate
  does not gain a transitive dependency on `strategy/exec/models/llm`.
  The `backtest` crate already encapsulates those edges; `ui`
  depends only on `backtest` (newly added under
  `ui/Cargo.toml [dependencies] backtest = { path = "../backtest" }`),
  not on the deeper crates.
- The UI isolation rule in `06-ui-and-cockpit.md` is **amended** by
  this ADR to allow the `ui → backtest` edge explicitly. No other
  edges relax. The amendment is local to this ADR's section reference;
  the rule body in `06-ui-and-cockpit.md` cites this ADR inline.

**What breaks if violated:**
- A future feature adding `ui → strategy` (e.g. "let the operator
  edit strategy params inline") must route the param mutation through
  `backtest::engine::run_scenario`'s `ParamSheet` — never directly
  poke a `strategy` type from the UI thread.
- A future engine API change that drops or reshapes
  `engine::run_scenario` requires a new ADR; the surface is now part
  of the architectural contract, not an internal helper.

**Determinism contract:**
- `cfg.seed` is mandatory; the function rejects `[0u8; 32]` to make
  "you forgot to set a seed" loud. The Lab's default seed for
  cockpit-initiated runs is `LAB_DEFAULT_SEED` defined in
  `crates/ui/src/lab/defaults.rs` — operator-visible in the run
  metadata strip so the same seed reruns reproduce exactly.

## Changelog
- 2026-05-17 (architect): initial accept. Locks the surface required by
  `ui-rethink-phase-a-lab` M2.5 to enable Lab Run button at Phase A
  per operator-decision Q-A2.
