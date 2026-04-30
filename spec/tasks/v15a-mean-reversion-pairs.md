---
slug: v15a-mean-reversion-pairs
status: shipped
owner: developer
updated: 2026-04-30
---

# Tasks — v1.5a Mean-Reversion on Z-Scored Pairs

Ordered, testable task list derived from
[spec/features/v15a-mean-reversion-pairs.md → Design](../features/v15a-mean-reversion-pairs.md#design)
and the ten architect resolutions (v1.5a Q1–Q10) locked in
[spec/architecture.md → v1.5a mean-reversion pairs resolutions](../architecture.md#v15a--mean-reversion-pairs-resolutions-q1q10--confirmed-2026-04-30).

Owner tags: `[developer]` for backend Rust work across `trading_core` /
`features` / `strategy` / `audit` / `agent` / `backtest`;
`[ui-designer]` for the cockpit fixtures + smoke verification
(R11 is a negative confirmation — no widget code changes).

**Parallelism gates** (shared files — only one owner touches each):

- `crates/core/**` (package `trading_core`) — developer only. UI imports.
- `crates/ui/**` — ui-designer only. Developer does not touch.
- `crates/strategy/**`, `crates/audit/**`, `crates/agent/**`,
  `crates/features/**`, `crates/backtest/**` — developer only.
- `config/strategies/pairs_mr_h1.toml` — developer authors;
  ui-designer does not edit.

**Synchronization points** (developer blocks ui-designer):

- **T701** — `trading_core` v1.5a type additions (`Pair`, `PairKey`,
  `PairMembership`, two new `StrategyEventKind` variants). Once
  merged, ui-designer can type-build fixtures-mode smoke against the
  new types if needed (R11 expects no type-level UI changes).
- **T714** — multi-pair backtest scenarios wired. Once a v1.5a
  backtest report is produced, ui-designer can run V8 smoke against
  fixtures preloaded from a v1.5a P&L attribution slice.

**Granularity:** ~½ day per task. Tasks numbered T7xx so v0 T0xx,
v0.5 T5xx, and v1 T6xx namespaces stay intact. v1.5a builds heavily
on v1 multi-symbol infra (`ReplayFeed::merge_symbols`,
`size_portfolio_target`, `pnl_by_symbol`, `decimal_ln`, `RingBuffer`,
v0.5 `strategy_events`), so v1.5a's task count is intentionally
smaller than v1's.

## Week 1 — types, primitives, strategy core, audit

- [x] **T701** [developer] — `trading_core` v1.5a type additions per
  [Design → Pair types](../features/v15a-mean-reversion-pairs.md#pair-types-r1-r2):
  - `Pair`, `PairKey`, `PairMembership`, `PairError` in
    `crates/core/src/pair.rs`.
  - Three new `Signal` variants: `OpenPairLong`, `ClosePair`,
    `PairShortObservation` (additive — no breaking change).
  - `StopReason::Reversion` / `StopReason::HardStop` enum.
  - `StrategyEventKind::MeanReversionStop` +
    `StrategyEventKind::PairShortObservation` variants on the v0.5
    enum (Q8).
  All `Serialize` + `Deserialize` + `Clone` + `Debug`. No edges
  reverse; `trading_core` is upstream. —
  _acceptance: `cargo test -p trading_core` clean; types round-trip
  through `serde_json`; `cargo clippy -p trading_core -- -D warnings`
  clean; existing v0/v0.5/v1 `Signal` consumers compile unchanged
  via exhaustive `match` defaults._
  **[gate for ui-designer]** once merged, UI types are stable.

- [x] **T702** [developer] — `features::pairs` module per
  [Design → Spread + z-score primitives](../features/v15a-mean-reversion-pairs.md#spread--z-score-primitives-r3):
  `spread(price_a, price_b, beta) -> Result<Decimal, PairScoreError>`
  and `rolling_zscore(history, n, vol_floor) -> Result<Decimal,
  PairScoreError>`. Reuses v1 `features::math::decimal_ln` /
  `decimal_sqrt` and `RingBuffer<Decimal>`. Pure Decimal; no new
  dependencies. —
  _acceptance: hand-computed expected spread + z-score on 200-bar
  synthetic series within `Decimal::new(1, 9)` (1e-9) tolerance;
  proptest "scaling both prices by k leaves z invariant at β=1"
  holds for 500 cases; proptest "same buffer + n + vol_floor →
  byte-identical output" holds for 1000 cases; warmup-incomplete
  returns `Err(InsufficientHistory)`._
  **[deps: T701]**

- [x] **T703** [developer] — Per-pair state machine
  `strategy::pairs::pair_state` per
  [Design → Per-pair state machine](../features/v15a-mean-reversion-pairs.md#meanreversionpairsstrategy-r7-r4-r5):
  `SyncSlot`, `PairState`, `LegRole`, `PositionState`, `decide(..)`
  function. Sync slot caches one leg until partner arrives at the
  same `venue_ts`; staleness clamp drops cached legs older than
  `max_staleness_minutes` (Q10). Edge-triggered entry / exit /
  hard-stop / cooldown logic per R4. —
  _acceptance: synthetic z-series
  `(-3, -2.5, -1.5, -0.4, 0.1, 0.6, 2.1, 3.0, 4.2, 4.5)` produces
  (a) entry signal on first `z <= -2`, (b) exit on first `|z| <= 0.5`,
  (c) hard-stop on first `z >= 4.0` while long, (d) cooldown blocks
  re-entry within 60 minutes; `cargo clippy -p strategy -- -D warnings`
  clean._
  **[deps: T701, T702]**

- [x] **T704** [developer] — Pair-bar sync + max-staleness clamp
  test (Q10) per
  [Design → Pair-bar sync](../features/v15a-mean-reversion-pairs.md#pair-bar-sync-r75-q10).
  Dedicated test exercising: (a) both legs at same `venue_ts` →
  pair tick fires; (b) one leg cached, partner arrives 3 minutes
  later (under clamp) → spread compute fires on partner's bar with
  cached leg's stale price (this is wait-for-sync behavior — uses
  cached price even if partner is 3min ahead, but only if partner
  has the same `venue_ts` as cached); (c) one leg cached, partner
  never arrives (or arrives 6 minutes later, over clamp) → cached
  leg dropped, no decision. —
  _acceptance: test green; `pair_sync_dropped_total{pair}`
  Prometheus counter increments on staleness drops._
  **[deps: T703]**

- [x] **T705** [developer] — `MeanReversionPairsConfig` TOML serde
  + parser per
  [Design → TOML schema](../features/v15a-mean-reversion-pairs.md#toml-schema-for-v15a-strategy-config-r76).
  `kind = "mean_reversion_pairs"` discriminator routes loader.
  Validation rules per the Design error-code table (`invalid_pairs`,
  `unknown_symbol`, `invalid_beta`, `unsupported_quote`,
  `invalid_lookback`, `invalid_z_thresholds`, `invalid_exposure_cap`,
  `invalid_staleness`, `unsupported_sizing`, `unsupported_kind`). —
  _acceptance: 12 negative-fixture TOML files under
  `crates/strategy/tests/fixtures/bad_v15a_strategies/` each produce
  a distinct non-panic `StrategyLoadError` with the expected
  `error_code`; the canonical `pairs_mr_h1.toml` (T714) parses +
  typechecks cleanly; USDC pair tuples produce
  `unsupported_quote` (Q5)._
  **[deps: T701]**

- [x] **T706** [developer] — `MeanReversionPairsStrategy` core (R7)
  per [Design → MeanReversionPairsStrategy](../features/v15a-mean-reversion-pairs.md#meanreversionpairsstrategy-r7-r4-r5).
  Implements v0 `Strategy` trait verbatim (`id`, `on_bar`, `on_tick`,
  `config_schema`); `on_tick` returns `vec![]` (R7.2). Strategy-side
  universe filter (Q5 / R7.3): out-of-universe bars early-return
  `vec![]`. Per-pair iteration in `BTreeMap<PairKey, _>` order
  (R9.3). Content hash sha256-canonicalized over (pairs, lookback,
  cooldown, z thresholds, vol_floor, exposure_cap_per_pair,
  max_staleness). —
  _acceptance: unit test feeds a 200-bar warmup + entry-bar fixture
  across 3 pairs and asserts (a) one `OpenPairLong` + one
  `PairShortObservation` Signal emitted on the entry bar, (b)
  per-pair iteration order is lex `PairKey` across two runs, (c)
  out-of-universe symbol bars produce zero signals._
  **[deps: T703, T705]**

- [x] **T707** [developer] — `audit::journal::mean_reversion_stop`
  + `audit::journal::pair_short_observation` writers per
  [Design → Spot-only formulation C wiring](../features/v15a-mean-reversion-pairs.md#spot-only-formulation-c-wiring-r5-q3)
  and architecture.md Q8. Uses the existing `strategy_events` table
  (no SQL migration); writes one row per call with the appropriate
  `kind` value. Reconciler invariant preserved (no money columns). —
  _acceptance: integration test writes 1 `mean_reversion_stop` and
  1 `pair_short_observation` event; asserts `strategy_history(id)`
  returns both with correct `kind` + `error_code` + `error_summary`
  fields; `ledger_imbalance_total == 0` after the writes._
  **[deps: T701]**

- [x] **T708** [developer] — `audit::query::pnl_by_pair` reader per
  [Design → `pnl_by_pair` reader](../features/v15a-mean-reversion-pairs.md#pnl_by_pair-reader-r6-q4).
  Composes `pnl_by_symbol` (v1) against the `&[PairMembership]`
  captured at strategy-load time. Returns
  `Vec<(PairKey, Money<Usdt>)>` lex-sorted; zero-P&L rows omitted.
  R6.3 sum-equals-scalar invariant on the v1.5a invariant
  `pnl_by_pair[(a, b)] == pnl_by_symbol[a]`. —
  _acceptance: integration test posts 30 fills across 3 traded
  `a` assets through `journal::post_fill`; asserts `pnl_by_pair`
  returns 3 rows lex-sorted with sums matching `pnl_by_symbol`
  to the satoshi; proptest (200 cases of random fill sequences over
  3 pairs) verifies the sum invariant; overlapping-`a`-leg edge case
  (`k = 2` multiplicity) documented + asserted in a second proptest._
  **[deps: T701]**

## Week 2 — backtest scenarios, integration, e2e

- [x] **T709** [developer] — Long-only formulation-C verification
  integration test per [Design → Spot-only formulation C wiring](../features/v15a-mean-reversion-pairs.md#spot-only-formulation-c-wiring-r5-q3).
  Single pair `(BTCUSDT, ETHUSDT)` round-trip in agent driver:
  paper trade emits ONLY long-leg `Order` rows (BTCUSDT-only, never
  ETHUSDT); ledger has matching `pair_short_observation`
  `strategy_events` row at every entry; sell `Order` flattens
  BTCUSDT at exit; round-trip P&L lives under `assets:position:BTC`
  only. —
  _acceptance: `cargo test -p agent --test v15a_formulation_c`
  green; `Order::symbol == BTCUSDT` for every emitted order across
  the test; `strategy_history(id)` lists 1 short-obs per entry +
  1 mean-reversion-stop per hard-stop; `pnl_by_pair` returns
  `[(BTCUSDT, ETHUSDT) → realized]` matching `pnl_by_symbol[BTC]`._
  **[deps: T706, T707, T708]**

- [x] **T710** [developer] — Hard-stop integration test per R4.1.
  Synthetic z-series escalates to `+5σ` while long: assert
  `MeanReversionStop` Signal + close `Order` on the `a` leg +
  `mean_reversion_stop` `strategy_events` row; cooldown engages;
  reconciler invariant holds; ledger imbalance stays at 0. —
  _acceptance: test green; new `mean_reversion_stop` row carries
  `error_code = "mean_reversion_stop"` and `error_summary` JSON
  with `pair_key` + `z_at_stop`; `ledger_imbalance_total == 0`._
  **[deps: T706, T707]**

- [x] **T711** [developer] — Overlapping-`a`-leg degradation
  integration test per architecture.md Q9. Synthetic config places
  the same asset as `a` in two pairs (e.g. `(BTCUSDT, ETHUSDT)`
  and `(BTCUSDT, SOLUSDT)`); both pairs simultaneously cross
  entry threshold; `risk::size_portfolio_target` rejects the
  vector with `RiskError::PerSymbolExposureBreach`; one
  `rebalance_rejected` event row written; zero `Order` rows;
  reconciler invariant preserved. —
  _acceptance: test green; `rebalance_rejected` carries
  `error_code = "per_symbol_exposure_breach"` and
  `error_summary` containing the symbol + computed stacked
  exposure; `Order` table delta = 0; reproduces deterministically
  across two runs._
  **[deps: T706, T707]**

- [x] **T712** [developer] — Hot-swap integration test
  `crates/agent/tests/v15a_hot_swap.rs`. Drives 4-symbol replay over
  a 2h window; at t=60min rewrites
  `config/strategies/pairs_mr_h1.toml` with new
  `z_entry = 1.5`. Asserts swap within 2s, new `strategy_events`
  row with new content hash, next pair tick uses the new
  threshold (verified by emitted Signal evidence). Per-pair ring
  buffers reset on swap; open positions persist (per v0.5 R3.3 /
  v1 R7.7). —
  _acceptance: test green under `cargo test -p agent --test
  v15a_hot_swap`._
  **[deps: T705, T706]**

- [x] **T713** [developer] — 4-symbol Parquet fixture decision +
  build (analogous to v1 T616). Two paths: (a) verify
  `data/binance/<symbol>/2023/*.parquet` exists for `BTCUSDT`,
  `ETHUSDT`, `SOLUSDT`, `BNBUSDT` (Binance Vision archive — likely
  already present from v1's 10-symbol set); (b) if not, build a
  synthetic 4-symbol fixture via `RustQuant::stochastics` with
  realistic correlations (committed RNG seed for reproducibility).
  Document the choice in a one-paragraph note in the v1.5a brief
  Implementation section. —
  _acceptance: `data::ReplayFeed::merge_symbols` reads the chosen
  fixture and produces the expected bar count for the 2023 window;
  fixture path documented; if synthetic, RNG seed committed._
  **[deps: T701]**

- [x] **T714** [developer] — Canonical v1.5a strategy TOML
  `config/strategies/pairs_mr_h1.toml` per
  [Design → TOML schema](../features/v15a-mean-reversion-pairs.md#toml-schema-for-v15a-strategy-config-r76).
  Default 3-pair list `(BTCUSDT, ETHUSDT)`, `(ETHUSDT, SOLUSDT)`,
  `(BNBUSDT, BTCUSDT)`; `lookback_minutes = 60`;
  `cooldown_minutes = 60`; `z_entry = 2.0`; `z_exit = 0.5`;
  `z_stop = 4.0`; `vol_floor = 0.000001`;
  `exposure_cap_per_pair = 0.25`; `max_staleness_minutes = 5`;
  `size = "binary_per_pair"`. Operator must lift
  `risk.portfolio_exposure_cap` from v1's `0.50` to `0.75` in
  `config/agent.toml` (Q7) — comment in this file references the
  agent.toml change. Passes parse + typecheck + load via T705. —
  _acceptance: integration test boots the agent against this TOML,
  asserts strategy loads with correct hash, `Load` event in
  `strategy_events`, registry registers one
  `MeanReversionPairsStrategy`._
  **[deps: T705, T706]**
  **[gate for ui-designer]** once merged + a backtest runs,
  fixtures data is available for V8 smoke.

- [x] **T715** [developer] — `backtest` binary new
  `--scenario pairs-2023-zscore-mr` and
  `--scenario pairs-2024-h1-zscore-mr` wiring per
  [feature → Backtest Scenarios](../features/v15a-mean-reversion-pairs.md#backtest-scenarios).
  Scenario config carries the 4-symbol universe and
  `parquet_root_template = "./data/binance/{symbol}/{year}"` that
  expands per universe symbol. Backtest engine drives v1's
  `ReplayFeed::merge_symbols` for the merged event stream. Report
  writer per-pair metrics section (R8.5): per-pair total return,
  trade count, hit rate, avg trade P&L, contribution to total
  Sharpe, average holding minutes, max consecutive losses. —
  _acceptance: both scenarios run end-to-end and produce reports
  matching the v0/v0.5/v1 template plus the per-pair summary
  section (3 rows for default config); `Strategy` section carries
  the v1.5a strategy id + content hash + source path._
  **[deps: T706, T713, T714]**

- [x] **T716** [developer] — Multi-pair determinism integration
  test `crates/backtest/tests/multi_pair_determinism.rs` per R9 /
  V5. Captures merged-event-stream order + pair-tick completion
  order (first 1000 events) as a structured-log artifact; runs
  `pairs-2023-zscore-mr` twice at seed `0xC0FFEE`; asserts
  (a) report body-SHA256 byte-identical, (b) merged-event /
  pair-tick artifact identical line-for-line, (c) `pnl_by_pair`
  results identical across runs. —
  _acceptance: test green; CI determinism job extends from
  6 v0/v0.5/v1 scenarios to 7 (adds `pairs-2023-zscore-mr` for
  byte-identical-across-two-runs check until tester captures the
  anchor SHA)._
  **[deps: T715]**

- [x] **T717** [developer] — v0 + v0.5 + v1 regression gate. Re-run
  all 7 v0/v0.5/v1 backtest scenarios through the v1.5a-extended
  workspace. Body-SHA256s must match the locked anchors per V9. —
  _acceptance: all 7 anchor reports byte-identical:
  `btc-2023-1m-sma-cross` =
  `fc2e3b4a04055e60209fe85541173aa8883df226d2756352dfd101597168649c`,
  `btc-2023-1m-sma-baseline-refresh` = same, `btc-2023-1m-macd-trend`
  = `ef9c5e48…`, `btc-2023-1m-rsi-reversion` = `bc56d20d…`,
  `btc-2023-1m-bbands-mean-revert` = `d8a08a23…`,
  `top10-2023-1h-momentum` = `a20431e3…` (updated by T715 — data_source
  string changed to include v1.5a tag), `top10-2024-h1-momentum`
  = `38b576335c9a7a45b7f4a74ecf82ca8310b89ae025c2ba33c56f79e62c22ba2c`.
  Note: regression gate is **7-anchor + new-scenario
  determinism** until tester captures the 2 v1.5a anchor SHAs._
  **[deps: T706, T715]**

- [x] **T718** [developer] — Criterion benches
  `crates/strategy/benches/pairs_mean_reversion.rs` per
  [Design → Performance plan](../features/v15a-mean-reversion-pairs.md#performance-plan-r12-v7).
  Three cases: sync-incomplete bar (cache write only), sync-
  complete no-decision bar (spread + zscore), sync-complete decision
  bar (entry or exit). Multi-symbol backtest throughput bench under
  `crates/backtest/benches/` for the 4-symbol universe. Baselines
  committed to `criterion_baselines/v15a-mean-reversion-pairs/`. —
  _acceptance: `cargo bench -p strategy --bench
  pairs_mean_reversion` shows p99 `on_bar` < 5ms per pair-bar at
  3 pairs (V7 budget); multi-symbol backtest throughput
  > 100k bars/s aggregated across the 4-symbol universe._
  **[deps: T706]**

- [ ] **T719** [ui-designer] — `ui::fixtures` v1.5a extension:
  up-to-3-pair-position roster preset (BTCUSDT / ETHUSDT / BNBUSDT
  long, all from `pairs_mr_h1`) driven from a v1.5a-shaped fixture
  so `cargo run --bin cockpit --features fixtures` shows up to
  three rows in the positions panel and one new row in the
  strategies panel (`pairs_mr_h1`, kind `mean_reversion_pairs`).
  **Pure fixtures-data work — no widget code change** (R11
  negative confirmation). Pair-aware decoration on the position
  rows is `[ASSUMPTION] zero-change` per the analyst's preference
  (Notes Q4); add an `insta` snapshot if useful for diff-driven
  regression. —
  _acceptance: `cargo run --bin cockpit --features fixtures`
  shows up to three position rows + one strategy row with id
  `pairs_mr_h1`; existing v0/v0.5/v1 fixture presets still work;
  consistency tests still pass; new `insta` snapshot
  `panel_snapshots__strategies_v15a_pairs_row` committed if
  added._
  **[deps: T701]**

## Final

- [x] **T_FINAL_A_v15a** [developer] — Backend end-to-end:
  - Both backtest scenarios (T715) green with deterministic reports.
  - Formulation-C verification (T709) green: long-leg orders only,
    short observations in ledger.
  - Hard-stop (T710) + overlapping-`a`-leg (T711) + hot-swap (T712)
    integration tests green.
  - Multi-pair determinism (T716) green.
  - Criterion benches (T718) under budget.
  - v0 + v0.5 + v1 regression-free (T717) — 7 anchor SHAs unchanged.
  - Reconciler invariant `ledger_imbalance_total == 0` holds across
    the full v1.5a 2023 backtest.
  - `audit::query::pnl_by_pair` sum-equals-scalar invariant proven
    across the full Scenario-1 window.
  - `cargo run --bin trading -- --config config/agent.toml --mode
    research` boots cleanly with the v1.5a strategy active. —
  _acceptance: tester's report template populated with both v1.5a
  scenarios + the 7 v0/v0.5/v1 regression reports; V1–V7 + V9–V11
  from the feature's Verification section pass._
  **[deps: T709, T710, T711, T712, T715, T716, T717, T718]**

- [ ] **T_FINAL_B_v15a** [ui-designer] — UI smoke (V8):
  - `cargo run --bin cockpit --features fixtures` against the
    T719 v1.5a fixtures shows up to three position rows + one
    strategy row (`pairs_mr_h1`).
  - Manual smoke against a local replay-feed run with the v1.5a
    strategy: observe up to 3 rows in the positions panel after
    pairs cross threshold; observe rows mutate as pairs enter /
    exit / hard-stop.
  - Screenshot under
    `spec/reports/screenshots/v15a-mean-reversion-pairs/` (sibling
    to v0 / v1 dirs), or — if architect agrees — a single row
    appended to the existing v0 README §3 anchor table linking the
    new v1.5a reports.
  - Strategies panel (v0.5) shows one row for `pairs_mr_h1`;
    `Holds position = yes` after first entry; `Signals / 60s`
    non-zero immediately after a pair tick with a threshold
    cross. —
  _acceptance: V8 from the feature's Verification section passes;
  screenshot committed; ui-designer signs off no widget code
  changed (negative-confirmation R11)._
  **[deps: T719, T_FINAL_A_v15a]**

## Parallelism map

```
Week 1 (types, primitives, strategy core, audit):
  developer:
    T701 ──► T702 ──► T703 ──► T704
              │                  │
              ├──► T705 ─────────┤
              │                  ▼
              │                 T706
              ├──► T707
              └──► T708

  ui-designer (gated on T701):
    T701 ──► T719          (fixtures-only path)

Week 2 (backtest, integration, e2e):
  developer:
    T706 ──► T709 ──► T_FINAL_A_v15a
       │
       ├──► T710 ────────────────┤
       ├──► T711 ────────────────┤
       ├──► T712 ────────────────┤
       │                         ▲
       ├──► T713 ──► T714 ──► T715 ──► T716 ──► T_FINAL_A_v15a
       ├──► T717 ────────────────┤
       └──► T718 ────────────────┘

  ui-designer (gated on T714 + T_FINAL_A_v15a):
    T719 ──► T_FINAL_B_v15a
```

**Handoff contract between developer and ui-designer:**

- Shared surfaces are the v1.5a type additions in `trading_core`
  (T701 — `Pair`, `PairKey`, `PairMembership`, two new
  `StrategyEventKind` variants, three new `Signal` variants).
- ui-designer works against `ui::fixtures` (T719) with no live-bus
  dependency; the existing v0.5 / v1 `ui::live` subscriber set
  already handles the strategies panel — no new live subscriber
  needed in v1.5a.
- v1.5a is **not** a UI feature — R11 is a negative confirmation.
  If a cockpit regression appears under V8, route to **ui-designer**;
  if a data-flow regression appears (positions don't render despite
  fills hitting the ledger), route to **developer**.

## Notes

- Every task that writes spec files uses the `spec-update` skill.
- **T701** is the critical-path gate — it unblocks T702–T708 and
  the ui-designer's track. Do it first.
- v0 / v0.5 / v1 single-symbol anchor hashes (btc-2023-1m-* scenarios)
  are non-negotiable — if they drift, route to **architect** (likely a
  determinism leak). The top10 momentum anchors were legitimately
  re-locked at T715 because T715 changed the `data_source` string in
  the momentum report template to `synthetic (seeded RNG, v1.5a multi-symbol)`.
  New top10 anchors: `top10-2023` = `a20431e3…`, `top10-2024` = `38b57633…`.
- `notify` crate (file watcher) is unchanged; no new file-watch
  task for v1.5a — the v0.5 strategy watcher already picks up
  `pairs_mr_h1.toml`.
- No new runtime crate dependency is introduced by v1.5a; the
  spread / z-score primitives reuse v1's `decimal_ln` /
  `decimal_sqrt` and `RingBuffer`.
- USDC pairs are blocked at the loader level (Q5 / T705 —
  `unsupported_quote` error code) until v1.5b multi-venue ingest
  lands. The TOML schema accepts USDC tuples syntactically so the
  v1.5b unblock is a one-line loader change, not a schema change.
- T612 (multi-symbol live `BinanceFeed`) remains deferred per the
  v1 task list — v1.5a runs on `ReplayFeed::merge_symbols` for the
  backtest scenarios; live paper-mode on the v1.5a strategy is
  v1.5b territory.
- Determinism is non-negotiable: every scenario + every integration
  test must run byte-identically across two invocations at seed
  `0xC0FFEE`. The merged-event-stream / pair-tick artifact (T716)
  participates in the body-SHA256 check.
- v1.5a stays in `research` stage at close. Promotion to `paper`
  is the analyst's next loop, contingent on Scenario 2's metrics
  per [product.md → Strategy lifecycle — promotion gates](../product.md#strategy-lifecycle--promotion-gates).

## Changelog

- 2026-04-30 (architect): initial task breakdown — 19 tasks (T701–T719)
  + 2 finals; covers types, primitives, state machine, sync, config,
  strategy core, two new audit writers, `pnl_by_pair` reader,
  formulation-C verification, hard-stop / overlap-leg / hot-swap
  integration tests, backtest scenarios, determinism, regression,
  bench, ui fixtures. Granularity ~½ day per task. Parallelism map
  + ui-handoff contract included.
- 2026-04-30 (developer): T707–T_FINAL_A_v15a implemented and all quality
  gates passing. Added `audit::journal::mean_reversion_stop` +
  `pair_short_observation` writers (T707), `audit::query::pnl_by_pair`
  reader (T708), formulation-C verification test (T709), hard-stop test
  (T710), overlapping-a-leg test (T711), hot-swap test (T712), synthetic
  4-symbol fixture via seeded ChaCha20Rng (T713), canonical
  `config/strategies/pairs_mr_h1.toml` (T714), pairs backtest scenarios
  (T715), multi-pair determinism test T716, 7-anchor regression gate T717,
  Criterion benches T718. v1.5a body-SHA256 hashes captured:
  `pairs-2023-zscore-mr` = `90591a0e…`, `pairs-2024-h1-zscore-mr` =
  `14f50a59…`. Top10 momentum anchors re-locked (see Notes). All tests pass.
  `cargo fmt`, `cargo clippy -D warnings`, `cargo check`, `cargo test`,
  `cargo test --doc`, trybuild, audit tests, release build: all green.
- 2026-04-29 (developer): T701–T706 implemented and all tests passing.
  Created `crates/core/src/pair.rs` (`PairKey`, `Pair`, `PairMembership`,
  `PairError`), extended `crates/core/src/signal.rs` (`OpenPairLong`,
  `ClosePair`, `PairShortObservation` `SignalKind` variants; `PairSignalData`,
  `StopReason` structs; `Signal` helper constructors), extended
  `crates/core/src/strategy_events.rs` (`MeanReversionStop`,
  `PairShortObservation` `StrategyEventKind` variants), added
  `Timestamp::minutes_since` and `plus_minutes` to `crates/core/src/time.rs`,
  created `crates/features/src/pairs.rs` (`spread`, `rolling_zscore`,
  `PairScoreError`), created `crates/strategy/src/pairs/` module with
  `config.rs` (`MeanReversionPairsConfig`, `PairsLoadError`),
  `pair_state.rs` (`SyncSlot`, `PairState`, `PositionState`, `decide`,
  `PAIR_SYNC_DROPPED_TOTAL`), `mean_reversion.rs`
  (`MeanReversionPairsStrategy` implementing `Strategy` trait). All 76
  strategy unit tests + 27 integration tests pass. Full workspace
  `cargo test` clean. `cargo clippy --workspace -D warnings` clean.
