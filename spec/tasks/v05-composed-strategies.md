---
slug: v05-composed-strategies
status: shipped
owner: developer
updated: 2026-04-19
---

# Tasks — v0.5 Composed Strategies (Hot-Load A) + Multi-Indicator Rules

Ordered, testable task list derived from
[spec/features/v05-composed-strategies.md → Design](../features/v05-composed-strategies.md#design)
and the five architect resolutions (Q1–Q5) locked in
[spec/architecture.md Changelog 2026-04-19](../architecture.md#changelog).

Owner tags: `[developer]` for backend Rust work across
`trading_core` / `features` / `strategy` / `audit` / `agent` / `backtest`;
`[ui-designer]` for the `ui` crate panel, live-subscription extension,
strings / snapshots, and screenshots README update.

**Parallelism gates** (shared files — only one owner touches each):

- `crates/core/**` (package `trading_core`) — developer only. UI imports.
- `crates/ui/**` — ui-designer only. Developer does not touch.
- `crates/strategy/**`, `crates/audit/**`, `crates/agent/**`,
  `crates/features/**`, `crates/backtest/**`, `crates/risk/**` —
  developer only.
- `config/strategies/*.toml` — developer authors the canonical recipes;
  ui-designer does not edit.
- `spec/reports/screenshots/v0-paper-sma/README.md` — ui-designer only
  (extends the v0 reference with a `strategies` panel row).

**Synchronization points** (developer blocks ui-designer):

- **T501** — `trading_core` new message types + `StrategyEventView`.
  Once merged, ui-designer can type-build the panel against fixtures.
- **T512** — `agent::EventBus` three new channels. Once merged,
  ui-designer wires the live subscription.

**Granularity:** each task is ~½ day. Tasks are numbered T5xx so the
v0 T0xx namespace stays intact.

## Week 1 — parser, engine, audit, new types

- [x] **T501** [developer] — `trading_core` new message types +
  read-side views per [Design → New broadcast events](../features/v05-composed-strategies.md#new-broadcast-events-q5-resolution).
  Add `StrategyLoaded`, `StrategySwapped`, `StrategyLoadError`,
  `StrategyEventView`, `StrategyEventKind` (all `Serialize` + `Deserialize`
  + `Clone` + `Debug`). No new edges; `trading_core` is upstream. —
  _acceptance: `cargo test -p trading_core` clean; types round-trip
  through `serde_json`; `cargo clippy -p trading_core -- -D warnings`
  clean._
  **[gate for ui-designer]** once merged, UI can type against the new types.

- [x] **T502** [developer] — `features` crate streaming-indicator
  additions: `Ema`, `Macd` (line + signal + histogram), `Rsi`,
  `Bollinger` (upper + mid + lower). Pure-`Decimal` implementation
  consistent with v0 `features::sma` (no new TA dep — `kand`/`quantedge-ta`
  stay excluded per the T21 note). Each ships a streaming `on_bar`
  interface and a batch equivalent for cross-check in tests. —
  _acceptance: proptest (500 cases each) cross-checks streaming vs batch
  within `Decimal::new(1, 8)`; property "RSI ∈ [0, 100] for all bar
  sequences" holds; `cargo clippy -p features -- -D warnings` clean._
  **[deps: T02 from v0-paper-sma]**

- [x] **T503** [developer] — Rule-DSL lexer + parser in
  `strategy::composed::dsl`. Recursive-descent parser (or `winnow`
  combinators — developer-owned dep choice, no new *runtime* dep) that
  turns a signal string into a `RuleAst`. Covers every production in
  [Design → Rule DSL grammar](../features/v05-composed-strategies.md#rule-dsl-grammar--toml-schema). —
  _acceptance: unit tests parse each of the six R2.3 example rules to
  the expected AST shape; 1 000-case proptest of generated valid rules
  round-trips parse → canonicalize → re-parse with identical AST._
  **[deps: T501]**

- [x] **T504** [developer] — `strategy::composed::typecheck` —
  arity / unknown-indicator / unknown-param / invalid-range /
  invalid-stage / unsupported-sizing detection with distinct
  `StrategyLoadError::error_code` values per [Design → Error codes](../features/v05-composed-strategies.md#rule-dsl-grammar--toml-schema). —
  _acceptance: 10 negative-fixture TOML files under
  `crates/strategy/tests/fixtures/bad_strategies/` each produce a
  distinct non-panic `StrategyLoadError`; error codes match the table._
  **[deps: T503]**

- [x] **T505** [developer] — `strategy::composed::node` — indicator
  node + rule node evaluators per [Design → ComposedStrategy type](../features/v05-composed-strategies.md#composedstrategy-type-r1).
  Ring buffers sized at construction; `on_bar` is allocation-free on
  the hot path (verified by a `#[test]` under `cargo test --features
  heap-track` against a 10_000-bar fixture). —
  _acceptance: unit test replays a 1 000-bar fixture through a
  programmatically-built `ComposedStrategy` (`macd_cross(12,26,9) AND
  rsi(14) < 35`) and asserts signal sequence is byte-identical to a
  hand-coded reference impl (R1 acceptance)._
  **[deps: T501, T502, T503]**

- [x] **T506** [developer] — `strategy::composed::config` —
  `ComposedStrategyConfig` serde deserialize + file loader +
  content-hash (sha256 of canonicalized AST). Filename-stem vs `id`
  mismatch check; single-symbol enforcement in v0.5. —
  _acceptance: unit test loads each of the three canonical recipes
  (from T515, or inline strings until T515 lands) and produces a
  stable, deterministic hash across two runs._
  **[deps: T503, T504]**

- [x] **T507** [developer] — `ComposedStrategy` implements
  `Strategy` trait end-to-end (ties T502–T506 together). Edge-triggered
  signal emission: `false → true` emits Buy, `true → false` emits Sell
  (Q3 symmetric signal-flip). —
  _acceptance: same R1 acceptance test as T505 passes through the full
  `Strategy` trait surface; `Vec<Signal>` output bounded to 0 or 1
  items per bar._
  **[deps: T505, T506]**

- [x] **T508** [developer] — `audit` schema migration
  `migrations/0003_strategy_events.sql` per [Design → Strategy-event audit schema](../features/v05-composed-strategies.md#strategy-event-audit-schema-r4-q1-resolution).
  Backwards-compatible — `sqlx::migrate!` applies it on next boot. —
  _acceptance: integration test opens an empty ledger, runs migrations,
  `sqlite_master` contains `strategy_events` table with the five
  expected indexes/columns._

- [x] **T509** [developer] — `audit::journal::strategy_event(..)`
  writer + `audit::query::{strategy_events_since, strategy_history}`
  readers per [Design → Strategy-event audit schema](../features/v05-composed-strategies.md#strategy-event-audit-schema-r4-q1-resolution).
  No `sqlx` types in the `audit::query` public surface. —
  _acceptance: integration test writes one of each kind (Load / Swap /
  Unload / Reject) and asserts `strategy_history(id)` returns them in
  chronological order with correct hashes + error fields; `sqlx` types
  are crate-private (check via `cargo public-api`)._
  **[deps: T501, T508]**

- [x] **T510** [developer] — Reconciler invariant extension: v0
  minute-boundary reconciler (T26) walks `journal_entries` only and
  ignores `strategy_events`. Add an assertion to the reconciler test
  harness that writing `strategy_events` rows between bars does **not**
  perturb `Σ debits == Σ credits`. —
  _acceptance: existing v0 reconciler tests stay green;
  new test "strategy_events_do_not_affect_balance" passes._
  **[deps: T509]**

- [x] **T511** [developer] — `strategy::registry` refactor: replace
  the v0 compiled-in `HashMap` with
  `parking_lot::RwLock<HashMap<StrategyId, Box<dyn Strategy>>>` per
  [architecture.md — registry concurrency (Q2)](../architecture.md#v05--registry-concurrency-q2--confirmed-2026-04-19).
  Expose `swap(id, new) -> Result<Option<Box<dyn Strategy>>, StrategyError>`
  (returns the previous strategy for hash-computing the Swap event) and
  `unload(id) -> Result<Option<Box<dyn Strategy>>, StrategyError>`.
  Reads via `on_bar` take a read guard; hot path stays sync. —
  _acceptance: existing `sma_crossover` smoke tests pass unchanged;
  new stress test fires 20 swaps in 10 seconds and asserts no torn
  reads (every `on_bar` during the race sees a consistent strategy)._
  **[deps: T507]**

## Week 2 — watcher, UI, backtest, recipes, end-to-end

- [x] **T512** [developer] — `agent::EventBus` three new broadcast
  channels: `strategy_loaded`, `strategy_swapped`, `strategy_error`
  (capacity 32 each) per [Design → New broadcast events](../features/v05-composed-strategies.md#new-broadcast-events-q5-resolution).
  Publisher methods infallible, identical pattern to v0 fills /
  positions / bars. Update
  `spec/reports/dev-week2-broadcast-api-2026-04-18.md` with a
  "v0.5 extension" section appended (new channels + backpressure). —
  _acceptance: `cargo test -p agent` clean; the broadcast doc update
  merged in the same commit as the channel additions._
  **[deps: T501]**
  **[gate for ui-designer]** once merged, UI can subscribe to real events.

- [x] **T513** [developer] — `agent::watcher::run_strategy_watcher`
  task: `notify` watcher on `config/strategies/`, 250ms debounce,
  dispatch to `load_and_swap` / `unload`; writes `strategy_event`
  rows; publishes to the three new bus channels per [Design → File watcher + atomic swap](../features/v05-composed-strategies.md#file-watcher--atomic-swap-r3).
  Parse + typecheck + construct happen **outside** the registry
  write-guard (guard held only for the pointer swap). —
  _acceptance: unit test against a `tempdir` simulates Create / Modify
  / Remove events and asserts the right method on a mock registry is
  called with the right args; 250ms debounce collapses a storm of 10
  write events into 1 load; an invalid TOML during reload does not
  touch the registry (old strategy stays)._
  **[deps: T507, T509, T511, T512]**

- [x] **T514** [developer] — `agent` binary wires
  `run_strategy_watcher` into the top-level orchestrator alongside the
  v0 tasks (halt-file watcher, heartbeat, reconciler, bus). Cancellation
  token ties into existing shutdown path. Mode gating: watcher runs in
  `paper` and `research`; inactive in `live` (which is rejected at
  startup anyway per v0). —
  _acceptance: `cargo run --bin trading -- --config config/agent.toml
  --mode research` logs "strategy_watcher started" at boot; dropping a
  new TOML under `config/strategies/` while the binary is running
  produces a `StrategyLoaded` log line within 2s._
  **[deps: T513]**

- [x] **T515** [developer] — Three canonical recipes committed as
  TOML under `config/strategies/` per R6.1–R6.3:
  `btc_macd_trend.toml`, `btc_rsi_reversion.toml`,
  `btc_bbands_mean_revert.toml`. Each passes parse + typecheck + load. —
  _acceptance: integration test boots the agent against a temp copy of
  the three files, asserts all three load with `stage="research"` and
  each appears in `strategy_history(id)` with a `Load` event._
  **[deps: T507, T513]**

- [x] **T516** [developer] — `backtest` binary `--strategy <id>` flag
  per [Design → Backtest harness alignment](../features/v05-composed-strategies.md#backtest-harness-alignment-r9).
  Resolves compiled-in first, then `config/strategies/<id>.toml`.
  Report writer's new `Strategy` subsection emits id + kind + full
  hash + source path + signal string. —
  _acceptance: `cargo run --bin backtest -- --scenario
  btc-2023-1m-sma-baseline-refresh --strategy sma_crossover --seed
  0xC0FFEE` produces a report whose `Strategy` section matches the
  compiled-in `sma_crossover`; `--strategy btc_macd_trend` produces a
  report whose hash matches the `config/strategies/btc_macd_trend.toml`
  hash._
  **[deps: T507, T515]**

- [x] **T517** [developer] — R7 hot-swap integration test
  `crates/agent/tests/strategy_hot_swap.rs` per R7.1 / R7.2. Drives
  `ReplayFeed` over a 1h fixture; at t=500 bars rewrites
  `btc_macd_trend.toml` with `(8,21,9)` params; asserts swap within
  2s, new signals carry the new hash,
  `strategy_history("btc_macd_trend")` returns exactly `[Load, Swap]`
  with distinct hashes. Determinism: two runs at seed `0xC0FFEE`
  produce byte-identical `strategy_events` tables. Includes the
  "rapid-fire 20 swaps in 10 seconds" sibling test. —
  _acceptance: both tests green under `cargo test -p agent --test
  strategy_hot_swap`._
  **[deps: T513, T515]**

- [x] **T518** [developer] — R8 invalid-config rejection integration
  test `crates/agent/tests/strategy_rejection.rs`. Ten malformed TOML
  fixtures under `tests/fixtures/bad_strategies/` (arity, unknown
  indicator, unknown param, non-UTF8, missing required key, invalid
  stage, undefined param, circular param reference, empty file, empty
  signal). Asserts: no crash; good strategy keeps running; ten
  `Reject` rows in `strategy_events`; original strategy's hash is
  unchanged. —
  _acceptance: all ten fixtures fail-closed; reconciler's
  `ledger_imbalance_total == 0` at every bar during and after the
  test._
  **[deps: T513, T515, T510]**

- [x] **T519** [developer] — Criterion benches
  `crates/strategy/benches/composed_strategies.rs` per R10.2.
  Three cases: 1-rule (`rsi(14) < 30`), 3-rule (`btc_macd_trend`
  shape), 5-rule (R10.2 mixed case). Baselines committed to
  `criterion_baselines/v05-composed-strategies/`. —
  _acceptance: `cargo bench -p strategy --bench composed_strategies`
  shows p99 `on_bar` under the budgets in [Design → Performance budget](../features/v05-composed-strategies.md#performance-budget);
  `cargo bench` with the baseline delta step passes._
  **[deps: T507]**

- [x] **T520** [developer] — Four backtest scenarios execution per
  [feature → Backtest Scenarios](../features/v05-composed-strategies.md#backtest-scenarios):
  1. `btc-2023-1m-sma-baseline-refresh` (must byte-match v0 report
     body sha256 to confirm additive changes didn't drift SMA output);
  2. `btc-2023-1m-macd-trend`;
  3. `btc-2023-1m-rsi-reversion`;
  4. `btc-2023-1m-bbands-mean-revert`.
  All under seed `0xC0FFEE`. Reports land in
  `spec/reports/backtest-<stamp>-<scenario-slug>.md`. —
  _acceptance: all four reports generated; scenario 1's body sha256
  matches the v0 `btc-2023-1m-sma-cross` body sha256
  (`fc2e3b4a04055e60209fe85541173aa8883df226d2756352dfd101597168649c`);
  each report's `Strategy` section carries id + hash + source._
  **[deps: T516, T515]**

- [x] **T521** [developer] — Determinism re-gate: extend the T33-style
  harness (`crates/backtest/tests/determinism.rs`) with the three new
  scenarios from T520. Each new scenario is run twice at seed
  `0xC0FFEE`; both runs produce byte-identical report bodies and
  empty `sqlite3 .dump` diff. —
  _acceptance: CI `determinism` job passes with 4 scenarios instead
  of 2._
  **[deps: T520]**

- [x] **T522** [ui-designer] — `ui::strings` additions per
  [Design → Cockpit strategies panel](../features/v05-composed-strategies.md#cockpit-strategies-panel-r5-q4-resolution).
  All `STRATEGIES_*` keys landed; `ui::strings::all()` still returns a
  stable deduplicated list; `crates/ui/tests/consistency.rs` still
  fails the build if any widget inlines a string. —
  _acceptance: `grep` finds zero new string literals inside
  `crates/ui/src/widgets/`; unit test for `strings::all()` membership
  passes._
  **[deps: T501]**

- [x] **T523** [ui-designer] — `state.rs` extensions:
  `StrategyRow`, `StrategyStatus`, new `Cockpit` fields
  (`strategies: PanelState<Vec<StrategyRow>>`,
  `strategies_signal_counters`), new `Message` variants
  (`StrategyLoaded`, `StrategySwapped`, `StrategyLoadError`,
  `StrategiesRefreshed`, `StrategiesError`, `StrategySignalObserved`).
  `update(..)` arms for each — no `_ =>` catch-all. —
  _acceptance: `cargo test -p ui` clean; new unit tests cover each
  Message variant's state transition (empty / loading / ready /
  error / per-row-error)._
  **[deps: T501, T522]**

- [x] **T524** [ui-designer] — `ui::widgets::strategies` panel
  widget: table with columns per R5.1
  (id / short-hash / status / last-event / signals-60s /
  holds-position), tooltip on hash showing full hash + source path,
  per-row error badge with `error_summary`. Theme reuse only
  (`color::{success, warning, danger, muted}`); no new tokens. —
  _acceptance: `insta` snapshot tests pass for the four panel states
  (loading / empty / ready / error) plus the per-row-error visual;
  widget-consistency test (no inline strings, no inline hex) passes._
  **[deps: T522, T523]**

- [x] **T525** [ui-designer] — `ui::fixtures` additions: deterministic
  `StrategyRow` generators covering Ready / Loading / Error variants so
  the panel can be driven from the cockpit fixtures feature flag
  without a running agent. —
  _acceptance: `cargo run --bin cockpit --features fixtures` shows the
  strategies panel populated with three rows (one Ready, one Loading,
  one Error) without any dependency on `agent`._
  **[deps: T523]**

- [x] **T526** [ui-designer] — `ui::live` extension: three new
  `BusRecipe` subscribers (`strategy_loaded`, `strategy_swapped`,
  `strategy_error`), each mapping to the corresponding `Message`
  variant. `RecvError::Lagged(n)` → log-and-continue;
  `RecvError::Closed` → `StrategiesError(STRATEGIES_CONNECTION_CLOSED)`
  message. —
  _acceptance: `cargo test -p ui --features live --test
  strategies_subscription` drives each channel from a fake `EventBus`
  and asserts the right `Message` variant arrives at the cockpit
  model within 2s; lagged-receiver test does not panic._
  **[deps: T512, T523]**

- [x] **T527** [ui-designer] — Cockpit layout update: `widgets::strategies`
  placed in the right column above Open positions per Q4 decision. v0
  layout for the left column (P&L, latency, kill switch) unchanged.
  Wireframe matches [Design → Cockpit strategies panel](../features/v05-composed-strategies.md#cockpit-strategies-panel-r5-q4-resolution). —
  _acceptance: `insta` snapshot of the full cockpit view (fixtures
  mode) matches the committed golden; consistency test still passes._
  **[deps: T524, T527-prev-snap]**

- [x] **T528** [ui-designer] — Extend
  `spec/reports/screenshots/v0-paper-sma/README.md` with a
  `strategies` panel row in the "Cockpit panel state reference"
  section (sibling to the existing four panels). Document each of the
  four states (loading / empty / error / ready) + the per-row-error
  visual, with string keys + theme token references. —
  _acceptance: README updated in place (v0 reference stays the
  primary doc); ui-designer confirms ownership on the PR._
  **[deps: T524]**

## Final

- [x] **T_FINAL_A** [developer] — Backend end-to-end:
  - All four backtest scenarios (T520) green with deterministic reports.
  - R7 hot-swap (T517) + R8 rejection (T518) integration tests green.
  - Criterion benches (T519) under budget.
  - Reconciler invariant holds across the full run (T510 + T518).
  - `cargo run --bin trading -- --config config/agent.toml --mode
    research` starts cleanly with the three canonical recipes loaded
    and the file watcher active. —
  _acceptance: tester's report template section 5 is populated with
  the four scenarios; V1–V6 + V8–V9 from the feature's Verification
  section pass._
  **[deps: T517, T518, T519, T520, T521]**

- [x] **T_FINAL_B** [ui-designer] — UI smoke extension:
  - Cockpit launches with the strategies panel rendered (fixtures mode
    and live mode).
  - Scripted run drives the panel through empty → loading → ready → error
    → ready-after-recovery; `insta` snapshots pass for each state.
  - Manual smoke: during a local `ReplayFeed` run, edit one of the
    canonical TOMLs and observe the short-hash flip in the panel within
    2s; rewrite with malformed content and observe the per-row error
    state. Screenshots appended to
    `spec/reports/screenshots/v0-paper-sma/` (or sibling dir
    `screenshots/v05-composed-strategies/` if the v0 README outgrows its
    scope — ui-designer's call). —
  _acceptance: V7 from the feature's Verification section passes;
  screenshots + state reference in `spec/reports/screenshots/…/README.md`
  carry the new `strategies` row._
  **[deps: T524, T525, T526, T527, T528, T_FINAL_A]**

## Parallelism map

```
Week 1 (parser, engine, audit, new types):
  developer:
    T501 ──► T502, T503 ──► T504, T505 ──► T506 ──► T507
              │                                       │
              └── T508 ──► T509 ──► T510              │
                                                      ▼
                                                   T511

  ui-designer (gated on T501):
    T501 ──► T522 ──► T523 ──► T525          (fixtures-only path, no live yet)

Week 2 (watcher, UI live, backtest, e2e):
  developer:
    T511, T509 ──► T512 ──► T513 ──► T514 ──► T515
                                                │
                                                ├──► T516 ──► T520 ──► T521 ──► T_FINAL_A
                                                │                        ▲
                                                ├──► T517 ────────────┤
                                                ├──► T518 ────────────┤
                                                └──► T519 ────────────┘

  ui-designer (gated on T512):
    T523 ──► T524 ──► T526 ──► T527 ──► T528 ──► T_FINAL_B
                                          ▲
                                          └── T525 (fixtures)
```

**Handoff contract between developer and ui-designer:**

- Shared surfaces are the three broadcast types + `StrategyEventView`
  in `trading_core` (T501), the `agent::EventBus` channel API (T512),
  and the `audit::query::strategy_history` read surface (T509 —
  indirect, UI consumes it via bus fills + ledger-snapshot refresh at
  `BarClose`, identical pattern to v0 P&L card).
- ui-designer works against `ui::fixtures` (T525) until T512 lands,
  then switches the cockpit's subscription to `ui::live` via the
  `--features live` flag.
- Any change to the three `trading_core` message types during week 2
  is a breaking event — developer posts a note in `spec/reports/` if
  the surface shifts, so ui-designer can re-sync.

## Notes

- Every task that writes spec files uses the `spec-update` skill.
- `T501` is the critical-path gate — it unblocks both the developer's
  audit + agent work and the ui-designer's whole track.
- `risk` crate is explicitly **not** changed in v0.5 per Q3 — leave a
  `// TODO(v1): max_strategy_drawdown_pct` breadcrumb in
  `crates/risk/src/lib.rs` so future development doesn't reinvent the
  question.
- `notify` crate is already a workspace dep from v0 kill-switch work —
  no new dep is added in v0.5 for the file watcher.
- No new runtime crate dependency is introduced by this feature; the
  new TA indicators (EMA, MACD, RSI, Bollinger) are hand-rolled on
  `rust_decimal::Decimal` to match the `features::sma` precedent set
  by v0 T21.
- Determinism is non-negotiable: every scenario + every integration
  test must run byte-identically across two invocations at seed
  `0xC0FFEE`. The `strategy_events` table participates in the DB-diff
  check in T521.
- 2026-04-19 (ui-designer): partial landing — T522, T523, T524, T525,
  T527, T528 ticked as complete. T526 (live subscribers) and T_FINAL_B
  (UI smoke extension) remain open pending developer T512; full
  writeup in [ui-v05-blockers-2026-04-19.md](../reports/ui-v05-blockers-2026-04-19.md).
- 2026-04-19 (ui-designer, resume): T512 landed; T526 now ticked — three
  new `ui::live` subscribers (`strategy_loaded`, `strategy_swapped`,
  `strategy_error`) wired with eager-subscribe and shared
  `CONNECTION_CHANNEL_CLOSED` copy for the closed-channel path; +3 live
  integration tests (67 → 70 total in the `live` feature suite).
  T_FINAL_B still deferred — the four v0.5 backtest reports from
  developer T_FINAL_A are not yet in `spec/reports/`.
- 2026-04-19 (developer, resume): All developer tasks T501–T521 + T_FINAL_A
  complete. Root fix: `Wall-clock time` moved from report body to YAML
  front-matter (`wall_clock_s:`) so body-SHA256 is stable across runs.
  Full workspace: 0 failures. `cargo fmt --all -- --check` clean.
  `cargo clippy --workspace --all-targets -- -D warnings` clean.
  6 determinism tests pass (T33 + 4 × T521). All four backtest reports
  in `spec/reports/`. T_FINAL_B unblocked.
- 2026-04-19 (ui-designer, resume): T_FINAL_B ticked. Smoke checklist
  extended with a `## v0.5 — strategies panel smoke + hot-swap drill`
  section in
  [ui-week2-smoke-checklist-2026-04-18.md](../reports/ui-week2-smoke-checklist-2026-04-18.md):
  four-state fixtures walkthrough referencing
  `screenshots/v0-paper-sma/README.md#45`, R7 hot-swap drill (edit
  `config/strategies/btc_macd_trend.toml`, observe swap within 2s), R8
  invalid-config drill (bad edit flips row to error while other
  strategies keep running), five deferred PNG entries, and a dedicated
  acceptance checklist. The four v0.5 backtest reports from T_FINAL_A
  are cross-linked. Documentation-only task — no `.rs` changes. Quality
  gates: `cargo fmt -p ui -- --check` clean, `cargo clippy -p ui
  --all-targets --all-features -- -D warnings` clean, `cargo test -p
  ui` green (57 tests), `cargo test -p ui --features live` green (70
  tests).
