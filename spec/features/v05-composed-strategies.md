---
slug: v05-composed-strategies
status: draft
owner: analyst
updated: 2026-04-19
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
  `spec/reports/backtest-<stamp>-btc-2023-1m-sma-cross.md`.

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

## Implementation

_developer fills this during build._

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
  under `spec/reports/backtest-<stamp>-<scenario-slug>.md` conforming
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

## Changelog

- 2026-04-19 (analyst): initial brief.
