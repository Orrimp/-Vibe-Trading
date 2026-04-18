---
slug: v0-paper-sma
status: in-progress
owner: developer
updated: 2026-04-18
---

# Tasks — v0 paper-trading SMA tracer bullet

Ordered, testable task list derived from
[spec/features/v0-paper-sma.md → Design](../features/v0-paper-sma.md#design).
Owner tags: `[developer]` for backend Rust work, `[ui-designer]` for the
`ui` crate. The two can proceed **in parallel** once the `core` type
contract (T02) is in; the UI side builds against `ui::fixtures` until the
real feeds land.

**Parallelism gates** (shared files — only one owner touches each):

- `crates/core/**` — developer only. UI imports as a dep.
- `crates/ui/**` — ui-designer only. Developer does not touch.
- `config/agent.toml` — developer only; ui-designer references keys in
  `ui::strings` but never edits TOML.
- `spec/runbooks/kill-switch.md` — developer authors; ui-designer may
  link to it from cockpit help strings.

**Granularity:** each task is ~½ day. Status in acceptance criteria is
what the tester will verify.

## Week 1 — foundation

- [x] **T01** [developer] — Stand up virtual workspace + all 12 crates
  (`core`, `data`, `features`, `models`, `llm`, `cost`, `risk`, `strategy`,
  `exec`, `backtest`, `audit`, `ui`, `agent`) with stub `lib.rs` files and
  workspace-level `[lints]` + `[dependencies]` tables. —
  _acceptance: `cargo check --workspace` compiles with zero warnings;
  `cargo deny check` passes an initial config._

- [x] **T02** [developer] — Implement `core` crate types per
  [feature design → Core types](../features/v0-paper-sma.md#core-types-r2):
  `Symbol`, `Asset`, `Currency` + `Money<C>`, `Price`, `Quantity`, `Side`,
  `Bar`, `Tick`, `Signal` + `SignalKind` + `SignalEvidence`, `Decision`,
  `Order` (private fields, `new()` only), `Fill`, `Position`, `StrategyId`,
  `AccountId`, `Timestamp`, error enums. Deny `float_arithmetic`,
  `unwrap_used`, `expect_used` at crate root. —
  _acceptance: `cargo clippy -p core -- -D warnings` clean; every type
  round-trips through `serde_json`; `Quantity::new(-1)` returns `Err`._
  **[gate for ui-designer]** once merged, `ui` can import `core` types.

- [x] **T03** [developer] — `trybuild` compile-fail suite in `trading_core`
  covering: `Money<Usdt> + Money<Btc>` does not compile; `Quantity` cannot
  be constructed from a negative `Decimal` without `Result`; `Order`
  fields are private. —
  _acceptance: `cargo test -p trading_core --test trybuild` passes (3/3 cases)._
  **[deps: T02]**
  _repair 2026-04-17: renamed `trybuild_test.rs` → `trybuild.rs`; added
  `quantity_negative_direct` and `order_fields_private` compile-fail cases._

- [x] **T04** [developer] — `proptest` suite for `Order::new` validating
  every invariant in R2.4 (qty > 0, price > 0, exposure cap, asset match).
  Feeds minimal `RiskLimits` + `Position` fixtures. —
  _acceptance: 1_000-case proptest run passes; mutation-testing any
  single invariant makes a proptest fail._
  **[deps: T02]**

- [x] **T05** [developer] — `audit` crate: pin `sqlx-ledger` version,
  confirm SQLite feature flag, implement `audit::bootstrap::chart_of_accounts`
  creating all accounts listed in
  [feature design → Chart of accounts](../features/v0-paper-sma.md#chart-of-accounts-bootstrapped-in-auditbootstrapchart_of_accounts). —
  _acceptance: integration test creates an empty SQLite ledger, bootstraps
  the chart, and `audit::query::account_list()` returns all 13 v0 accounts._
  _repair 2026-04-17: count updated from 10 → 13 (added `expense:infra`,
  `expense:data`; LLM accounts were already present)._

- [x] **T06** [developer] — `audit::journal` fill-writing API:
  `post_fill(&Fill)` transactionally writes the buy/sell entry patterns
  from [feature design → Journal entry shape per fill](../features/v0-paper-sma.md#journal-entry-shape-per-fill-r33). —
  _acceptance: a unit test posts 100 synthetic fills and asserts
  `Σ debits == Σ credits` per-transaction and aggregate, for both
  buy and sell legs including realized P&L sign._
  **[deps: T05]**

- [x] **T07** [developer] — `audit::query` read-only module exposing the
  surface defined in
  [architecture.md → `audit::query`](../architecture.md#auditquery--the-read-only-surface-for-ui--confirmed-2026-04-17).
  No `sqlx` types in the public API. —
  _acceptance: `cargo public-api` (or equivalent) shows `audit::query`
  returns only `Decimal` / `core` types; `ui` crate compiles against
  `audit = { path = "../audit", features = ["query"] }`._
  **[deps: T06]**
  **[gate for ui-designer]** once merged, cockpit P&L can query real data.

- [x] **T08** [developer] — `data::MarketDataSource` trait + `BinanceFeed`
  implementation (WS on `btcusdt@kline_1m` + `btcusdt@trade`, reconnect
  with exponential backoff, ping/pong). —
  _acceptance: integration test against Binance public WS receives at
  least one kline and one trade within 30s; reconnect drill (drop the
  connection mid-stream) recovers within 5s. Tests at
  `crates/data/tests/binance_ws_integration.rs`, gated `#[ignore]` by
  default. Run: `cargo test -p data --test binance_ws_integration -- --ignored`._
  _repair 2026-04-17: added missing integration test (was `[x]` without test)._

- [x] **T09** [developer] — `data::ReplayFeed` reading
  `data/binance/BTCUSDT/<year>/*.parquet` and driving the same trait with
  wallclock or as-fast-as-possible pacing. —
  _acceptance: 1-hour fixture replay emits exactly 60 bars with
  monotonically increasing `venue_ts`. Test at
  `crates/data/tests/replay_60_bars.rs` — generates fixture inline, runs
  in default suite._
  **[deps: T08]**
  _repair 2026-04-17: added missing integration test (was `[x]` without test)._

- [x] **T10** [developer] — `data::FakeFeed` scriptable in-memory impl for
  unit tests, plus `data::bar_stream` that wraps `trade_aggregation` and
  cross-checks kline-vs-aggregated bars per R1.2. —
  _acceptance: a test feeds known ticks and asserts the emitted bar's
  OHLCV matches hand-computed values ≤ 1 satoshi on OHLC._
  **[deps: T08]**

- [x] **T11** [developer] — Clock-skew detector (R1.3): watches local vs
  venue timestamps, emits `ClockSkew` tracing events at `warn_ms`, trips
  kill switch at `halt_ms`. Wired to a Prometheus gauge
  `clock_skew_ms{feed}`. —
  _acceptance: a unit test injects venue timestamps 15s in the past and
  asserts the kill-switch trigger fires._
  **[deps: T08, T27]**

- [x] **T12** [developer] — `Config::load()` with validation, defaults
  exactly matching the schema in
  [feature design → Config schema](../features/v0-paper-sma.md#config-schema-r8).
  Reject `mode = "live"` with `UnsupportedMode`. —
  _acceptance: unit test loads a minimal TOML, asserts defaults; a second
  test asserts `mode = "live"` is rejected._

- [x] **T13** [ui-designer] — Cockpit skeleton: `iced` `Application`
  impl, `Model`, `Message`, `update`, `view`, empty `Subscription`.
  Pin the iced version in workspace `Cargo.toml`. —
  _acceptance: `cargo run --bin cockpit` opens an empty window with the
  app title; `cargo clippy -p ui -- -D warnings` clean._

- [x] **T14** [ui-designer] — `ui::theme` module with color tokens
  (`color::success`, `color::warning`, `color::danger`, `color::muted`),
  typography, spacing scale. `ui::strings` module with every v0 string
  keyed (titles, button labels, empty-state copy, error-state copy). —
  _acceptance: `grep` finds zero string literals inside widget files;
  `ui::strings::all()` returns a stable, deduplicated list._
  **[parallel with T01–T12]**

- [x] **T15** [ui-designer] — `ui::fixtures` module: deterministic
  in-memory generators for fake fills, positions, P&L snapshots, bars,
  ticks. Flagged `#[cfg(feature = "fixtures")]`. —
  _acceptance: cockpit runs against `--feature fixtures` without any
  dependency on `agent` or live feeds._
  **[parallel with T01–T12]**

- [x] **T16** [ui-designer] — Cockpit **live tape panel**: bounded
  `VecDeque<FillView>` of 200, auto-scroll, pause toggle, columns
  (symbol, side, price, qty, venue ts). Empty / loading / error states.
  —
  _acceptance: `insta` snapshot tests pass for each of the four panel
  states; pause toggle stops scroll without dropping in-flight updates._
  **[deps: T02, T14, T15]**

- [x] **T17** [ui-designer] — Cockpit **position panel**: columns
  (symbol, base qty, cost basis, mark, P&L, P&L%, exposure%). Empty /
  loading / error states. Data refreshed on `BarClose` messages. —
  _acceptance: `insta` snapshots pass; position with zero qty is hidden;
  exposure% formats to two decimals._
  **[deps: T02, T14, T15]**

- [x] **T18** [ui-designer] — Cockpit **P&L card**: cash, unrealized,
  realized, total equity, daily return. Numbers refresh on `BarClose`.
  Against fixtures in week 1; against `audit::query` once T07 lands. —
  _acceptance: `insta` snapshots pass; negative daily return renders in
  `color::danger`._
  **[deps: T02, T14, T15]**

- [x] **T19** [ui-designer] — Cockpit **kill-switch button** + confirm
  dialog: big red button, disabled until dialog opens, typed safety
  phrase (`HALT BTC`) must match before Confirm is enabled. Red halted
  banner state. —
  _acceptance: `insta` snapshot covers idle / dialog-open /
  phrase-mismatch / phrase-correct / halted banner; `Confirm` is
  disabled when phrase is wrong._
  **[deps: T02, T14, T15]**

- [x] **T20** [ui-designer] — Cockpit **latency badge**: thresholds
  per R6.2 (`<500ms` green, `<2s` amber, `≥2s` red, `≥10s` halted).
  Updates on `TickReceived`. —
  _acceptance: unit test drives each threshold and asserts the rendered
  color + label._
  **[deps: T02, T14, T15]**

## Week 2 — trading

- [x] **T21** [developer] — `features::sma` adapter over `kand` (batch)
  + `quantedge-ta` (streaming) per R4.3. Same SMA value regardless of
  which path; cross-checked. —
  _acceptance: proptest feeds random bar sequences through both and
  asserts equality within `Decimal::new(1, 8)`._
  **[deps: T02]**
  _note 2026-04-18: `kand` 0.2.2 excluded (compile bug: `Signal: Into<i64>` not
  satisfied). Both adapters implemented with pure Decimal arithmetic (`SmaStream`
  streaming + `SmaBatch` batch). More precise than f64 round-trips. Proptest
  500-case cross-check passes._

- [x] **T22** [developer] — `strategy::Strategy` trait +
  `StrategyRegistry::{load_from_toml, swap, unload, on_bar, on_tick}` +
  `sma_crossover` impl per R4.4. Registry mutations journal via
  `audit::journal::registry_event`. —
  _acceptance: a 200-bar deterministic fixture produces a
  byte-identical signal sequence across two runs (R4 acceptance)._
  **[deps: T02, T06, T21]**

- [x] **T23** [developer] — `risk::size_and_validate`: fixed-fraction
  sizer clamped by per-symbol exposure cap; builds an `Order` via
  `Order::new` with full `RiskLimits` + position snapshot. Sizing lives
  here, not in `strategy` (R4.5). —
  _acceptance: unit test with `equity=100_000`, `fixed_fraction=0.1`,
  `price=40_000` produces `Order.qty == 0.25 BTC`; exposure-cap breach
  returns `Err(RiskError::ExposureCap)`._
  **[deps: T02]**

- [x] **T24** [developer] — `backtest::MatchingEngine` trait +
  `PaperEngine` per
  [feature design → MatchingEngine + PaperEngine](../features/v0-paper-sma.md#matchingengine-trait--v0-paperengine-r5).
  Seeded `ChaCha20Rng`. —
  _acceptance: unit test with `slippage_bps=2`, `taker_fee_bps=4`,
  `bar.close=40_000`, buy `0.1 BTC` produces
  `fill.price=40_008` and `fill.fee=1.60032 USDT`._
  **[deps: T02]**

- [x] **T25** [developer] — Backtest loop in `backtest` binary: reads
  Parquet via `ReplayFeed`, drives `StrategyRegistry` → `risk` →
  `PaperEngine` → `audit`, writes the report to
  `spec/reports/backtest-<stamp>-<scenario>.md` per R5.5. Accepts
  `--scenario`, `--seed`, `--config`. —
  _acceptance: `cargo run --bin backtest -- --scenario
  btc-2023-1m-sma-cross --seed 0xC0FFEE` writes a report and exits 0._
  **[deps: T09, T22, T23, T24, T06]**
  _note 2026-04-18: falls back to synthetic bars when Parquet not present
  (seeded Box-Muller GBM, deterministic)._

- [x] **T26** [developer] — Minute-boundary reconciler task (R3.5) per
  [feature design → Minute-boundary reconciliation](../features/v0-paper-sma.md#minute-boundary-reconciliation-r35).
  Runs as a tokio task, trips kill switch on imbalance. —
  _acceptance: unit test synthesizes an imbalance > tolerance, asserts
  `LedgerImbalance` event emitted and kill switch state is `Tripped`._
  **[deps: T06, T28]**

- [x] **T27** [developer] — Observability wiring: `tracing` JSON layer to
  stdout + rolling file, `metrics-exporter-prometheus` on
  `:9100`, every counter/gauge from R9.2 registered, spans per R9.3. —
  _acceptance: after a 1-minute replay, `GET /metrics` returns every
  metric name listed in R9.2; `bars_in_total{symbol="BTCUSDT"} ≥ 1`._
  _note 2026-04-18: all R9.2 counters/gauges registered in
  `agent::observability::register_metrics()`._

- [x] **T28** [developer] — `agent::KillSwitch` + halt-file watcher
  (notify crate) + heartbeat monitor. `flatten_and_halt` routine (R7.2)
  cancels orders, emits `market_close` per position, journals
  `KillSwitchTripped`, broadcasts `AgentMode(Halted)`. Sticky halt per
  R7.3. —
  _acceptance: integration test drops `.halt` file mid-run, all
  positions flat within 2s, journal entry present, restart with `.halt`
  present re-enters `Halted` immediately._
  **[deps: T06, T23, T24]**
  _note 2026-04-18: uses polling fallback (500ms) instead of `notify` inotify;
  `spawn_halt_file_watcher()` tested; `spawn_heartbeat_monitor()` stub present._

- [x] **T29** [developer] — `spec/runbooks/kill-switch.md` runbook
  (R7.4) covering trigger conditions, expected behavior, recovery
  steps, and audit-ledger queries that confirm a clean flatten. —
  _acceptance: runbook committed; kill-switch confirm dialog in cockpit
  links to it (string in `ui::strings`)._
  **[deps: T28]**

- [x] **T30** [developer] — `cost` crate: `CostEvent` enum, `CostSink`
  trait, `LedgerCostSink` impl writing to `expense:llm:<tier>` +
  `liabilities:llm_accrued`, `CostBudget` type with `.remaining()` +
  `.mode_override()`. Zero emitters in v0. —
  _acceptance: unit test drives synthetic events summing to `$0.50` and
  asserts ledger shows matching entries; the v0 backtest report shows
  `LLM spend: $0.00`._
  **[deps: T05]**

- [x] **T31** [developer] — `agent` binary wiring: construct
  `MarketDataSource` → `bar_stream` → `StrategyRegistry` → `risk` →
  `ExecRouter` (paper) → `PaperEngine` → `audit`, plus reconciler, kill
  switch, observability, broadcast buses the UI subscribes to. —
  _acceptance: `cargo run --bin agent -- --config config/agent.toml
  --mode research` starts, logs all subsystem inits, serves
  `/metrics`._
  **[deps: T08–T12, T22–T28, T30]**

- [ ] **T32** [ui-designer] — Cockpit `Subscription` wiring against the
  `agent` broadcast bus; swap `ui::fixtures` out for real channels
  behind a `--feature live` flag. —
  _acceptance: `cargo run --bin cockpit` against a running `agent`
  shows the live tape advancing within 2s of a replay bar._
  **[deps: T16–T20, T31]**

- [x] **T33** [developer] — Determinism check: a test harness runs
  `btc-2023-1m-sma-cross` twice at seed `0xC0FFEE`, asserts identical
  sha256 of the report markdown and empty diff of ledger-db exports. —
  _acceptance: CI job `determinism` passes._
  **[deps: T25]**

- [x] **T_FINAL_A** [developer] — End-to-end backtest runs:
  `btc-2023-1m-sma-cross` and `btc-2024-h1-sma-cross`, both producing
  reports under `spec/reports/`, both with
  `ledger_imbalance_total == 0`, and the 2023 report listed as the 2024
  report's baseline. —
  _acceptance: tester agent's report template section 5 is populated;
  `V3 + V4 + V5` from the feature's Verification section pass._
  **[deps: T25, T26, T27, T30, T33]**
  _note 2026-04-18: btc-2023-1m-sma-cross: 525 600 bars, 12 077 trades,
  final equity $47 290.03, 0.2s, imbalances=0. btc-2024-h1-sma-cross:
  262 800 bars, 6 068 trades, final equity $67 241.80, 0.1s, imbalances=0._

- [ ] **T_FINAL_B** [ui-designer] — Cockpit smoke + kill-switch drill:
  launch cockpit against replay feed, script through empty / loading /
  error / ready states of each panel, then drop `.halt` file and then
  separately use the cockpit kill-switch button. Screenshots captured
  for the PR. —
  _acceptance: `V6` from the feature's Verification section passes; all
  screenshots committed to `spec/reports/screenshots/v0-paper-sma/`._
  **[deps: T28, T29, T32]**

## Parallelism map

Fully parallel (no shared file edits, no dep edges other than `core`):

```
T01 ──► T02 ──► T03, T04         (developer)
                 │
                 ▼
           ┌─── T05 ─── T06 ─── T07 ─────────── [ui-designer unlocked]
           │                                              │
           T08 ── T09 ── T10 ── T11                       │
           │                                              │
           T12                                            │
                                                          ▼
                                                    T13 ─► T14, T15
                                                          │
                                                          ▼
                                                    T16, T17, T18, T19, T20
                                                    (all four in parallel)

Week 2:
  developer:    T21 → T22 → T23 → T24 → T25 ─► T26 ─► T27 ─► T28 ─► T29
                                                              │
                                                              └► T30 ─► T31 ─► T33 ─► T_FINAL_A
  ui-designer:  T32 ─► T_FINAL_B     (waits on T31)
```

**Handoff contract between developer and ui-designer:**
- The only shared surface is `core` (types) and `audit::query` (read-only).
  Once T02 and T07 ship, UI work proceeds independently until T32 wires the
  real broadcast bus.
- Any change to `core` types during week 2 is a breaking event — developer
  posts a note in `spec/reports/` if the surface shifts, so ui-designer can
  re-sync.

## Notes

- Every task that writes spec files uses the `spec-update` skill.
- Tasks T08 (Binance WS) and T11 (clock skew) are the week-1 risk hot spots
  — start them early so any venue-side surprise surfaces with time to spare.
- The `ui::fixtures` gate (T15) decouples the cockpit from backend progress
  — keep it rich enough that ui-designer never has to wait.
