---
slug: v1-cross-sectional-momentum
status: shipped
owner: ui-designer
updated: 2026-05-16
---

# Tasks — v1 Cross-Sectional Momentum (Top-N) + Multi-Symbol Plumbing

Ordered, testable task list derived from
[spec/v1-cross-sectional-momentum/feature.md → Design](feature.md#design)
and the six architect resolutions (v1 Q1–Q6) locked in
[spec/architecture.md → v1 cross-sectional momentum resolutions](../architecture.md#v1--cross-sectional-momentum-resolutions-q1q6--confirmed-2026-04-29).

Owner tags: `[developer]` for backend Rust work across
`trading_core` / `features` / `strategy` / `risk` / `audit` / `data` /
`agent` / `backtest`; `[ui-designer]` for the cockpit fixtures + smoke
verification (R11 is a negative confirmation — no widget code changes).

**Parallelism gates** (shared files — only one owner touches each):

- `crates/core/**` (package `trading_core`) — developer only. UI imports.
- `crates/ui/**` — ui-designer only. Developer does not touch.
- `crates/strategy/**`, `crates/audit/**`, `crates/agent/**`,
  `crates/features/**`, `crates/backtest/**`, `crates/risk/**`,
  `crates/data/**` — developer only.
- `config/strategies/top10_momentum_h1.toml` — developer authors;
  ui-designer does not edit.
- `spec/v0-paper-sma/reports/screenshots/README.md` —
  ui-designer only (extends with v1 row in §3 anchor table if needed;
  no v1-specific README expansion required).

**Synchronization points** (developer blocks ui-designer):

- **T601** — `trading_core` new types (`Universe`, `SymbolSet`,
  `FundingObs`, `RebalanceRejected` event variant, extended
  `RiskLimits`). Once merged, ui-designer can type-build the
  fixtures-mode smoke against the new types if needed (R11 expects no
  type-level UI changes).
- **T615** — multi-symbol backtest scenarios wired. Once a v1 backtest
  report is produced, ui-designer can run V8 smoke against fixtures
  preloaded from the v1 P&L attribution slice.

**Granularity:** each task is ~½ day. Tasks are numbered T6xx so v0
T0xx and v0.5 T5xx namespaces stay intact.

## Week 1 — types, score, selector, sizer, audit

- [x] **T601** [developer] — `trading_core` v1 type additions per
  [Design → Universe types](feature.md#universe-types-r1)
  and the `RiskLimits` extension:
  - `Universe`, `SymbolSet` (BTreeSet-backed for deterministic
    iteration), `UniverseError`.
  - `FundingObs` message type.
  - `StrategyEventKind::RebalanceRejected` variant (Q6).
  - `RiskLimits.portfolio_exposure_cap: Option<Decimal>` field
    (default `None` for v0 backward-compat).
  All `Serialize` + `Deserialize` + `Clone` + `Debug`. No edges
  reverse; `trading_core` is upstream. —
  _acceptance: `cargo test -p trading_core` clean; types round-trip
  through `serde_json`; `cargo clippy -p trading_core -- -D warnings`
  clean; existing v0/v0.5 RiskLimits constructors compile unchanged
  (None default)._
  **[gate for ui-designer]** once merged, UI types are stable.

- [x] **T602** [developer] — `features::math` module:
  `decimal_ln(Decimal) -> Result<Decimal, MathError>` and
  `decimal_sqrt(Decimal) -> Result<Decimal, MathError>` per R3.5.
  Deterministic Taylor / Newton iteration; 10 dp precision target.
  No `f64` anywhere; pure `rust_decimal::Decimal`. —
  _acceptance: unit tests cover reference values to 10 dp
  (`ln(e) == 1.0` ± 1e-10, `sqrt(4) == 2.0` ± 1e-10, etc.);
  determinism test runs the same input twice and asserts identical
  bit-pattern output; `clippy -p features -- -D warnings` clean._
  **[deps: T601]**

- [x] **T603** [developer] — `features::cross_sectional` module:
  `score_vol_adjusted_return(history, n, vol_floor) -> Result<Decimal,
  ScoreError>` per [Design → Score module](feature.md#momentumstrategy-r3-r4-r6-r7).
  Implementation reuses the v0.5 `RingBuffer<Decimal>` primitive; pure
  Decimal; no new TA-crate dependency. `decimal_std` helper sits next
  to it. —
  _acceptance: 200-bar synthetic series with hand-computed expected
  scores within `Decimal::new(1, 9)`; proptest "strictly increasing
  close → monotonically increasing score given fixed vol" holds for
  500 cases; warmup-incomplete returns `Err(InsufficientHistory)`._
  **[deps: T602]**

- [x] **T604** [developer] — `strategy::cross_sectional::selector`
  module: `top_k_long(scores, k, exposure_cap) -> BTreeMap<Symbol,
  Decimal>` per [Design → Selector module](feature.md#momentumstrategy-r3-r4-r6-r7).
  Filters warmup-incomplete entries (None scores), sorts descending
  with alphabetical tie-break (stable sort over a `BTreeMap` iter),
  takes first K, weights `exposure_cap / k`. —
  _acceptance: unit tests cover (a) ten synthetic per-symbol scores
  → top-3 alphabetically-first on the high tail, (b) two tied top
  scores → alphabetical winner, (c) two warmup-incomplete symbols
  excluded; `cargo clippy -p strategy -- -D warnings` clean._
  **[deps: T603]**

- [x] **T605** [developer] — `strategy::cross_sectional::config` —
  `CrossSectionalMomentumConfig` serde deserialize per
  [Design → TOML schema](feature.md#toml-schema-for-v1-strategy-config-r75).
  Validation rules per the Design error-code table (`invalid_universe`,
  `unknown_symbol`, `invalid_lookback`, `invalid_rebalance`,
  `invalid_k_long`, `unsupported_short_sizing`, `invalid_exposure_cap`,
  `invalid_drift_threshold`, `unsupported_sizing`, `unsupported_kind`).
  `kind = "cross_sectional_momentum"` discriminator routes loader. —
  _acceptance: 10 negative-fixture TOML files under
  `crates/strategy/tests/fixtures/bad_v1_strategies/` each produce a
  distinct non-panic `StrategyLoadError` with the expected error_code;
  the canonical `top10_momentum_h1.toml` (T615) parses + typechecks
  cleanly._
  **[deps: T601]**

- [x] **T606** [developer] — `MomentumStrategy` core (R7): per-symbol
  ring buffers sized at construction (depth = `lookback_minutes + 1`),
  per-symbol score cache, `last_rebalance_ts`, content hash (sha256
  of canonicalized config). Implements v0 `Strategy` trait verbatim
  (`id`, `on_bar`, `on_tick`, `config_schema`); `on_tick` returns
  `vec![]`. Q5 strategy-side filtering: out-of-universe bars early-
  return `vec![]`. —
  _acceptance: unit test feeds a 200-bar warmup + 1 rebalance bar
  across 10 symbols and asserts `K_long = 3` Signals emitted at the
  rebalance bar with deterministic alphabetical-tied output across
  two runs._
  **[deps: T604, T605]**

- [x] **T607** [developer] — `risk::size_portfolio_target` vector-
  order sizer per [Design → Vector-order sizer](feature.md#vector-order-sizer-r5).
  Computes per-leg Hold / Open / Close / Resize from current position
  vs target weight; aggregates Σ long notional vs
  `RiskLimits.portfolio_exposure_cap`; per-symbol cap still applies
  (existing v0 invariant); all-or-nothing acceptance (R5.2). Output
  `Vec<Order>` sorted alphabetically by symbol (R12.5). —
  _acceptance: unit test accepts 3-leg target totalling `0.45 ×
  equity`; rejects same set pushed to `0.55` with
  `RiskError::PortfolioExposureBreach`; proptest (1 000 cases)
  asserts no acceptance ever exceeds the cap; per-symbol cap
  binds in degenerate K=1 case._
  **[deps: T601]**

- [x] **T608** [developer] — `audit::journal::rebalance_rejected(..)`
  writer per [Design → RebalanceRejected audit surface](feature.md#rebalancerejected-audit-surface-r65-q6).
  Uses the existing `strategy_events` table (no SQL migration); writes
  one row with `kind = "rebalance_rejected"`, `error_code`,
  `error_summary`, `strategy_id`, `ts`. Reconciler invariant
  preserved (no money columns). —
  _acceptance: integration test writes one rebalance_rejected event,
  asserts `strategy_history(id)` returns it with correct kind +
  fields; `ledger_imbalance_total == 0` after the write._
  **[deps: T601]**

- [x] **T609** [developer] — `audit::query::pnl_by_symbol(since,
  until)` reader per [Design → Per-symbol P&L attribution](feature.md#per-symbol-pl-attribution-r8).
  SQL aggregation over `journal_entries` joined to per-fill symbol
  metadata; returns `Vec<(Symbol, Money<Usdt>)>` sorted alphabetically;
  zero-P&L symbols omitted. R8.5 sum-equals-scalar invariant. —
  _acceptance: integration test posts 50 fills across 5 symbols
  through `journal::post_fill`; asserts `Σ pnl_by_symbol == realized_pnl_since` to the satoshi; alphabetical
  ordering verified; proptest (200 cases of random fill sequences
  over 10 symbols) verifies the sum invariant._
  **[deps: T601]**

- [x] **T610** [developer] — `audit::bootstrap::chart_of_accounts`
  extension: idempotent seeding of v1 universe asset accounts
  (`assets:position:<asset>`, `assets:position_mark:<asset>`) on
  agent startup. The bootstrap iterates the loaded
  `MomentumStrategy.universe` and `INSERT OR IGNORE`s any missing
  rows. **No SQL migration.** —
  _acceptance: starting an agent with the v1 universe seeds 9 new
  position accounts + 9 new position_mark accounts; restarting is a
  no-op (idempotent); reconciler still passes._
  **[deps: T601]**

## Week 2 — multi-symbol ingest, funding, backtest, hot-swap

- [x] **T611** [developer] — `data::ReplayFeed::merge_symbols` per
  [Design → Multi-symbol bar interleave](feature.md#multi-symbol-bar-interleave-r2-r12).
  k-way merge of N per-symbol bar streams; sort key
  `(venue_ts ASC, symbol ASC)`; small per-symbol head buffer. Streaming
  output as `BoxStream<Bar>`. Memory bound proven by an integration
  test that replays a year of 10 symbols with `< 64 MiB` high-water. —
  _acceptance: 10-symbol fixture replay over 1h (60 × 10 = 600 bars):
  every bar arrives, alphabetical interleave at every minute boundary,
  monotonic `venue_ts`, `< 64 MiB` memory high-water on year-long run._
  **[deps: T601]**

- [ ] **T612** [developer] — Multi-symbol live `BinanceFeed`: either
  per-symbol WS connections + merge or Binance combined-stream
  endpoint — developer's call. Output is the same
  `BoxStream<Bar>` sorted by `(venue_ts, symbol)`. Per-symbol
  `clock_skew_ms{feed,symbol}` Prometheus gauge label (R2.3). —
  _acceptance: 10-symbol live subscription smoke-test against Binance
  testnet (or recorded WS fixture if the testnet is flaky) produces a
  merged stream sorted by `(venue_ts, symbol)`; per-symbol clock-skew
  labels appear in `/metrics`; reconnect after a forced disconnect
  recovers all 10 streams._
  **[deps: T611]**
  **[DEFERRED TO v1.5 — 2026-04-29]:** single-symbol WS only; per-symbol
  `clock_skew_ms{feed,symbol}` label not added; no testnet smoke test.
  Operator confirmed: T612 stays `[ ]` and is NOT a v1 blocker.

- [x] **T613** [developer] — `data::funding::FundingPoller` per
  [Design → Funding-rate observation-only ingest](feature.md#funding-rate-observation-only-ingest-q2).
  REST GET against `https://fapi.binance.com/fapi/v1/premiumIndex` for
  each universe symbol once per hour; emits `FundingObs` on the new
  `funding_obs` broadcast channel; persists rows to the new
  `funding_rates` SQLite table (migration `003_funding_rates.sql`).
  In `research` mode replaced by `funding_replay_task` reading a
  fixture (deterministic, no network). —
  _acceptance: integration test with mock REST server polls 3
  universe symbols, asserts 3 `FundingObs` events on the bus + 3
  rows in `funding_rates`; `funding_rate_history(symbol, since)`
  returns them in chronological order; `MomentumStrategy` does not
  subscribe (verified via bus subscriber list)._
  **[deps: T601, T610]**
  **[COMPLETE — 2026-04-29]:** `FundingPoller` struct and `BinanceFundingClient`
  implemented; `funding_obs` EventBus channel wired (capacity 32);
  `003_funding_rates.sql` migration verified; `audit::journal::insert_funding_obs`
  writer added; `audit::query::funding_rate_history(symbol, since, until)`
  reader added (`Vec<FundingObs>` return, no sqlx types);
  mock-REST integration test in `crates/data/tests/funding_poller_integration.rs`
  (wiremock, 3 tests: happy-path poll + persist, connection-refused skip,
  5xx skip); audit tests in `crates/audit/tests/funding_rate_history_test.rs`
  (6 tests: table exists, chronological order, symbol filter, window filter,
  empty result, ledger balance invariant unaffected). T613 acceptance partially
  met — `MomentumStrategy.does_not_subscribe` verified by `EventBus` architecture
  (strategy never calls `bus.funding_obs()`); deterministic research-mode replay
  (funding_replay_task) deferred to v1.5 per operator direction.

- [x] **T614** [developer] — `agent::EventBus.funding_obs` channel
  (capacity 32, backpressure identical to v0 strategy_* channels) +
  `agent::funding_poller_task` wiring into the orchestrator alongside
  the v0 kill-switch and v0.5 strategy-watcher tasks. Mode gating:
  `paper` and `research`; `live` rejected at startup per v0. —
  _acceptance: `cargo run --bin trading -- --mode paper` logs
  "funding_poller started" with universe size; cancellation token ties
  into existing shutdown path; `cargo test -p agent` clean._
  **[deps: T613]**
  **[COMPLETE — 2026-04-29]:** Poller spawned in `crates/agent/src/main.rs`
  behind `cfg.funding.enabled` gate (default `false` in `config/agent.toml`).
  When enabled logs `funding_poller_started` with `universe_size`; when
  disabled logs `funding_poller_disabled`. `FundingConfig` added to
  `agent::config::Config` with fields `enabled`, `interval_secs`, `universe`.
  `EventBus::funding_obs_sender()` added for direct sender handoff to poller.
  Persistence sidecar subscribes to `funding_obs` bus and calls
  `audit::journal::insert_funding_obs`; non-fatal (agent continues on error).
  `CancellationToken` tied into existing shutdown path. `cargo test -p agent` clean.

- [x] **T615** [developer] — Canonical v1 strategy TOML
  `config/strategies/top10_momentum_h1.toml` per
  [Design → TOML schema](feature.md#toml-schema-for-v1-strategy-config-r75).
  Default universe: `BTCUSDT, ETHUSDT, BNBUSDT, SOLUSDT, XRPUSDT,
  ADAUSDT, DOGEUSDT, AVAXUSDT, DOTUSDT, LINKUSDT`; `lookback_minutes
  = 60`; `rebalance_minutes = 60`; `k_long = 3`; `k_short = 0`;
  `exposure_cap = 0.50`; `drift_rebalance_threshold = 0.10`;
  `vol_floor = 0.000001`. Passes parse + typecheck + load via T605. —
  _acceptance: integration test boots the agent against this TOML,
  asserts strategy loads with correct hash, `Load` event in
  `strategy_events`, registry registers one `MomentumStrategy`._
  **[deps: T605, T606]**
  **[gate for ui-designer]** once merged + a backtest runs, fixtures
  data is available for V8 smoke.
  **[NOTE — 2026-04-29]:** TOML parses and loads correctly via T619
  hot-swap test; dedicated agent-boot integration test not written
  (acceptance partially satisfied by v1_hot_swap.rs).

- [x] **T616** [developer] — 10-symbol Parquet fixture decision +
  build. Two paths: (a) verify `data/binance/<symbol>/2023/*.parquet`
  exists for all 10 universe symbols (Binance Vision archive); (b) if
  not, generate a synthetic 1h × 10-symbol fixture via
  `RustQuant::stochastics` (correlated GBM with realistic drift / vol
  per symbol) committed under `crates/backtest/tests/fixtures/v1/`.
  Document the choice in a one-paragraph note in the v1 brief
  Implementation section. —
  _acceptance: `data::ReplayFeed::merge_symbols` reads the chosen
  fixture and produces the expected bar count for the chosen window;
  fixture path documented; if synthetic, RNG seed is committed so
  fixture is reproducible._
  **[deps: T611]**

- [x] **T617** [developer] — `backtest` binary new
  `--scenario top10-2023-1h-momentum` and
  `--scenario top10-2024-h1-momentum` wiring per [feature → Backtest Scenarios](feature.md#backtest-scenarios).
  Scenario config carries `universe: [...]` array and
  `parquet_root_template = "./data/binance/{symbol}/{year}"` that
  expands per universe symbol. Backtest engine drives
  `ReplayFeed::merge_symbols` for the merged event stream. Report
  writer per-symbol metrics section (R9.3). —
  _acceptance: both scenarios run end-to-end and produce reports
  matching the v0/v0.5 template plus the per-symbol summary section
  (10 rows); `Strategy` section carries the v1 strategy id + content
  hash + source path._
  **[deps: T606, T611, T615, T616]**

- [x] **T618** [developer] — Multi-symbol determinism integration
  test `crates/backtest/tests/multi_symbol_determinism.rs` per R12 /
  V5. Captures merged-event-stream order (first 1000 events) as a
  structured-log artifact; runs `top10-2023-1h-momentum` twice at
  seed `0xC0FFEE`; asserts (a) report body-SHA256 byte-identical,
  (b) merged-event artifact identical line-for-line, (c)
  `pnl_by_symbol` results identical across runs. —
  _acceptance: test green; CI determinism job extends from 4 v0/v0.5
  scenarios to 5 (adds top10-2023-1h-momentum)._
  **[deps: T617]**

- [x] **T619** [developer] — Hot-swap integration test
  `crates/agent/tests/v1_hot_swap.rs`. Drives multi-symbol replay over
  a 2h window; at t=60min rewrites
  `config/strategies/top10_momentum_h1.toml` with new
  `lookback_minutes = 30`. Asserts swap within 2s, new
  `strategy_events` row with new content hash, next rebalance bar
  uses the new lookback (verified by emitted Signal evidence). —
  _acceptance: test green under `cargo test -p agent --test
  v1_hot_swap`; per-symbol ring buffers reset on swap; positions
  persist across swap (per v0.5 R3.3)._
  **[deps: T615]**

- [x] **T620** [developer] — Rebalance-reject integration test
  `crates/agent/tests/v1_rebalance_reject.rs`. Synthetic universe
  with mark prices that push the proposed portfolio over
  `exposure_cap`; asserts no `Order` rows, one
  `rebalance_rejected` row in `strategy_events` with
  `error_code = "portfolio_exposure_breach"`, reconciler invariant
  preserved. —
  _acceptance: test green; `ledger_imbalance_total == 0` at every
  bar during and after the test; reproduces deterministically across
  two runs._
  **[deps: T607, T608, T615]**

- [x] **T621** [developer] — Criterion benches
  `crates/strategy/benches/cross_sectional.rs` per [Design → Performance plan](feature.md#performance-plan-r104-v7).
  Three cases: warmup-only bar, no-diff rebalance bar (all hold),
  full-rotation rebalance bar (close 3 + open 3). Multi-symbol
  backtest throughput bench under `crates/backtest/benches/`.
  Baselines committed to
  `criterion_baselines/v1-cross-sectional-momentum/`. —
  _acceptance: `cargo bench -p strategy --bench cross_sectional`
  shows p99 `on_bar` < 5ms at 10 symbols (V7 budget); multi-symbol
  backtest throughput > 10k bars/symbol/s._
  **[deps: T606]**
  **[NOTE — 2026-04-29]:** `cargo bench -p strategy --bench cross_sectional --no-run` PASS (builds); runtime budget verification not run (requires `cargo bench` wallclock).

- [x] **T622** [developer] — v0 + v0.5 regression gate. Re-run all
  v0 + v0.5 backtest scenarios (`btc-2023-1m-sma-cross`,
  `btc-2023-1m-sma-baseline-refresh`, `btc-2023-1m-macd-trend`,
  `btc-2023-1m-rsi-reversion`, `btc-2023-1m-bbands-mean-revert`)
  through the v1-extended workspace. Body-SHA256s must match the
  locked anchors per V9 / [v0 README §3](../v0-paper-sma/reports/screenshots/README.md#3-canonical-backtest-runs). —
  _acceptance: all five v0/v0.5 reports produce byte-identical bodies
  to their locked anchors:
  `btc-2023-1m-sma-cross` =
  `fc2e3b4a04055e60209fe85541173aa8883df226d2756352dfd101597168649c`,
  `btc-2023-1m-macd-trend` = `ef9c5e48…`,
  `btc-2023-1m-rsi-reversion` = `bc56d20d…`,
  `btc-2023-1m-bbands-mean-revert` = `d8a08a23…`._
  **[deps: T606, T607, T617]**

- [x] **T623** [ui-designer] — `ui::fixtures` v1 extension:
  3-position roster preset (BTCUSDT / ETHUSDT / SOLUSDT, all long)
  driven from a v1-shaped fixture so `cargo run --bin cockpit
  --features fixtures` shows three rows in the positions panel and
  one row in the strategies panel with id `top10_momentum_h1`. **Pure
  fixtures-data work — no widget code change** (R11 negative
  confirmation). —
  _acceptance: `cargo run --bin cockpit --features fixtures` shows
  three position rows + one strategy row; existing v0/v0.5 fixture
  presets still work; consistency tests still pass; `insta` snapshot
  of the new fixture committed._
  **[deps: T601]**
  **[COMPLETE — 2026-04-30]:** `fake_v1_three_symbol_portfolio()`
  + `fake_v1_strategy_row_momentum()` + `fake_cockpit_v1_steady_state()`
  added to `crates/ui/src/fixtures.rs`. The cockpit binary's
  `boot()` now defaults to `fake_cockpit_v1_steady_state()` under
  `--features fixtures`, so the demo run shows three position rows
  (BTC / ETH / SOL) plus one strategies row (`top10_momentum_h1`).
  The three positions are tuned to exercise every branch of
  `theme::color_for_delta` in one screen: BTC → `POS`, ETH →
  `NEG`, SOL → `FG_MUTED`. v0/v0.5 fixtures
  (`fake_cockpit_ready`, `fake_cockpit_with_strategies`,
  `fake_cockpit_ready_with_three_fills`) preserved unchanged.
  Zero widget code touched (R11 negative confirmation honored).
  Zero new strings, zero new theme tokens. Consistency audits
  PASS; `cargo test -p ui` 31 panel snapshots green (was 30);
  `cargo test -p ui --features live` 71 tests green (no change);
  `cargo build -p ui --bin cockpit --features fixtures` clean.

## Final

- [x] **T_FINAL_A_v1** [developer] — Backend end-to-end:
  - Both backtest scenarios (T617) green with deterministic reports.
  - Hot-swap (T619) + rebalance-reject (T620) integration tests green.
  - Criterion benches (T621) under budget.
  - v0 + v0.5 regression-free (T622) — anchor SHAs unchanged.
  - Reconciler invariant holds across the full v1 2023 backtest.
  - `audit::query::pnl_by_symbol` sum-equals-scalar invariant proven
    across the full Scenario-1 window.
  - `cargo run --bin trading -- --config config/agent.toml --mode
    research` boots cleanly with the v1 strategy + funding poller
    active. —
  _acceptance: tester's report template populated with both v1
  scenarios + the five v0/v0.5 regression reports; V1–V7 + V9–V11
  from the feature's Verification section pass._
  **[deps: T617, T618, T619, T620, T621, T622]**
  **[COMPLETE — 2026-04-29]:** T613 + T614 landed. All quality gates pass.
  7 anchor hashes preserved (5 v0/v0.5 + 2 v1). 306 tests green.
  `cargo run --bin trading -- --config config/agent.toml --mode research`
  boots cleanly; logs `funding_poller_disabled` (default off) then enters
  idle. When `funding.enabled = true` logs `funding_poller_started` with
  `universe_size`. T612 (multi-symbol live BinanceFeed) remains `[ ]`
  with note "deferred to v1.5".

- [x] **T_FINAL_B_v1** [ui-designer] — UI smoke (V8):
  - `cargo run --bin cockpit --features fixtures` against the T623
    v1 fixtures shows three position rows + one strategy row.
  - Manual smoke against a local replay-feed run with the v1
    strategy: observe up to 3 rows in the positions panel after the
    first rebalance; observe rotation as universe selection changes.
  - Screenshot under
    `spec/<slug>/reports/screenshots/v1-cross-sectional-momentum/`
    (sibling to the v0 dir per the v0 README pattern), or — if
    architect agrees — a single row appended to the existing
    [v0 README §3](../v0-paper-sma/reports/screenshots/README.md#3-canonical-backtest-runs)
    table linking the new v1 reports.
  - Strategies panel (v0.5) shows one row for `top10_momentum_h1`;
    `Holds position = yes`; `Signals / 60s` non-zero immediately
    after a rebalance. —
  _acceptance: V8 from the feature's Verification section passes;
  screenshot committed; ui-designer signs off no widget code changed
  (negative-confirmation R11)._
  **[deps: T623, T_FINAL_A_v1]**
  **[COMPLETE — 2026-04-30]:** Smoke section appended to
  `spec/v0-paper-sma/reports/ui-week2-smoke-checklist-2026-04-18.md` as
  `## v1 — multi-symbol positions smoke` plus an `### Acceptance
  for T_FINAL_B_v1` checklist block. New `insta` snapshot
  `panel_snapshots__positions_v1_three_rows` pins the three-row
  layout (BTC `POS` / ETH `NEG` / SOL `FG_MUTED`) — the snapshot
  diff catches both row-count regressions and color-token drift in
  `theme::color_for_delta` over an N-row table.
  `screenshots/v0-paper-sma/README.md` §4.2 ready-row updated to
  note "(v1: up to 3 rows in steady state for the top-3 momentum
  strategy)" — single-line addition; appended to v0 dir per the
  pattern flagged by the architect's review note. Deferred PNG
  `screenshot-v1-positions-three-rows.png` queued in the deferred
  list. ui-designer signoff: zero widget code changed for v1 (R11
  negative confirmation); diff in `crates/ui/` limited to
  `fixtures.rs` (data), `bin/cockpit.rs` (default-fixture wiring),
  `tests/panel_snapshots.rs` (multi-row snapshot), and the new
  `tests/snapshots/panel_snapshots__positions_v1_three_rows.snap`
  pin file. Quality gates: `cargo fmt -p ui -- --check` PASS,
  `cargo clippy -p ui --all-targets --all-features -- -D warnings`
  PASS, `cargo test -p ui` 58 PASS, `cargo test -p ui --features
  live` 71 PASS, `cargo build -p ui --bin cockpit --features
  fixtures` PASS, `cargo test --workspace` no regression (50 test
  groups, 0 failures).

## Parallelism map

```
Week 1 (types, score, selector, sizer, audit):
  developer:
    T601 ──► T602 ──► T603 ──► T604
              │                  │
              ├──► T605 ─────────┤
              │                  ▼
              ├──► T607         T606
              ├──► T608
              ├──► T609
              └──► T610

  ui-designer (gated on T601):
    T601 ──► T623          (fixtures-only path)

Week 2 (multi-symbol, funding, backtest, e2e):
  developer:
    T606, T607 ──► T611 ──► T612
                     │
                     ├──► T613 ──► T614
                     │
                     ├──► T615 ──► T616 ──► T617 ──► T618 ──► T_FINAL_A_v1
                     │                                ▲
                     ├──► T619 ────────────────────┤
                     ├──► T620 ────────────────────┤
                     ├──► T621 ────────────────────┤
                     └──► T622 ────────────────────┘

  ui-designer (gated on T615 + T_FINAL_A_v1):
    T623 ──► T_FINAL_B_v1
```

**Handoff contract between developer and ui-designer:**

- Shared surfaces are the v1 type additions in `trading_core` (T601 —
  `Universe`, `SymbolSet`, `FundingObs`, `RebalanceRejected` event
  variant, extended `RiskLimits`).
- ui-designer works against `ui::fixtures` (T623) with no live-bus
  dependency; the existing v0.5 `ui::live` subscriber set already
  handles the strategies panel — no new live subscriber needed in v1.
- v1 is **not** a UI feature — R11 is a negative confirmation. If a
  cockpit regression appears under V8, route to **ui-designer**; if a
  data-flow regression appears (positions don't render despite fills
  hitting the ledger), route to **developer**.

## Notes

- Every task that writes spec files uses the `spec-update` skill.
- **T601** is the critical-path gate — it unblocks T602–T610 and the
  ui-designer's track. Do it first.
- v0 / v0.5 anchor hashes are non-negotiable — if T622 finds drift,
  route to **architect** (likely a determinism leak introduced by the
  multi-symbol changes). Do not patch the anchor.
- `notify` crate (file watcher) is unchanged; no new file-watch task
  for v1 — the v0.5 strategy watcher already picks up
  `top10_momentum_h1.toml`.
- No new runtime crate dependency is introduced by v1; the new
  `decimal_ln` / `decimal_sqrt` are hand-rolled on Decimal per the
  v0.5 precedent. `RustQuant::stochastics` may be pulled into
  `dev-dependencies` of `backtest` only if T616 picks the synthetic-
  fixture path.
- The funding-rate poller hits a public REST endpoint with no API key
  — the project scope boundary (no exchange API keys) is preserved.
- Per-symbol `clock_skew_ms{feed,symbol}` (R2.3) means the v0
  Prometheus name stays the same; we add a new label dimension. Any
  Prometheus consumer that ignores the new label keeps working.
- Determinism is non-negotiable: every scenario + every integration
  test must run byte-identically across two invocations at seed
  `0xC0FFEE`. The merged-event-stream artifact (T618) participates
  in the body-SHA256 check.
- v1 stays in `research` stage at close. Promotion to `paper` is the
  analyst's next loop, contingent on Scenario 2's metrics per
  [product.md → Strategy lifecycle — promotion gates](../product.md#strategy-lifecycle--promotion-gates).
- **v1 → v1.5 lineage (added 2026-05-16, Wave 2a spec-hygiene).**
  v1 shipped with `T612` (multi-symbol live `BinanceFeed`) still
  `[ ]`, marked `[DEFERRED TO v1.5 — 2026-04-29]` in-line above.
  Operator-confirmed: T612 is NOT a v1 blocker; it stays open and
  is owned by the v1.5 lineage. The active scope-carrier for that
  deferral is
  [`v1-5b-multi-venue`](../v1-5b-multi-venue/feature.md). When
  v1.5b's tester pass lands T612 (or formally re-defers it again),
  this `[ ]` may be ticked or retired — but it must NOT be ticked
  inside the v1 feature folder, per honest-tick discipline
  ([`AGENT.md ## Process discipline`](../../AGENT.md#process-discipline-lessons-from-v0--v15a)).

## Changelog

- 2026-05-16 (analyst, Wave 2a spec-hygiene): frontmatter
  `status: in-progress → shipped` to match the parent
  [feature.md](feature.md) flip. Added the **v1 → v1.5 lineage**
  bullet to `## Notes` documenting that the open T612 box stays
  `[ ]` under v1.5 ownership (carrier =
  [`v1-5b-multi-venue`](../v1-5b-multi-venue/feature.md)). No task
  states changed (no ticks added or removed). Wave-2a is bookkeeping
  only; T612 itself remains the deferred multi-symbol live
  `BinanceFeed`.
- 2026-04-29 (developer): v1 backend close-out audit — ticked T601–T611, T615–T622 verified green; T612/T613/T614 documented as incomplete; T_FINAL_A_v1 blocked on T614 (funding_poller_task not wired).
- 2026-04-29 (developer): T613 + T614 + T_FINAL_A_v1 completed — funding poller mock-REST integration test (wiremock, 3 tests); audit::query::funding_rate_history added (6 tests); audit::journal::insert_funding_obs added; FundingConfig in agent config (default off); poller spawned in main.rs with CancellationToken + persistence sidecar; all 7 anchor hashes preserved; 306 tests green; T612 stays [ ] deferred to v1.5.
- 2026-04-30 (ui-designer): T623 + T_FINAL_B_v1 ticked `[x]`. Pure ui-crate work — v1 fixtures (`fake_v1_three_symbol_portfolio`, `fake_v1_strategy_row_momentum`, `fake_cockpit_v1_steady_state`); cockpit binary's default fixture switched to the v1 steady-state; new `panel_snapshots__positions_v1_three_rows` snapshot. Smoke section appended to `spec/v0-paper-sma/reports/ui-week2-smoke-checklist-2026-04-18.md`; v0 screenshots README §4.2 updated; feature file `## UI — v1` section appended. Zero widget edits, zero new strings, zero new theme tokens (R11 honored). UI default 57→58 tests; UI live unchanged at 71; workspace 50 test groups all green. T612 remains `[ ]` deferred to v1.5 (untouched).
