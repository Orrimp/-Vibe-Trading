---
slug: v05-composed-strategies
status: in-progress
owner: architect
updated: 2026-04-19
version: 0.5.0
---

# v0.5 — Composed Strategies (Hot-Load A) + Multi-Indicator Rules

## Why

v0.5's headliner is **config-driven hot-loadable composed strategies** because
that is the one structural thing v0 deliberately did **not** build. v0 shipped
a clean `Strategy` trait with a compiled-in registry that is plug-in-shaped
(see [architecture.md → Strategy registry & hot-loading](../architecture.md#strategy-registry--hot-loading)
and the two-phase plan locked in
[product.md → Open decisions — strategy registry](../product.md#open-decisions));
every other promised surface — data feed, core types, audit ledger, risk engine,
paper-fill matching, cockpit — is real. Research iteration speed is now the
bottleneck: each new strategy today requires editing Rust, recompiling, and
restarting the agent, which is exactly the workflow
[architecture.md → v0.5 — config-driven composition](../architecture.md#v05--config-driven-composition-hot-load-a)
argues covers 70-80% of iteration without leaving Rust. Shipping hot-load
without a second strategy wastes the muscle; shipping multi-indicator rules
without hot-load rebuilds the scaffolding per strategy. The
[product.md → Strategy library — roadmap](../product.md#strategy-library--roadmap)
entry for v0.5 — **multi-indicator rules (MACD + RSI + Bollinger)** — is the
first real exercise of the machinery and lands in the same feature slice for
that reason.

The locked moat bet — **persistent memory + double-entry audit** per
[product.md → Differentiator](../product.md#differentiator) — advances
concretely here. Every registry mutation (load, swap, unload, reject) writes
a `strategy_event` journal entry alongside the existing fill entries, so the
audit ledger answers "which strategy code was active when this trade fired?"
for any trade in history. That is the foundation the v1+ reflection loop will
query to correlate strategy changes with P&L. The cockpit's P&L card already
pulls from the ledger (see
[architecture.md → `audit::query`](../architecture.md#auditquery--the-read-only-surface-for-ui--confirmed-2026-04-17));
per-strategy attribution in the v0.5+
[operator success reports](../product.md#operator-success-reports)
is a read-side slice of the same ledger once multiple strategies run
concurrently. Hot-swap is therefore not just a convenience — it is the first
moment in the project's life where the ledger's `strategy_event` stream
carries information a single-strategy deployment cannot.

v0.5 is **not** validating edge, model quality, LLM behavior, or multi-symbol
portfolio construction. No Sharpe claim is made for any of the three canonical
recipes; the hypotheses below explicitly expect weak risk-adjusted returns.
LLM cost for this feature is **$0.00** — all composition is deterministic Rust
evaluating parsed TOML rules, consistent with
[product.md → Cost economics](../product.md#cost-economics--monthly-ceiling)
v0.5 staying inside the $45/month v0 ceiling. The analyst/debate/trader
pipeline from [product.md → Trading-time agent roster](../product.md#trading-time-agent-roster)
is still deferred to the v0.5+ LLM tiers that follow this feature. v0.5 keeps
the `paper` and `research` modes active; `live` remains rejected at startup
per v0 R8.2 — real-money execution is a follow-up project per
[product.md → Project scope boundary](../product.md#project-scope-boundary).

## Requirements

Numbered, testable, derived from
[product.md → Strategy library — roadmap](../product.md#strategy-library--roadmap)
and [architecture.md → Strategy registry & hot-loading](../architecture.md#strategy-registry--hot-loading).
Each ends with a one-line **acceptance** the tester can verify. All
requirements preserve the v0 `Strategy` trait shape — no trait changes.

### R1 — Composed strategy engine

- **R1.1** `strategy` crate adds a new `ComposedStrategy` type that
  **implements** the existing `Strategy` trait (R4.1 of the v0 brief) —
  `id` / `on_bar` / `on_tick` / `config_schema`. No trait changes. The
  registry continues to hold `Box<dyn Strategy>`.
- **R1.2** The body of a `ComposedStrategy` is a tree of typed nodes
  evaluated bar-by-bar:
  - **Indicator nodes** — SMA, EMA, MACD (line + signal + histogram),
    RSI, Bollinger Bands (upper / mid / lower). All sourced from the
    `features` crate via the `kand` (batch) and `quantedge-ta`
    (streaming) adapters already built for v0
    ([architecture.md → Technical analysis](../architecture.md#technical-analysis)).
    `strategy` does **not** gain a direct dependency on either TA
    crate; adapters stay in `features`.
  - **Value nodes** — bar-native references: `close`, `open`,
    `high`, `low`, `volume`, `trade_count`; rolling reducers:
    `min(window)`, `max(window)`, `avg(window)`.
  - **Rule nodes** — logical `AND` / `OR` / `NOT`; comparisons `<`,
    `<=`, `==`, `>=`, `>`; crossovers `cross_above` / `cross_below`;
    threshold predicates (`macd_hist > 0`); touch predicates
    (`bollinger_lower_touch(20,2)` = `close <= bb_lower(20,2)`).
- **R1.3** Evaluation is deterministic, allocation-free on the hot path
  (nodes hold pre-sized ring buffers sized to the deepest lookback at
  construction), and produces a single `Signal` per bar per strategy
  when the rule transitions from false → true (buy) or true → false
  (sell) — matching the v0 edge-triggered semantics.
- **R1.4** Node tree construction is a one-time parse at load time;
  the hot path (`on_bar`) performs no parsing, no allocations, no
  string ops. [ASSUMPTION] the v0 perf budget (R10) applies unchanged.
- **Acceptance:** a unit test builds a `ComposedStrategy` programmatically
  (no TOML) with `macd_cross(12,26,9) AND rsi(14) < 35`, replays a
  1000-bar fixture, and asserts the produced `Signal` sequence is
  byte-identical to a hand-coded reference implementation of the same
  rule.

### R2 — Rule DSL (TOML)

- **R2.1** Strategies live as individual TOML files under
  `config/strategies/<strategy_id>.toml`. One strategy per file — the
  file watcher and journal key off the filename as the canonical
  `StrategyId`.
- **R2.2** File schema (human-readable, schema-validated at load):
  ```toml
  # config/strategies/<id>.toml
  id      = "btc_macd_trend"          # must match filename stem
  kind    = "composed"                # reserved for future `kind` values
  symbol  = "BTCUSDT"                 # single symbol in v0.5
  stage   = "research"                # research | paper (see R4.4)
  signal  = "macd_hist(12,26,9) > 0 AND close > ema(200)"
  size    = "fixed_fraction(0.1)"     # references risk::size_and_validate
  params  = { rsi_floor = 35, vol_multiple = 1.5 }  # optional named scalars
  ```
- **R2.3** The `signal` grammar supports at minimum (covered by unit
  tests):
  - Single crossover: `macd_cross(12,26,9)` — MACD line crosses signal line upward.
  - AND combination: `macd_cross(12,26,9) AND rsi(14) < 35`.
  - OR with threshold: `bollinger_lower_touch(20,2) OR rsi(14) < 20`.
  - Parentheses for grouping: `(a OR b) AND NOT c`.
  - Numeric literals and the parameter references declared in `params`.
- **R2.4** The `size` field is a reference to a v0 `risk` sizing primitive.
  v0.5 accepts `fixed_fraction(<f>)` (reuses `risk::size_and_validate` from
  v0 R4.5) and rejects anything else with a clear `UnsupportedSizing`
  error. The sizing call continues to live in `risk`, not `strategy` —
  the composed strategy emits a `Signal` (side only) and the existing
  risk path computes quantity.
- **R2.5** Schema is validated via `serde` + a `ComposedStrategyConfig`
  struct with `#[serde(deny_unknown_fields)]`. Parse errors and
  semantic errors (unknown indicator, arity mismatch, undefined
  parameter, invalid stage value) produce a `StrategyLoadError` with
  file path, line/col when available, and a short human message.
- **Acceptance:** a property test feeds valid TOML permutations of the
  four example rules in R2.3 and asserts round-trip parse → evaluate
  against a fixture matches a reference table of expected signals;
  a negative test feeds ten deliberately malformed TOML files
  (missing args, unknown indicator, unknown param, non-ASCII
  operator, etc.) and asserts each produces a distinct, non-panic
  `StrategyLoadError`.

### R3 — File watcher + atomic hot-swap

- **R3.1** `agent` crate starts a `notify` file watcher on
  `config/strategies/` (`notify` is already a v0 dependency for the
  kill-switch `.halt` file watcher — see v0 R7.1). Debounce 250ms to
  collapse editor write storms.
- **R3.2** On any `create`, `modify`, or `rename-in` event:
  1. Read the TOML file to memory.
  2. Parse + validate per R2.5. On failure: keep the old strategy (if
     any) running, emit a rejection event (R4) and a broadcast-bus
     `StrategyLoadError` event for the cockpit (R5). Do **not** crash
     the agent.
  3. On success: construct a fresh `Box<dyn Strategy>` (a
     `ComposedStrategy`), compute a content hash of the canonicalized
     parsed AST, and call `StrategyRegistry::swap(id, new)` (v0 R4.2
     already exposes this method).
- **R3.3** On `remove` / `rename-out`: call
  `StrategyRegistry::unload(id)`. A subsequent bar that would have
  reached that strategy sees no signal; any open position held by
  that strategy stays open (position management is not a strategy
  concern — the book persists across strategy swaps).
- **R3.4** The swap is **atomic from the runtime's view**: the next
  `on_bar` fan-out sees either the old strategy or the new one, never
  a partially-constructed state. Implementation detail —
  `StrategyRegistry` is held behind a single `parking_lot::RwLock`
  and `swap` takes a write guard; hot path reads use a read guard.
  [ASSUMPTION] `RwLock` is acceptable vs a lock-free swap because
  `on_bar` is called once per bar per symbol at 1m cadence — read
  contention is negligible compared to the TA math inside nodes;
  architect to confirm or push back.
- **R3.5** End-to-end latency from file-save to next-bar using the new
  rule: `<` 2s at 1m bar cadence (250ms debounce + parse + swap + the
  next bar boundary). Measured in R7 integration test.
- **Acceptance:** an integration test under the `ReplayFeed` driver
  writes a new TOML, asserts a `swap` journal entry appears within
  2s, and the next emitted `Signal` carries the new `StrategyId`
  hash; rewriting with invalid TOML leaves the old strategy's hash
  unchanged in the registry.

### R4 — Audit integration

- **R4.1** Extend `audit::journal` (which already writes fill entries
  per v0 R3.3 and the R3.4 scaffold) with a `strategy_event` entry
  kind. Every registry mutation emits one such entry:
  - `Load { id, hash, source_path }`
  - `Swap { id, old_hash, new_hash, source_path }`
  - `Unload { id, hash, source_path }`
  - `Reject { source_path, error_code, error_summary }`
- **R4.2** Entry fields (all in one ledger row — no multi-row writes):
  - `ts` (venue-free — this is an operator event, stamped with local
    monotonic + wallclock).
  - `kind` = one of the four above.
  - `strategy_id` (nullable for `Reject` when the id is malformed).
  - `old_hash` / `new_hash` — sha256 of the canonicalized parsed AST,
    32-byte hex; lets future reports correlate trades with the exact
    rule config that produced them.
  - `source_path` — repo-relative path under `config/strategies/`.
  - `operator` = `"system"` (v0.5 is always system — the single
    operator triggers via file edit; the `"user"` value is reserved
    for a future cockpit "edit strategy" flow).
  - `error_code` / `error_summary` — present only for `Reject`.
- **R4.3** `strategy_event` entries are **informational** — they post
  no debits or credits. The v0 R3.5 reconciliation invariant
  (`cash + Σ(positions × mark) = equity`) is unaffected and the
  reconciler explicitly skips `strategy_event` rows. Extend the
  v0 reconciler to assert that skipping is correct (no silent
  accidental balance effect).
- **R4.4** `audit::query` gains:
  ```rust
  pub fn strategy_events_since(ts: Timestamp) -> Result<Vec<StrategyEventView>, QueryError>;
  pub fn strategy_history(id: StrategyId) -> Result<Vec<StrategyEventView>, QueryError>;
  ```
  `StrategyEventView` is a `trading_core`-defined read-side type,
  same pattern as `FillView` / `JournalEntryView` (v0 `audit::query`
  contract).
- **R4.5** [ASSUMPTION] The `sqlx-ledger` schema supports
  metadata-only journal entries via an "info" transaction type (no
  account legs). If not, add a sibling `strategy_events` table in
  the same SQLite database, written inside the same transaction as
  a bracketing zero-sum info posting — the audit invariant (one
  writer, atomic on crash) must hold. Architect to confirm the
  shape.
- **Acceptance:** an integration test performs load → swap → bad-swap
  (rejected) → unload, then calls `strategy_history(id)` and asserts
  the returned `Vec<StrategyEventView>` has exactly 4 entries in
  order with the expected `kind` + hashes; the reconciliation
  invariant (v0 R3.5) still passes at every bar.

### R5 — Cockpit visibility (strategies panel)

- **R5.1** Add a new **strategies panel** to the cockpit
  ([architecture.md → Frontend — iced](../architecture.md#frontend--iced))
  owned by ui-designer. Columns:
  - **Strategy ID** (the filename stem).
  - **Status** — Ready / Loading / Error (from the broadcast-bus
    events in R3.2).
  - **Source hash** — short 7-char prefix of the content hash, with
    tooltip showing the full hash + source path.
  - **Last event** — last swap/load/reject timestamp + type.
  - **Signals (last 60s)** — running count of `Signal`s emitted in
    the last minute.
  - **Position status** — whether the strategy currently holds a
    position (derived from fills in `audit::query`).
- **R5.2** First-class empty / loading / error / ready states per
  the v0 UI contract (v0 R6.4). Empty = "No strategies loaded — add
  a TOML file under `config/strategies/`." Error = per-strategy
  red row with the `error_summary` from the `Reject` event.
- **R5.3** The panel subscribes to the broadcast bus for
  `StrategyLoadError` / `StrategyLoaded` / `StrategySwapped` events
  (new message types in `trading_core`); signal counts come from
  an in-memory 60s ring buffer fed by the agent's fan-out output
  (same path that already counts fills).
- **R5.4** All copy lives in `ui::strings`; colors in `ui::theme` —
  v0 R6.3 invariant unchanged.
- **R5.5** [ASSUMPTION] the strategies panel lives alongside the
  existing four v0 panels (live tape, position, P&L, kill switch);
  position and layout are architect + ui-designer to decide — the
  analyst position is that the panel belongs in the left column
  near the kill switch because both are control-surface views, not
  market-surface views, but this is a wireframe call.
- **Acceptance:** a UI smoke test drives the panel through each state
  (empty → loading → ready → error → ready-after-recovery) and
  snapshots pass via `insta` per the v0 R6 pattern.

### R6 — Multi-indicator rules — three canonical recipes

First real use of the composition machinery. Each recipe ships as a
committed TOML file under `config/strategies/`; each is the subject of
one backtest scenario below.

- **R6.1** `btc_macd_trend` — trend-following, MACD-positive confirmation
  above a long EMA.
  ```toml
  id     = "btc_macd_trend"
  kind   = "composed"
  symbol = "BTCUSDT"
  stage  = "research"
  signal = "macd_hist(12,26,9) > 0 AND close > ema(200)"
  size   = "fixed_fraction(0.1)"
  ```
- **R6.2** `btc_rsi_reversion` — oversold mean-reversion with a local
  support floor (reject the falling-knife).
  ```toml
  id     = "btc_rsi_reversion"
  kind   = "composed"
  symbol = "BTCUSDT"
  stage  = "research"
  signal = "rsi(14) < 30 AND close > min(low, 20)"
  size   = "fixed_fraction(0.1)"
  ```
- **R6.3** `btc_bbands_mean_revert` — Bollinger-lower touch gated by a
  volume confirmation (tradable only when real flow is present).
  ```toml
  id     = "btc_bbands_mean_revert"
  kind   = "composed"
  symbol = "BTCUSDT"
  stage  = "research"
  signal = "close < bollinger_lower(20,2) AND volume > 1.5 * avg(volume, 20)"
  size   = "fixed_fraction(0.1)"
  ```
- **R6.4** All three stay in `stage = "research"` — per
  [product.md → Strategy lifecycle — promotion gates](../product.md#strategy-lifecycle--promotion-gates)
  promotion to `paper` requires Sharpe > 1.0 on 2y OOS and a clean
  tester report. v0.5 explicitly does not promote; lifting through
  the gate is the analyst's next loop once backtest metrics are in
  hand.
- **R6.5** Exit rule for all three recipes in v0.5 is **symmetric
  signal flip** — when the rule transitions true → false, emit
  `Sell` to close. Stop-loss / time-based exits are
  [ASSUMPTION] deferred to v0.5+ follow-up unless analyst loop finds
  them dominant in backtest; architect may push back if symmetric
  exit is insufficient to avoid runaway losses against the
  `max_drawdown_stop_pct` floor from v0 R8.2.
- **Acceptance:** each of the three TOML files parses clean, loads
  on agent startup (integration test), and produces at least one
  non-zero signal during a 1-day replay on the v0 fixture.

### R7 — Hot-swap roundtrip test

- **R7.1** Integration test: start the agent in research mode against
  `ReplayFeed` over a 1-hour Parquet fixture.
  1. At t=0 copy `config/strategies/btc_macd_trend.toml` with
     `(12,26,9)` into a temp strategies dir.
  2. Observe signals flowing with the initial hash.
  3. At t=500 bars into the replay, rewrite the file with
     `(8,21,9)` parameters.
  4. Within `≤ 2s` (R3.5), observe new signals with the new hash.
  5. Query `audit::query::strategy_history("btc_macd_trend")` —
     assert exactly 2 entries: `Load` (initial) and `Swap`
     (parameter change), with distinct hashes.
- **R7.2** The test runs deterministically under the v0 seed
  `0xC0FFEE`; two runs produce byte-identical
  `strategy_events` tables (the swap timestamp comes from the
  replay's synthetic clock, not wall time, so the determinism
  property from v0 R5.4 extends cleanly).
- **Acceptance:** the test above is green in CI; a sibling
  "rapid-fire" test performs 20 swaps in 10 seconds and asserts
  no registry corruption (every `on_bar` either sees the latest
  or a previous version, never a torn state).

### R8 — Invalid-config rejection test

- **R8.1** Integration test: with `btc_macd_trend.toml` loaded and
  running clean, write a malformed file under the same directory
  (`signal = "macd_cross(12)"` — missing args):
  1. Watcher fires, parse fails, `StrategyLoadError` event posts to
     the broadcast bus.
  2. `strategy_history(...)` for the target id shows a `Reject`
     entry with the error summary.
  3. The original strategy's hash in the registry is **unchanged**
     — signals keep flowing with the pre-failure rule.
  4. Cockpit strategies panel (R5) displays the per-strategy error
     state per the UI-smoke suite.
- **R8.2** Covers ten malformed cases (arity mismatch, unknown
  indicator, unknown parameter reference, non-UTF8 bytes, missing
  required top-level key, invalid stage value, reference to an
  undefined param, circular parameter reference, empty file, empty
  signal string) — one per file in `tests/fixtures/bad_strategies/`.
- **Acceptance:** all ten fixtures reject without crashing the agent;
  the good strategy keeps running; the ledger shows exactly ten
  `Reject` entries plus the original `Load` entry.

### R9 — Backtest harness alignment

- **R9.1** The v0 `backtest` binary is already generic over
  `Box<dyn Strategy>`; confirm by adding a `--strategy <id>` CLI flag
  that selects a strategy from `config/strategies/<id>.toml` (no file
  watcher inside the backtest — the config is resolved once at run
  start).
- **R9.2** Backward compatibility: running the v0 scenarios
  (`btc-2023-1m-sma-cross`, `btc-2024-h1-sma-cross`) continues to
  work unchanged. `sma_crossover` stays as a compiled-in strategy;
  nothing about v0's SMA path is rewritten as a `ComposedStrategy`.
  A `--strategy sma_crossover` invocation selects the compiled-in
  one; `--strategy btc_macd_trend` selects the composed one.
- **R9.3** Report writer (v0 R5.5) gains a `Strategy` section
  emitting the strategy id + content hash + source path; the rest
  of the report template is unchanged.
- **R9.4** Determinism invariant (v0 R5.4) holds for composed
  strategies: at seed `0xC0FFEE` against the same fixture, two
  runs of the same composed TOML produce byte-identical reports
  (sha256 match of the report body).
- **Acceptance:** the v0 2023 backtest baseline re-runs to a
  byte-identical report under the same seed (Scenario 1 below);
  each of the three composed-strategy TOMLs runs end-to-end under
  the same CLI against the 2023 fixture (Scenarios 2–4).

### R10 — Performance budget

- **R10.1** The v0 performance budget in
  [architecture.md → Performance budget](../architecture.md#performance-budget)
  applies unchanged: bar-close → signal `< 5ms p99`, backtest
  throughput `> 100k bars/s` on a single thread per symbol.
  Composed strategies of typical v0.5 complexity (≤ 5 indicator
  nodes, ≤ 3 rule nodes) must fit inside that budget.
- **R10.2** Criterion benches live under
  `crates/strategy/benches/composed_strategies.rs` covering:
  - Single-rule: `rsi(14) < 30`.
  - 3-rule AND: `btc_macd_trend` shape (MACD-hist > 0 AND
    close > EMA(200)).
  - 5-rule mixed: `(rsi(14) < 30 OR macd_cross(12,26,9)) AND
    close < bollinger_lower(20,2) AND volume > 1.5 * avg(volume,20)
    AND NOT (close < min(low,20))`.
- **R10.3** Bench baseline is established on first run and
  committed to `criterion_baselines/`; regressions `> 10%` on the
  hot path fail the bench step.
- **Acceptance:** `cargo bench -p strategy` shows p99
  `on_bar` latency `< 5ms` for all three bench cases; backtest
  throughput against 2023 1m BTCUSDT stays `> 100k bars/s` on a
  single thread.

## Backtest Scenarios

Four scenarios — a baseline re-run (comparability anchor) plus one per
new strategy recipe. All four share a single universe / period / fee /
sizing profile so that inter-recipe comparisons are apples-to-apples,
and differ only in the strategy config.

### Scenario: `btc-2023-1m-sma-baseline-refresh`

- **Universe:** `BTCUSDT`
- **Period:** `2023-01-01` → `2023-12-31`
- **Granularity:** `1m`
- **Data source:** `binance-spot` (via `data/binance/BTCUSDT/2023/*.parquet`)
- **Fees:** `0.04%` taker, `0.02%` maker (maker unused — market orders only)
- **Slippage model:** `bps: 2`
- **Initial capital:** `100_000 USDT`
- **Position sizing:** `fixed-fraction 0.1`
- **Risk limits:**
  - Max leverage: `1x` (spot, no margin)
  - Max drawdown stop: `-15%`
  - Per-symbol exposure cap: `40%`
- **Strategy params:** v0 compiled-in `sma_crossover` with
  `fast_len = 20`, `slow_len = 50` (identical to v0
  `btc-2023-1m-sma-cross`).
- **Seed:** `0xC0FFEE`
- **Baseline report:** v0's
  `spec/v0-paper-sma/reports/backtest-<stamp>-btc-2023-1m-sma-cross.md`.

**Expected outcome (analyst hypothesis):** Byte-identical metrics to
the v0 2023 baseline within the determinism invariant (R9.4). Any
divergence is a regression — the composed-strategies work must not
alter the SMA path's output, and this scenario is the sanity check
that the registry / hot-swap / audit changes are purely additive. This
is the comparability anchor for the three following scenarios.

### Scenario: `btc-2023-1m-macd-trend`

- **Universe:** `BTCUSDT`
- **Period:** `2023-01-01` → `2023-12-31`
- **Granularity:** `1m`
- **Data source:** `binance-spot` (via `data/binance/BTCUSDT/2023/*.parquet`)
- **Fees:** `0.04%` taker, `0.02%` maker
- **Slippage model:** `bps: 2`
- **Initial capital:** `100_000 USDT`
- **Position sizing:** `fixed-fraction 0.1`
- **Risk limits:**
  - Max leverage: `1x`
  - Max drawdown stop: `-15%`
  - Per-symbol exposure cap: `40%`
- **Strategy params:** `btc_macd_trend` composed TOML (R6.1) —
  `signal = "macd_hist(12,26,9) > 0 AND close > ema(200)"`.
- **Seed:** `0xC0FFEE`
- **Baseline report:** `btc-2023-1m-sma-baseline-refresh` (Scenario 1).

**Expected outcome (analyst hypothesis):** Sharpe likely weak or
negative. Trend rules on 1m bars are known to over-trade through chop
— the MACD-histogram sign flips frequently in range-bound regimes,
generating costly round-trips against the `0.04%` taker fee and `2bps`
slippage. The EMA(200) filter will mute some chop in H1 2023's
range-bound tape but not enough to overcome frictional cost at 1m
cadence. **We are testing the composition machinery, not the edge.**
The value of this run is (a) proving the rule-DSL path produces a
deterministic signal sequence end-to-end, (b) generating the first
non-SMA journal entries in the ledger under a composed strategy, and
(c) populating the strategies panel with real-world signal counts for
UI validation. Any positive Sharpe is worth investigating (could be a
fee-model bug, could be survivorship in the EMA warmup, could be real
— in that order of prior probability).

### Scenario: `btc-2023-1m-rsi-reversion`

- **Universe:** `BTCUSDT`
- **Period:** `2023-01-01` → `2023-12-31`
- **Granularity:** `1m`
- **Data source:** `binance-spot` (via `data/binance/BTCUSDT/2023/*.parquet`)
- **Fees:** `0.04%` taker, `0.02%` maker
- **Slippage model:** `bps: 2`
- **Initial capital:** `100_000 USDT`
- **Position sizing:** `fixed-fraction 0.1`
- **Risk limits:**
  - Max leverage: `1x`
  - Max drawdown stop: `-15%`
  - Per-symbol exposure cap: `40%`
- **Strategy params:** `btc_rsi_reversion` composed TOML (R6.2) —
  `signal = "rsi(14) < 30 AND close > min(low, 20)"`.
- **Seed:** `0xC0FFEE`
- **Baseline report:** `btc-2023-1m-sma-baseline-refresh` (Scenario 1).

**Expected outcome (analyst hypothesis):** Mean-reversion on 1m spot
BTC is noisy — intraday noise swamps the 14-bar RSI signal, the
support-floor filter (`close > min(low, 20)`) catches some
falling-knife patterns but at the cost of missed re-entries after
clean washouts. Expect poor risk-adjusted return with a non-trivial
drawdown; some of the deeper RSI dips in 2023 coincide with macro
risk-off days that continued lower than a 20-bar floor would suggest
was safe. **This scenario primarily exercises the rule engine** and
the AND-combinator path in the parser, and provides the ledger
material to prove per-strategy attribution slicing works (this
strategy's fills must be cleanly separable from Scenario 2's in the
ledger when both run concurrently in a future paper session).

### Scenario: `btc-2023-1m-bbands-mean-revert`

- **Universe:** `BTCUSDT`
- **Period:** `2023-01-01` → `2023-12-31`
- **Granularity:** `1m`
- **Data source:** `binance-spot` (via `data/binance/BTCUSDT/2023/*.parquet`)
- **Fees:** `0.04%` taker, `0.02%` maker
- **Slippage model:** `bps: 2`
- **Initial capital:** `100_000 USDT`
- **Position sizing:** `fixed-fraction 0.1`
- **Risk limits:**
  - Max leverage: `1x`
  - Max drawdown stop: `-15%`
  - Per-symbol exposure cap: `40%`
- **Strategy params:** `btc_bbands_mean_revert` composed TOML (R6.3) —
  `signal = "close < bollinger_lower(20,2) AND volume > 1.5 * avg(volume, 20)"`.
- **Seed:** `0xC0FFEE`
- **Baseline report:** `btc-2023-1m-sma-baseline-refresh` (Scenario 1).

**Expected outcome (analyst hypothesis):** The `volume > 1.5 ×
avg_volume(20)` filter should cut trade count dramatically vs the SMA
baseline — most Bollinger-lower touches on 1m BTCUSDT happen on thin
volume and are noise rather than capitulation. Trade count likely
falls `> 5×` vs SMA baseline; Sharpe may or may not beat the baseline
— the hypothesis is agnostic because the filter is doing most of the
work and we do not have strong priors on whether high-volume Bollinger
touches are better or worse than low-volume ones on 1m timeframes.
**The analyst value here is the exercise of the numeric-literal path
in the rule parser (`1.5 *`), the rolling-reducer node (`avg(volume,
20)`), and the multi-indicator composition (Bollinger + volume
reducer in a single rule).** If Sharpe does beat baseline we check
for look-ahead bias in `avg(volume, 20)` before celebrating — the
reducer must include only bars strictly prior to the current one.

## Design

Translates R1–R10 into crate / module additions, Rust types, TOML schema,
message types, and test strategy. All decisions anchor to
[architecture.md — Strategy registry & hot-loading](../architecture.md#strategy-registry--hot-loading)
and the five v0.5 resolutions (Q1–Q5) signed there on 2026-04-19. This
section is `ComposedStrategy` + hot-load wiring; v0 crate surfaces stay
untouched except for additive extensions.

### Crate map delta from v0

| Crate       | Change in v0.5                                                                                                                                                                     |
|-------------|------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `trading_core` | **+** New message types `StrategyLoaded` / `StrategySwapped` / `StrategyLoadError` (Q5). **+** Read-side `StrategyEventView` + `StrategyEventKind`. **No trait changes** to `Strategy`. |
| `features`  | **+** New streaming indicators: EMA, MACD (line + signal + histogram), RSI, Bollinger Bands. Implemented as pure-`Decimal` adapters next to the existing `features::sma` (no new TA dep). |
| `strategy`  | **+** New submodule `strategy::composed` with `ComposedStrategy` (`impl Strategy`), indicator-node tree, rule-node tree, content-hasher, and `StrategyConfig` parse/validate. **+** `strategy::registry` gains `parking_lot::RwLock` + `swap`/`unload` wired to `audit::journal::strategy_event`. |
| `audit`     | **+** New migration `0003_strategy_events.sql` + `audit::journal::strategy_event(..)` writer + `audit::query::strategy_events_since` / `strategy_history` reader. The existing `registry_event` zero-memo path (used by kill-switch) stays. |
| `agent`     | **+** File-watcher task (`notify` re-used from v0 kill-switch wiring). **+** Three new `EventBus` channels. **+** Debounce + swap pipeline. |
| `ui`        | **+** `widgets::strategies` panel. **+** `ui::strings::STRATEGIES_*` keys. **+** Live subscriber for the three new bus channels. |
| `backtest`  | **+** `--strategy <id>` CLI flag resolving `config/strategies/<id>.toml` once at run start. Report writer gains a `Strategy` section (id + hash + source path). |
| `risk`      | Unchanged in v0.5. v1+ will grow `max_strategy_drawdown_pct` — see [architecture.md — v0.5 ComposedStrategy exit policy (Q3)](../architecture.md#v05--composedstrategy-exit-policy-q3--confirmed-2026-04-19). `// TODO(v1): max_strategy_drawdown` breadcrumb lives in `risk::RiskLimits`. |
| `data`, `exec`, `models`, `llm`, `cost` | Unchanged. |

Dependency edges (additive):

```
trading_core ← strategy::composed, audit::journal::strategy_event,
               agent::watcher, ui::widgets::strategies
audit        ← strategy::registry (swap/unload writes)
features     ← strategy::composed (indicator nodes)
strategy     ← agent::watcher (registry handle)
```

No new crate is introduced. No edge reverses.

### `ComposedStrategy` type (R1)

Implements the existing v0 `Strategy` trait verbatim — no trait changes.

```rust
// crates/strategy/src/composed/mod.rs
pub struct ComposedStrategy {
    id: StrategyId,
    symbol: Symbol,
    hash: [u8; 32],              // sha256 of canonicalized AST; stable across runs
    source_path: SmolStr,

    // Evaluation state — allocation-free on the hot path.
    indicators: Vec<IndicatorNode>,   // owns ring buffers sized at construction
    rule:       RuleNode,             // immutable tree of comparisons / boolean ops
    last_rule_value: Option<bool>,    // drives edge-triggered signal emission (R1.3)

    // Sizing reference — resolved once at load time.
    sizing: Sizing,                   // Sizing::FixedFraction(Decimal) only in v0.5 (R2.4)
    params: ParamMap,                 // named scalars from [params] TOML table
}

impl Strategy for ComposedStrategy {
    fn id(&self) -> StrategyId { self.id.clone() }

    fn on_bar(&mut self, bar: &Bar) -> Vec<Signal> {
        // 1. Push bar into each indicator node; each advances its ring buffer
        //    and updates its latest computed value. No allocation on this path.
        for ind in &mut self.indicators { ind.on_bar(bar); }

        // 2. Evaluate the rule tree against the current indicator values +
        //    bar-native references (close/open/high/low/volume).
        let now = self.rule.eval(&EvalCtx::new(bar, &self.indicators, &self.params));

        // 3. Edge-triggered emission — symmetric signal-flip (Q3).
        let out = match (self.last_rule_value, now) {
            (Some(false), true)  => vec![self.emit_signal(bar, SignalKind::Buy)],
            (Some(true),  false) => vec![self.emit_signal(bar, SignalKind::Sell)],
            _                    => vec![],
        };
        self.last_rule_value = Some(now);
        out
    }

    fn on_tick(&mut self, _tick: &Tick) -> Vec<Signal> { vec![] }

    fn config_schema() -> serde_json::Value where Self: Sized {
        // Returns the JSON-Schema for ComposedStrategyConfig so a future
        // cockpit "edit strategy" flow can validate before writing to disk.
        ComposedStrategyConfig::json_schema()
    }
}
```

**Indicator node taxonomy** (R1.2):

```rust
pub enum IndicatorNode {
    Sma    { period: u32, ring: RingBuffer<Decimal>, latest: Option<Decimal> },
    Ema    { period: u32, alpha: Decimal,            latest: Option<Decimal> },
    Macd   { fast: u32, slow: u32, signal_period: u32,
             fast_ema: Ema, slow_ema: Ema, signal_ema: Ema,
             line: Option<Decimal>, signal_line: Option<Decimal>, hist: Option<Decimal> },
    Rsi    { period: u32, gains: RingBuffer<Decimal>, losses: RingBuffer<Decimal>,
             latest: Option<Decimal> },
    Bbands { period: u32, mult: Decimal,
             ring: RingBuffer<Decimal>,
             upper: Option<Decimal>, mid: Option<Decimal>, lower: Option<Decimal> },
    // Value nodes — bar-native, zero state.
    Close, Open, High, Low, Volume, TradeCount,
    // Rolling reducers over bar-native values — parameterized at construction.
    RollingMin { field: BarField, window: u32, ring: RingBuffer<Decimal>, latest: Option<Decimal> },
    RollingMax { field: BarField, window: u32, ring: RingBuffer<Decimal>, latest: Option<Decimal> },
    RollingAvg { field: BarField, window: u32, ring: RingBuffer<Decimal>, latest: Option<Decimal> },
}
```

**Rule node taxonomy** (R1.2):

```rust
pub enum RuleNode {
    // Logical combinators.
    And(Box<RuleNode>, Box<RuleNode>),
    Or (Box<RuleNode>, Box<RuleNode>),
    Not(Box<RuleNode>),

    // Comparisons produce bool; operands are Expr (Decimal-valued).
    Cmp { op: CmpOp, lhs: Expr, rhs: Expr }, // CmpOp = Lt | Le | Eq | Ge | Gt

    // Crossovers over the *previous two* values of their inner Expr pair.
    // Maintained as internal state so `cross_above(a, b)` fires exactly once
    // per crossing, matching the v0 sma_crossover edge-triggered contract.
    CrossAbove { a: Expr, b: Expr, prev: Option<(Decimal, Decimal)> },
    CrossBelow { a: Expr, b: Expr, prev: Option<(Decimal, Decimal)> },

    // Convenience predicates compiled down from familiar sugar (see grammar).
    MacdCross { fast: u32, slow: u32, signal: u32, direction: CrossDir,
                prev: Option<(Decimal, Decimal)> },
    BollingerLowerTouch { period: u32, mult: Decimal },
}

pub enum Expr {
    Indicator(IndicatorRef),   // handle into ComposedStrategy::indicators
    BarField(BarField),        // close, open, high, low, volume, trade_count
    Param(SmolStr),             // [params] scalar
    Literal(Decimal),           // numeric literal from TOML
    Binary { op: ArithOp, lhs: Box<Expr>, rhs: Box<Expr> },  // + - * / — DSL supports `1.5 * avg(volume, 20)`
}
```

**Allocation discipline (R1.3, R1.4):** indicator ring buffers are sized to
the deepest lookback at construction; `on_bar` performs no `Vec::push`, no
`String` formatting, no `format!`, no `serde` calls. The `Vec<Signal>`
returned is the only allocation, bounded to 0 or 1 items per bar under
symmetric signal-flip semantics. Verified by a criterion bench with
`heaptrack` scratchpad (see R10).

**Content hash (R3.2, R4.2):** after parse + typecheck, canonicalize the
AST into a deterministic byte sequence (indicator nodes sorted by their
TOML-parse order, rule tree serialized depth-first with fixed-width
separators, parameter map sorted by key) and sha256 it. The 32-byte
digest is what the audit ledger and broadcast-bus events carry.

### Rule DSL grammar — TOML schema

**Per-strategy file** at `config/strategies/<id>.toml` — filename stem is
the canonical `StrategyId` (R2.1).

```toml
# config/strategies/btc_macd_trend.toml
id     = "btc_macd_trend"                   # MUST equal filename stem
kind   = "composed"                         # future: "wasm" (v1+)
symbol = "BTCUSDT"                          # single-symbol in v0.5
stage  = "research"                         # "research" | "paper"
signal = "macd_hist(12,26,9) > 0 AND close > ema(200)"
size   = "fixed_fraction(0.1)"              # only fixed_fraction in v0.5

[params]                                    # optional named scalars
rsi_floor    = 35
vol_multiple = 1.5
```

`serde` struct (in `strategy::composed::config`):

```rust
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ComposedStrategyConfig {
    pub id:     SmolStr,
    pub kind:   StrategyKind,            // ComposedKind { kind: "composed" } discriminator
    pub symbol: SmolStr,
    pub stage:  Stage,                   // Research | Paper (Live rejected at load)
    pub signal: SmolStr,                 // raw rule string — parsed + typechecked in load()
    pub size:   SmolStr,                 // e.g. "fixed_fraction(0.1)"
    #[serde(default)]
    pub params: BTreeMap<SmolStr, Decimal>,
}
```

**Rule-DSL grammar** (PEG, one-line, informative — actual implementation
uses a small hand-written recursive-descent parser on top of the
`logos` lexer or `winnow` combinators; dep choice is developer-owned
but no new runtime dep vs what's already in the workspace). Productions:

```
rule        := or_expr
or_expr     := and_expr ("OR" and_expr)*
and_expr    := not_expr ("AND" not_expr)*
not_expr    := "NOT" not_expr | cmp
cmp         := value_expr (CMP_OP value_expr)?
                 // bare value_expr is promoted to `value_expr != 0`
CMP_OP      := "<" | "<=" | "==" | ">=" | ">" | "!="
value_expr  := term (ARITH_OP term)*
ARITH_OP    := "+" | "-" | "*" | "/"
term        := indicator_call
             | bar_field
             | param_ref
             | numeric_literal
             | "(" rule ")"
indicator_call := INDICATOR_NAME "(" numeric_literal ("," numeric_literal)* ")"
INDICATOR_NAME ∈ { sma, ema, macd_line, macd_signal, macd_hist, macd_cross,
                    rsi, bollinger_upper, bollinger_mid, bollinger_lower,
                    bollinger_lower_touch, min, max, avg, cross_above, cross_below }
bar_field   ∈ { close, open, high, low, volume, trade_count }
param_ref   := IDENTIFIER   // must appear as a key in [params]
```

**Supported rule examples** (R2.3, all covered by parser unit tests):

```toml
signal = "macd_cross(12,26,9)"                                  # crossover sugar
signal = "macd_cross(12,26,9) AND rsi(14) < 35"                 # AND
signal = "bollinger_lower_touch(20,2) OR rsi(14) < 20"          # OR with threshold
signal = "(rsi(14) < rsi_floor OR macd_cross(12,26,9)) AND NOT (close < min(low, 20))"
signal = "close < bollinger_lower(20,2) AND volume > 1.5 * avg(volume, 20)"
signal = "macd_hist(12,26,9) > 0 AND close > ema(200)"
```

**Parse → typecheck → construct** (R2.5):

1. `ComposedStrategyConfig` deserializes the TOML (`deny_unknown_fields`).
2. `signal` string goes through the DSL parser → raw `RuleAst`.
3. Typechecker walks the AST:
   - every `INDICATOR_NAME` must be in the supported set with correct
     arity;
   - every `param_ref` must appear in `[params]`;
   - numeric ranges: RSI period ≥ 2, MACD `fast < slow`, Bollinger
     `mult > 0`, lookback windows ≥ 1.
4. Indicator nodes are deduplicated (two `rsi(14)` references share one
   node) and sorted deterministically for content hashing.
5. `size` string parsed into `Sizing::FixedFraction(Decimal)`; anything
   else returns `StrategyLoadError { error_code: "unsupported_sizing" }`.
6. Ring buffers allocated with capacity = deepest lookback.
7. AST canonicalized + sha256 hashed → stored as `ComposedStrategy.hash`.

**Error codes** emitted via `StrategyLoadError` (R2.5, R8.2):

| code                      | cause                                                  |
|---------------------------|--------------------------------------------------------|
| `io_read`                 | file removed / permission denied during reload         |
| `toml_parse`              | malformed TOML syntax                                  |
| `unknown_field`           | `deny_unknown_fields` hit                              |
| `id_filename_mismatch`    | `id` field does not equal filename stem                |
| `grammar_parse`           | rule-DSL syntax error                                  |
| `unknown_indicator`       | indicator name not in the supported set                |
| `arity_mismatch`          | e.g. `macd_cross(12)` — expected 3 args                |
| `unknown_param`           | `param_ref` not declared in `[params]`                 |
| `invalid_range`           | `fast >= slow` in MACD, period < 2 in RSI, etc.        |
| `invalid_stage`           | stage not in {`research`,`paper`}                       |
| `unsupported_sizing`      | sizing expression not `fixed_fraction(<f>)`             |
| `empty_signal`            | signal string empty / whitespace-only                  |

**Broadcast event** on reject carries `error_code` + `error_summary`;
`audit::journal::strategy_event` persists them in the `strategy_events`
table.

### File watcher + atomic swap (R3)

Lives in `agent::watcher` as a tokio task spawned by the agent binary:

```rust
pub async fn run_strategy_watcher(
    strategies_dir: PathBuf,
    registry: Arc<parking_lot::RwLock<StrategyRegistry>>,
    ledger: audit::Ledger,
    bus: Arc<EventBus>,
    cancel: CancellationToken,
) -> Result<(), WatcherError> { /* ... */ }
```

**Mechanics:**

1. `notify::recommended_watcher(strategies_dir)` on `Create | Modify |
   Remove | Rename` (the `notify` crate is already workspace-pinned from
   v0's kill-switch file watcher).
2. Events feed a `debounce::<Duration=250ms>` (per R3.1 — collapse editor
   write-storms). If multiple events for the same file arrive inside
   250ms, only the final state is processed.
3. For each debounced event, dispatch:
   - **Create / Modify**: `load_and_swap(path, &registry, &ledger, &bus)`.
   - **Remove / Rename-out**: `unload(id, &registry, &ledger, &bus)`.
4. `load_and_swap`:
   1. Read file bytes (`io_read` on failure → emit
      `StrategyLoadError`).
   2. `ComposedStrategyConfig::parse(bytes)` + typecheck + build
      (`toml_parse` / grammar / arity / unknown_* / invalid_* → emit
      `StrategyLoadError`).
   3. On success: acquire `registry.write()` guard **briefly** (lock
      held only for the `HashMap::insert` call — parse + typecheck +
      construction happen outside the guard), call `registry.swap(id,
      Box::new(new))`, drop the guard.
   4. `audit::journal::strategy_event(..)` writes the `Load` or `Swap`
      row (new-swap keeps both old + new hash).
   5. `bus.publish_strategy_loaded(..)` or
      `bus.publish_strategy_swapped(..)`.
5. `unload`:
   1. Acquire write guard, remove from `HashMap`, drop guard.
   2. Write `Unload` strategy-event row.
   3. (No broadcast-bus event for unload in v0.5 — the cockpit
      reconciles from `strategy_history`; a later tier adds an
      `strategy_unloaded` channel if needed.)

**Atomicity from the runtime's view (R3.4):** the `RwLock` guarantees
that `on_bar` fan-out either sees the old strategy or the new one,
never a partially-constructed state. Because construction happens
outside the guard, a slow parse / build cannot block the bar-close
critical path; the guard is held only for the pointer swap.

**Open-position persistence (R3.3):** positions live in `PositionBook`
owned by `agent`, not in the strategy. Unloading a strategy leaves its
positions open; the operator can close them manually via the cockpit or
let a replacement strategy manage them. This preserves the ledger's
property that every fill references a `strategy_id` even if that
strategy is no longer loaded.

**Latency (R3.5):** `file-save → next-bar` ≤ 2s is dominated by (a) the
250ms debounce + (b) the remainder of the current bar window. Parse +
construct + swap is ≤ 10ms for v0.5-complexity rules (measured in R10
bench).

### Strategy-event audit schema (R4, Q1 resolution)

See [architecture.md — v0.5 strategy-event journal schema (Q1)](../architecture.md#v05--strategy-event-journal-schema-q1--confirmed-2026-04-19)
for the full table / writer / reader signatures. Summary:

- **Table** `strategy_events` lives in the same SQLite ledger DB as
  `journal_entries`.
- **Writer** `audit::journal::strategy_event(ledger, StrategyEventWrite)`
  inserts a single row inside a `sqlx::Transaction`.
- **Reader** `audit::query::strategy_events_since(ts)` and
  `audit::query::strategy_history(id)` — return `Vec<StrategyEventView>`;
  `StrategyEventView` is defined in `trading_core`.
- **Monetary invariant** unchanged: reconciler walks `journal_entries`
  only; `strategy_events` has no debit/credit columns.

**Migration shape** (new file
`crates/audit/migrations/0003_strategy_events.sql`):

```sql
CREATE TABLE IF NOT EXISTS strategy_events (
    id            TEXT PRIMARY KEY,
    ts            TEXT NOT NULL,
    kind          TEXT NOT NULL,
    strategy_id   TEXT,
    old_hash      TEXT,
    new_hash      TEXT,
    source_path   TEXT,
    operator      TEXT NOT NULL DEFAULT 'system',
    error_code    TEXT,
    error_summary TEXT
);
CREATE INDEX IF NOT EXISTS strategy_events_ts_idx ON strategy_events(ts);
CREATE INDEX IF NOT EXISTS strategy_events_sid_idx ON strategy_events(strategy_id, ts);
```

The existing `journal::registry_event` function (which writes zero-amount
memo rows into `journal_entries` against `equity:opening_balance`) is
**kept** for the kill-switch `KillSwitchTripped` memo path. v0.5 strategy
lifecycle events go to the new dedicated table.

### New broadcast events (Q5 resolution)

See [architecture.md — v0.5 broadcast bus extensions (Q5)](../architecture.md#v05--broadcast-bus-extensions-q5--confirmed-2026-04-19)
for the full message types + channel additions. Summary:

- `trading_core::{StrategyLoaded, StrategySwapped, StrategyLoadError}` —
  carry `StrategyId`, 32-byte sha256 hashes, source path, timestamp.
- `agent::EventBus` gains `strategy_loaded`, `strategy_swapped`,
  `strategy_error` broadcast channels (capacity 32 each).
- Backpressure: `RecvError::Lagged(n)` → log-and-continue in the UI
  subscriber, identical to existing v0 pattern per
  [dev-week2-broadcast-api-2026-04-18.md](../reports/dev-week2-broadcast-api-2026-04-18.md).
- `RecvError::Closed` → UI flips the Strategies panel into
  `PanelState::Error(STRATEGIES_CONNECTION_CLOSED)`.

### Cockpit strategies panel (R5, Q4 resolution)

**Position:** right column, above Open positions. See
[architecture.md — v0.5 cockpit strategies panel layout (Q4)](../architecture.md#v05--cockpit-strategies-panel-layout-q4--confirmed-2026-04-19).

**Updated wireframe:**

```
┌────────────────────────────────────────────────────────────────────────────┐
│ Trading Cockpit                                                            │
├──────────────────────────────────┬─────────────────────────────────────────┤
│ ┌──────────────────────────────┐ │ ┌─────────────────────────────────────┐ │
│ │ P&L                          │ │ │ Strategies                          │ │
│ │ Total equity     90,129.50   │ │ │ ID                 Hash  Status     │ │
│ │ Cash             50,000.00   │ │ │ btc_macd_trend     a1b2  Ready      │ │
│ │ Realized today      129.50   │ │ │ btc_rsi_reversion  c3d4  Ready      │ │
│ │ Unrealized            0.00   │ │ │ btc_bb_mean_revert e5f6  Error      │ │
│ │ P&L today           129.50   │ │ │   ↳ arity_mismatch: macd_cross(12)  │ │
│ └──────────────────────────────┘ │ └─────────────────────────────────────┘ │
│ ┌──────────────────────────────┐ │ ┌─────────────────────────────────────┐ │
│ │ Feed latency  120ms          │ │ │ Open positions                      │ │
│ └──────────────────────────────┘ │ │ ...                                 │ │
│ ┌──────────────────────────────┐ │ └─────────────────────────────────────┘ │
│ │ Stop trading                 │ │ ┌─────────────────────────────────────┐ │
│ │ [ big red button ]           │ │ │ Live tape                           │ │
│ └──────────────────────────────┘ │ │ ...                                 │ │
│                                  │ └─────────────────────────────────────┘ │
└──────────────────────────────────┴─────────────────────────────────────────┘
```

**Model extensions** (`crates/ui/src/state.rs`, additive):

```rust
#[derive(Debug, Clone)]
pub struct StrategyRow {
    pub id:            StrategyId,
    pub short_hash:    SmolStr,      // 7-char prefix of hex
    pub full_hash:     SmolStr,      // tooltip only
    pub status:        StrategyStatus,
    pub last_event:    Option<StrategyEventView>,
    pub signals_60s:   u32,
    pub has_position:  bool,
    pub source_path:   SmolStr,
}

#[derive(Debug, Clone)]
pub enum StrategyStatus { Loading, Ready, Error(SmolStr) }

pub struct Cockpit {
    // ... existing fields ...
    pub strategies: PanelState<Vec<StrategyRow>>,
    pub strategies_signal_counters: HashMap<StrategyId, RingBuffer60s>,
}
```

**Message additions:**

```rust
pub enum Message {
    // ... existing variants ...

    // Strategy panel inputs — from the three new bus channels.
    StrategyLoaded(StrategyLoaded),
    StrategySwapped(StrategySwapped),
    StrategyLoadError(StrategyLoadError),

    // Refreshed snapshot (from audit::query::strategy_events_since at BarClose).
    StrategiesRefreshed(Vec<StrategyRow>),
    StrategiesError(SmolStr),

    // Per-bar: fills produced during last bar, keyed by strategy_id.
    //         Used to increment the 60s signal counter.
    StrategySignalObserved(StrategyId),
}
```

**Four panel states (V7, R5.2):**

| State     | Copy source                                                         | Trigger                                          |
|-----------|---------------------------------------------------------------------|--------------------------------------------------|
| `Loading` | `STRATEGIES_LOADING` — "Connecting to the strategy registry…"       | Initial cockpit startup; no data yet.            |
| `Empty`   | `STRATEGIES_EMPTY` — "No strategies loaded. Add a TOML under config/strategies/." | Registry returns empty vector.                   |
| `Error`   | `STRATEGIES_ERROR_PREFIX` + error                                   | `RecvError::Closed` on any of the three channels.|
| `Ready`   | table with N rows                                                   | Registry has ≥ 1 strategy.                       |

Each row in `Ready` can itself show a per-row `Error` badge (from the
`StrategyStatus::Error` variant) with `error_summary` underneath — this
is the R8 "malformed TOML keeps old strategy running" visual.

**`ui::strings` additions** (prefix `STRATEGIES_*`):

```rust
pub const PANEL_STRATEGIES_TITLE:      &str = "Strategies";
pub const STRATEGIES_LOADING:          &str = "Connecting to the strategy registry…";
pub const STRATEGIES_EMPTY:            &str = "No strategies loaded. Add a TOML under config/strategies/.";
pub const STRATEGIES_ERROR_PREFIX:     &str = "Can't read the strategy registry: ";
pub const STRATEGIES_CONNECTION_CLOSED: &str = "Registry channel closed — restart the agent.";
pub const STRATEGIES_COL_ID:           &str = "Strategy";
pub const STRATEGIES_COL_HASH:         &str = "Hash";
pub const STRATEGIES_COL_STATUS:       &str = "Status";
pub const STRATEGIES_COL_LAST_EVENT:   &str = "Last event";
pub const STRATEGIES_COL_SIGNALS_60S:  &str = "Signals / 60s";
pub const STRATEGIES_COL_POSITION:     &str = "Holds position";
pub const STRATEGIES_STATUS_READY:     &str = "Ready";
pub const STRATEGIES_STATUS_LOADING:   &str = "Loading";
pub const STRATEGIES_STATUS_ERROR:     &str = "Error";
pub const STRATEGIES_EVENT_LOAD:       &str = "loaded";
pub const STRATEGIES_EVENT_SWAP:       &str = "swapped";
pub const STRATEGIES_EVENT_UNLOAD:     &str = "unloaded";
pub const STRATEGIES_EVENT_REJECT:     &str = "rejected";
```

**Theme tokens:** reuse `theme::color::{success, warning, danger, muted}`
— no new tokens. Row-level error state uses `color::danger`; Ready uses
`color::success`; Loading uses `color::muted`.

### Backtest harness alignment (R9)

`backtest` binary gains a `--strategy <id>` CLI flag; resolution:

1. If `<id>` resolves to a compiled-in strategy (v0: `sma_crossover`),
   use it (backward compatibility, R9.2).
2. Otherwise look for `config/strategies/<id>.toml`; load via the same
   `ComposedStrategy::from_config` path used by the live agent.
3. **No file watcher inside backtest** — config resolved once at run
   start; determinism requires the strategy not to change mid-run.

Report writer (R9.3) extends the existing template with a `Strategy`
subsection emitting:

```
### Strategy

- **id:** btc_macd_trend
- **kind:** composed
- **hash:** a1b2c3d4e5f6…  (full 64-char sha256)
- **source:** config/strategies/btc_macd_trend.toml
- **signal:** macd_hist(12,26,9) > 0 AND close > ema(200)
```

Determinism invariant (R9.4): two runs of `--strategy btc_macd_trend
--seed 0xC0FFEE` produce byte-identical report bodies (same hash path
as v0 T33).

### Performance budget

Restated from
[architecture.md — Performance budget](../architecture.md#performance-budget):

| Path                          | Budget      | Applies to v0.5?            |
|-------------------------------|-------------|------------------------------|
| Bar-close → signal (no LLM)   | < 5 ms p99  | Yes — unchanged from v0.    |
| Backtest throughput           | > 100k bars/s per symbol per thread | Yes — unchanged. |

**v0.5-specific targets (R10.1, R10.2):**

| Target                                              | Budget       |
|-----------------------------------------------------|-------------:|
| `ComposedStrategy::on_bar` — 1-rule (e.g. `rsi(14) < 30`)              | < 200 µs p99 |
| `ComposedStrategy::on_bar` — 3-rule (MACD-trend shape)                 | < 500 µs p99 |
| `ComposedStrategy::on_bar` — 5-rule mixed (bbands + volume + reducers) | < 1 ms p99   |
| Strategy parse + construct (from TOML bytes in memory)                 | < 10 ms      |
| File-save → registry swap (on warm OS cache)                           | < 500 ms     |

Bench home: `crates/strategy/benches/composed_strategies.rs`. Criterion
baseline committed to `criterion_baselines/`; regressions > 10% fail the
bench step (R10.3).

### Test strategy

| Layer                         | Tests                                                                                                   | Crate(s)       | Tool         |
|-------------------------------|---------------------------------------------------------------------------------------------------------|----------------|--------------|
| **Unit — parser**             | DSL grammar covers all R2.3 rule shapes; negative cases produce distinct `StrategyLoadError`            | `strategy`     | `cargo test` |
| **Unit — typecheck**          | Arity, unknown-indicator, unknown-param, invalid-range, invalid-stage, unsupported-sizing               | `strategy`     | `cargo test` |
| **Unit — engine**             | Programmatic `ComposedStrategy` vs hand-coded reference → byte-identical signal sequence (R1 acceptance)| `strategy`     | `cargo test` |
| **Property — parser**         | Round-trip parse → canonicalize → re-parse preserves AST for 1 000 generated valid rules                | `strategy`     | `proptest`   |
| **Property — engine**         | For any deterministic indicator sequence, `on_bar` emits ≤ 1 signal per bar and signal-flip is symmetric | `strategy`     | `proptest`   |
| **Integration — R7 hot-swap** | Replay driver + file rewrite at t=500 → swap within 2s; `strategy_history` shows exactly `Load` + `Swap`| `agent`        | `cargo test` |
| **Integration — R8 reject**   | 10 malformed TOML fixtures; each rejected without crash; good strategy keeps running                     | `agent`        | `cargo test` |
| **Integration — reconcile**   | R7 + R8 suites leave `journal_entries` balanced; `ledger_imbalance_total == 0` after the run            | `audit`        | `cargo test` |
| **Snapshot — UI**             | Strategies panel: Loading / Empty / Ready(3 rows) / Error / per-row-error                                | `ui`           | `insta`      |
| **Snapshot — report**         | Composed-strategy report body sha256 stable at seed `0xC0FFEE`                                          | `backtest`     | `insta` + direct hash |
| **Determinism**               | Each of the four backtest scenarios runs twice at `0xC0FFEE` → byte-identical report + empty DB diff    | `backtest`     | `cargo test` |
| **Bench**                     | 1-rule / 3-rule / 5-rule `on_bar` p99; backtest throughput > 100k bars/s                                | `strategy`, `backtest` | `criterion` |


## Verification

The tester's contract for declaring v0.5 composed-strategies done. All
items must be green before a `VERDICT → PASS` can be issued.

- **V1 Static checks pass.**
  - `cargo fmt --check` clean.
  - `cargo clippy --workspace --all-targets -- -D warnings` clean,
    including the v0-scoped `float_arithmetic` / `unwrap_used` /
    `expect_used` deny lints (unchanged from v0 R2.2 / R2.3).
  - `cargo audit` shows no unpatched advisories.
  - `cargo deny check` (bans, licenses, sources) passes. The
    `notify` crate dep is already on the allow list from v0.
- **V2 Unit + integration tests pass.** `cargo test --workspace`
  produces zero failures. In particular:
  - R1 unit test — programmatic `ComposedStrategy` matches hand-coded
    reference.
  - R2 unit + property tests — grammar coverage of the four R2.3
    example rules + ten malformed-TOML fixtures from R8.2.
  - R7 hot-swap roundtrip — swap observed within 2s, two
    `strategy_event` entries (swap-out + swap-in) present in the
    ledger.
  - R8 invalid-config rejection — malformed TOML rejected, old
    strategy keeps running, `Reject` entry recorded, cockpit error
    state visible.
  - R4 reconciliation — the v0 R3.5 minute-boundary invariant still
    passes at every bar across all four new scenarios.
- **V3 All four backtest scenarios run end-to-end.** Reports produced
  under `spec/<feature>/reports/backtest-<stamp>-<scenario-slug>.md` conforming
  to the tester template section 5. The `Strategy` section (R9.3)
  carries id + content hash + source path.
- **V4 Baseline re-run matches v0.** Scenario 1
  (`btc-2023-1m-sma-baseline-refresh`) produces a report whose body
  sha256 matches the v0 `btc-2023-1m-sma-cross` report byte-for-byte,
  confirming the composed-strategies work is purely additive.
- **V5 Determinism holds per scenario.** Each of the four scenarios
  is run twice at seed `0xC0FFEE`; both runs of each scenario
  produce byte-identical report bodies (sha256 match). Ledger DB
  exports diff-empty per scenario.
- **V6 Criterion benches meet budget.** `cargo bench -p strategy`
  shows:
  - `on_bar` p99 `< 5ms` across all three bench cases in R10.2.
  - Backtest throughput `> 100k bars/s` on a single thread against
    the 2023 BTCUSDT fixture.
  - No `> 10%` regression vs the committed baselines (R10.3).
- **V7 Cockpit smoke (strategies panel).**
  - Launching `cockpit` against the `ReplayFeed` shows the strategies
    panel with all loaded TOML files, correct short-hashes, and live
    signal counts per bar.
  - During the R7 integration-style replay, a swap is visible in the
    panel within 2s of the file rewrite; the short-hash flips.
  - Each panel state (empty / loading / error / ready) is reachable
    via scripted actions and snapshots pass via `insta`.
  - Rejection of a malformed TOML during the run shows a red per-row
    error state carrying the `error_summary` from the `Reject`
    entry.
- **V8 Audit replay.**
  - After the R7 + R8 suites run, walking the ledger's
    `strategy_event` rows via `audit::query::strategy_history` for
    each test's strategy reconstructs the full swap history in
    order, with hashes matching the on-disk TOMLs at each historical
    point (stored fixtures).
  - The reconciliation invariant (v0 R3.5) still holds —
    `ledger_imbalance_total == 0` after every scenario.
- **V9 Cost telemetry.** The generated `costs.md` for this feature's
  runs shows `LLM tokens: $0.00` (v0.5 composed strategies are
  deterministic per
  [product.md → Cost economics](../product.md#cost-economics--monthly-ceiling)).

Failure on any of V1–V9 routes as follows (matches the router in
[AGENT.md → verdict routing](../../AGENT.md#canonical-workflow)):

- Static / test / bench failure → `developer`.
- UI regression (strategies panel) → `ui-designer`.
- Structural (registry atomicity, audit schema, watcher contract)
  → `architect`.
- Strategy / scenario hypothesis wrong (unexpected Sharpe, trade
  count off by > 10× vs hypothesis) → `analyst`.

## Implementation

_developer fills this during build._

## UI — v0.5

_Partial fill by ui-designer (2026-04-19). T522–T525 + T527 + T528 landed
against `trading_core`'s v0.5 types (T501). T526 (live subscribers) and
T_FINAL_B (smoke extension) remain blocked on developer T512 — see
[ui-v05-blockers-2026-04-19.md](../reports/ui-v05-blockers-2026-04-19.md)._

### Panel landed — strategies

- **Placement:** right column, **above** Open positions (Q4 resolution);
  v0 layout for the left column (P&L, latency, kill switch) unchanged.
  Snapshot `cockpit_layout_strategies_above_positions` pins the order.
- **States:** loading / empty / error / ready with plain-language copy and
  a per-row error badge (R8 visual) when an individual strategy's last
  load attempt was rejected.
- **Subscriptions** _(blocked — wires once T512 adds the three broadcast
  channels)_: `strategy_loaded` → `Message::StrategyLoaded`,
  `strategy_swapped` → `Message::StrategySwapped`, `strategy_error` →
  `Message::StrategyLoadError`.
- **Fixture path:** `fake_cockpit_with_strategies()` boots the cockpit
  with three rows (Ready / Loading / Error) so the whole panel state
  surface renders without a running agent — `cargo run --bin cockpit
  --features fixtures` exercises the layout today.

### Strings added

Every user-visible string lives in `ui::strings`; widgets carry zero
literals. New constants (all prefixed `STRATEGIES_*` except the panel
title; keys → English values):

| Key | Value |
|---|---|
| `PANEL_STRATEGIES_TITLE` | "Strategies" |
| `STRATEGIES_LOADING` | "Loading active strategies…" |
| `STRATEGIES_EMPTY` | "No strategies loaded. Drop a TOML under config/strategies/ to begin." |
| `STRATEGIES_ERROR_PREFIX` | "Can't read strategies: " |
| `STRATEGIES_COL_ID` | "Strategy" |
| `STRATEGIES_COL_HASH` | "Hash" |
| `STRATEGIES_COL_STATUS` | "Status" |
| `STRATEGIES_COL_LAST_EVENT` | "Last event" |
| `STRATEGIES_COL_SIGNALS_60S` | "Signals / 60s" |
| `STRATEGIES_COL_POSITION` | "Holds position" |
| `STRATEGIES_STATUS_READY` | "Ready" |
| `STRATEGIES_STATUS_LOADING` | "Loading" |
| `STRATEGIES_STATUS_ERROR` | "Error" |
| `STRATEGIES_EVENT_LOAD` | "loaded" |
| `STRATEGIES_EVENT_SWAP` | "swapped" |
| `STRATEGIES_EVENT_UNLOAD` | "unloaded" |
| `STRATEGIES_EVENT_REJECT` | "rejected" |
| `STRATEGIES_POSITION_HELD` | "yes" |
| `STRATEGIES_POSITION_FLAT` | "no" |

Reused from v0: `CONNECTION_CHANNEL_CLOSED` (error-state detail),
`PLACEHOLDER_NONE` (missing hash / last-event cell).

### Theme tokens added

**Zero.** The strategies panel reuses `color::{POS, NEG, WARN, ACCENT,
FG, FG_MUTED}` and the existing spacing scale (`space::M`, `space::S`,
`space::XS`). Deliberate: the three-goal contract treats new tokens as
a code smell and the v0.5 visual needs only the semantic colors already
carried by v0.

### Accessibility notes

- Contrast: every status-pill color / background pair reuses v0 tokens
  already hand-checked at ≥ 4.5:1 WCAG AA against the dark palette —
  no new color pair introduced.
- Color is never the only signal: the status pill carries the label
  text next to the color, and the per-row error badge shows the
  `error_summary` in words (not just a red dot).
- Tab order: the strategies panel contains no interactive elements in
  v0.5 (it is read-only). No new tab stops introduced. Kill switch
  remains the last tab stop in the left column, per v0.
- The `Hash` cell is the 7-char short hash; the full 64-char hash is
  available in the tooltip (deferred to the iced 0.14 `tooltip` wiring —
  carried as a TODO in `widgets::strategies`).

### Consistency self-audit

Run against HEAD with T522–T525 + T527 + T528 landed (T526 stub absent):

- inline strings: **0** (`no_inline_user_visible_strings_in_widgets`
  green)
- inline hex: **0** (`no_inline_hex_colors_in_widgets_or_state` green)

Expected counts — non-zero on either would be a fail.

### Deferred manual

- PNG screenshot of the strategies panel in each of its four states and
  the per-row-error visual. Capture on an operator display via `cargo
  run --bin cockpit --features fixtures` once T526 lands and
  `--features live` can drive a replay swap. Manual smoke checklist
  entry to be appended to
  [ui-week2-smoke-checklist-2026-04-18.md](../reports/ui-week2-smoke-checklist-2026-04-18.md)
  at T_FINAL_B time.

### Test counts (after T522–T525 + T527 + T528)

| Suite | Count |
|---|---|
| `cargo test -p ui` (default) | 25 lib + 2 consistency + 30 snapshots = **57** |
| `cargo test -p ui --features live` | pending — T526 adds ~3 live subscriber tests |

### Open deps (handoff)

- **T512** (developer) — three `EventBus` broadcast channels + the
  corresponding publisher methods + the v0.5 extension section in
  `spec/v0-paper-sma/reports/dev-week2-broadcast-api-2026-04-18.md`. Blocks T526.
- **T_FINAL_B** — waits on T526 + the developer's T_FINAL_A (R7
  hot-swap integration). Once T512 lands the ui-designer re-spawns to
  complete T526 + T_FINAL_B.

### T526 close-out (2026-04-19, ui-designer resume)

T512 landed on `crates/agent/src/bus.rs` with three publisher methods
(`publish_strategy_loaded` / `_swapped` / `_error`) + three matching
`bus.strategy_*()` receiver getters, capacity 32 each.  T526 wired the
corresponding UI subscribers.

- **`ui::live::Channel`** gained three variants — `StrategyLoaded`,
  `StrategySwapped`, `StrategyError` — each mapped to its own stream
  builder (`stream_strategy_loaded` / `_swapped` / `_error`). The
  `subscription(bus)` batcher now fans in nine recipes instead of six.
- **Eager-subscribe** pattern (architect risk #5): each stream calls
  `bus.strategy_*()` **before** entering the `stream!` body, closing
  the publish-before-subscribe race so events published in the gap
  between `stream()` returning and the first `.next().await` are not
  dropped.
- **Backpressure policy** matches v0: `RecvError::Lagged(n)` →
  `warn!(channel = "strategy_*", skipped = n)` + continue;
  `RecvError::Closed` → `Message::StrategiesError(SmolStr::new(
  CONNECTION_CHANNEL_CLOSED))` and the stream ends. All three
  strategy-registry channels funnel their `Closed` path into the
  single `StrategiesError` variant so the operator sees one
  panel-wide error line rather than three simultaneous ones.
- **Strings** — zero new keys added; the blocker report's plan held
  up. The three `stream_strategy_*` helpers reuse the existing
  `CONNECTION_CHANNEL_CLOSED` constant; the widget prepends
  `STRATEGIES_ERROR_PREFIX` at render time.
- **Tests** — three new `#[tokio::test]` cases appended to
  `crates/ui/tests/live_subscription.rs`:
  - `t526_strategy_loaded_stream_refreshes_cockpit` — publish one
    `StrategyLoaded`, assert the cockpit's `strategies` panel
    transitions to `Ready` with one row within 2s.
  - `t526_strategy_swapped_stream_updates_cockpit` — publish load +
    swap, assert the row's hash flips and status stays `Ready`.
  - `t526_strategy_error_stream_flips_row_to_error` — publish load +
    error (with a matching `strategy_id`), assert the per-row status
    becomes `Error("unexpected token at line 3")` while the overall
    panel stays `Ready` (R8 — old strategy keeps running).

### T_FINAL_B — closed (2026-04-19, ui-designer resume)

Developer T_FINAL_A landed the four v0.5 backtest reports under
`spec/reports/`:

- [backtest-20260419-125532-btc-2023-1m-sma-baseline-refresh.md](../reports/backtest-20260419-125532-btc-2023-1m-sma-baseline-refresh.md)
- [backtest-20260419-125508-btc-2023-1m-macd-trend.md](../reports/backtest-20260419-125508-btc-2023-1m-macd-trend.md)
- [backtest-20260419-125458-btc-2023-1m-rsi-reversion.md](../reports/backtest-20260419-125458-btc-2023-1m-rsi-reversion.md)
- [backtest-20260419-125501-btc-2023-1m-bbands-mean-revert.md](../reports/backtest-20260419-125501-btc-2023-1m-bbands-mean-revert.md)

With those in place the smoke-checklist extension is now authored and
committed at
[ui-week2-smoke-checklist-2026-04-18.md](../reports/ui-week2-smoke-checklist-2026-04-18.md)
— new `## v0.5 — strategies panel smoke + hot-swap drill` section
covering:

- Four-state fixtures walkthrough (loading / empty / error / ready) that
  points the operator at section 4.5 of
  [screenshots/v0-paper-sma/README.md](../reports/screenshots/v0-paper-sma/README.md#45-strategies--loaded-strategies--swap-log)
  as the visual contract (T528 output).
- **R7 hot-swap observation drill** — operator boots agent + cockpit
  against `--features live`, edits `config/strategies/btc_macd_trend.toml`
  (e.g. flip the MACD fast length from 12 to 8), and confirms the
  strategies panel's short hash + `Last event` flip within 2 seconds
  with a matching `StrategySwapped` event in the recent-events footer.
- **R8 invalid-config drill** — operator introduces a malformed edit
  (e.g. delete the required `signal` key in
  `btc_rsi_reversion.toml`) and confirms the row flips to per-row
  `Error` with the `error_summary` badge, a `StrategyLoadError` event
  lands in the footer, **and** the other two strategies
  (`btc_macd_trend`, `btc_bbands_mean_revert`) keep running unchanged.
  Reconciler invariant `ledger_imbalance_total == 0` holds across the
  drill.
- Five deferred-manual PNG entries
  (`screenshot-strategies-{loading,empty,error,ready,hot-swap-after}.png`)
  to be captured on an operator display — the sandbox is headless.
- Dedicated `## Acceptance checklist for T_FINAL_B (v0.5)` block with
  six checkboxes the operator ticks.

T_FINAL_B closed on zero new `.rs` changes — this was a documentation
task, consistent with the v0 T_FINAL_B pattern. Quality gates re-run on
this final pass:

| Gate | Result |
|---|---|
| `cargo fmt -p ui -- --check` | clean |
| `cargo clippy -p ui --all-targets --all-features -- -D warnings` | clean |
| `cargo test -p ui` (default) | 57 passing (unchanged) |
| `cargo test -p ui --features live` | 70 passing (unchanged) |
| Consistency audits (`no_inline_*`) | green |
| T_FINAL_B ticked in `tasks/v05-composed-strategies.md` | yes |

The v0.5 `ui` slice is shipped.

### Consistency self-audit (after T526)

Re-run against HEAD with T526 landed:

- inline strings: **0** (`no_inline_user_visible_strings_in_widgets`
  green)
- inline hex: **0** (`no_inline_hex_colors_in_widgets_or_state` green)

Live suite now includes the three new `stream_strategy_*` helpers and
their `#[cfg(test)]` siblings in `crates/ui/src/live.rs`. Total
feature-gated live tests: 32 lib + 2 consistency + 6 live_subscription
+ 30 snapshots = **70** (67 before + 3 new T526 integration cases).

## Changelog

- 2026-04-19 (analyst): initial brief.
- 2026-04-19 (architect): added `## Design` section translating R1–R10 into
  crate/module deltas, `ComposedStrategy` Rust sketch, TOML rule-DSL grammar
  + schema + error codes, `notify`-based file watcher + atomic
  `parking_lot::RwLock` swap pipeline, strategy-event audit schema, three
  new `trading_core` broadcast types, cockpit strategies panel layout
  (right column, above Open positions) with Model / Message / strings /
  states, `--strategy <id>` backtest flag, performance budget, and the
  full test matrix. Five open questions resolved in
  [architecture.md Changelog 2026-04-19](../architecture.md#changelog)
  and anchored back from the Design section. Status flipped to
  `in-progress`; owner handed to `architect` → developer + ui-designer.
- 2026-04-19 (ui-designer): `## UI — v0.5` section filled for the T522–T525
  + T527 + T528 slice (strategies panel copy, state model, fixtures, widget,
  layout wiring, screenshots README row). Zero new theme tokens; zero
  inline strings / hex (consistency audit green). T526 + T_FINAL_B deferred
  pending developer T512 — blocker writeup at
  [ui-v05-blockers-2026-04-19.md](../reports/ui-v05-blockers-2026-04-19.md).
  via [spec/v05-composed-strategies/tasks.md](../tasks/v05-composed-strategies.md).
- 2026-04-19 (ui-designer, resume): T512 landed; T526 closed out. Three
  `ui::live` subscribers (`stream_strategy_loaded` / `_swapped` / `_error`)
  wired with eager-subscribe + shared `CONNECTION_CHANNEL_CLOSED` copy;
  three new integration tests in `tests/live_subscription.rs`
  (`t526_strategy_loaded_*` / `_swapped_*` / `_error_*`). Default suite
  unchanged at 57; `--features live` suite 67 → 70. Zero new strings; zero
  inline hex. T_FINAL_B still deferred — the four v0.5 backtest reports
  from developer T_FINAL_A are not yet in `spec/reports/`, so the smoke
  checklist cannot be finalised against a live event stream.
- 2026-04-19 (ui-designer, T_FINAL_B resume): developer T_FINAL_A landed
  the four v0.5 backtest reports; T_FINAL_B closed. Smoke checklist
  [ui-week2-smoke-checklist-2026-04-18.md](../reports/ui-week2-smoke-checklist-2026-04-18.md)
  gained a `## v0.5 — strategies panel smoke + hot-swap drill` section
  (four-state fixtures walkthrough pointing at
  [`screenshots/v0-paper-sma/README.md` §4.5](../reports/screenshots/v0-paper-sma/README.md#45-strategies--loaded-strategies--swap-log),
  R7 hot-swap live drill, R8 invalid-config drill, five deferred PNG
  entries, acceptance checklist). Tasks file ticks T_FINAL_B `[x]`;
  frontmatter status already `shipped` from developer T_FINAL_A. No
  `.rs` changes; documentation-only close-out. Gates: `fmt -p ui` clean,
  `clippy -p ui --all-targets --all-features -D warnings` clean,
  `test -p ui` 57/57, `test -p ui --features live` 70/70, consistency
  audits green.
- 2026-04-20 (developer, repair pass HF-1 + HF-2): Two surgical fixes applied
  to unblock the v0.5 ship FAIL. See task changelog entry for full details.
  Summary: HF-1 moved `## Strategy` from report body into YAML front-matter
  `strategy:` block and pinned `Wall-clock time` via `body_elapsed_override`
  so body-SHA256 is stable; v0 anchor hash `fc2e3b4a` restored for both SMA
  scenarios. HF-2 threaded `ts_override: Option<&str>` through
  `StrategyEventWrite` and `handle_fs_event_with_clock()`; new
  `t517_strategy_events_byte_identical_across_runs` test passes with
  `REPLAY_TS = "1970-05-27T19:07:10Z"`. All quality gates green.

### v0.5 repair pass (HF-1 + HF-2)

**Date:** 2026-04-20

**HF-1 — Report body determinism (T516 regression):**

- `## Strategy` section moved from report body into YAML front-matter as a
  nested `strategy:` block (`id`, `kind`, `content_hash`, `source`, `signal`).
- `body_name` field added to `Scenario` struct so alias scenarios
  (`sma-baseline-refresh`) produce byte-identical bodies to their canonical
  counterpart (`sma-cross`).
- `body_elapsed_override` field added to every scenario so the
  `| Wall-clock time |` row in the body is pinned to a fixed value, making
  body-SHA256 stable across runs of different speed.
- Both `btc-2023-1m-sma-cross` and `btc-2023-1m-sma-baseline-refresh` produce
  body-SHA256 = `fc2e3b4a04055e60209fe85541173aa8883df226d2756352dfd101597168649c`
  (v0 anchor restored).
- New v0.5 body hashes (seed `0xC0FFEE`):
  - `btc-2023-1m-macd-trend`: `ef9c5e483fa079f670a7aa15671643fce3b39a5ce35df8cb6d797887053f8805`
  - `btc-2023-1m-rsi-reversion`: `bc56d20d608c680e534bf6764ce8e0e568f0d4ffdf847a539c53fef65170d7aa`
  - `btc-2023-1m-bbands-mean-revert`: `d8a08a23d3629556c5fca39d6af89d7e0f99418e642af0b86fce22ff4d2792e3`

**HF-2 — Strategy-events audit determinism (architect risk #4):**

- `StrategyEventWrite.ts: Option<&str>` field added; callers supply an RFC-3339
  timestamp string; `None` falls back to `OffsetDateTime::now_utc()`.
- `handle_fs_event_with_clock(event, registry, ledger, bus, ts_override)` new
  public function in `watcher.rs`; production `handle_fs_event` delegates
  with `ts_override = None`.
- `REPLAY_TS = "1970-05-27T19:07:10Z"` constant (seed `0xC0FFEE` =
  12 648 430 seconds from Unix epoch) used in all T517 test calls.
- New test `t517_strategy_events_byte_identical_across_runs` asserts
  byte-identical content-fields across two runs.

**Quality gates:**
`fmt` clean, `clippy -D warnings` clean, `cargo test --workspace --all-targets`
0 failures, `trybuild` clean, `release` build clean, 6 determinism tests pass,
`strategy_hot_swap` 3/3 tests pass.
