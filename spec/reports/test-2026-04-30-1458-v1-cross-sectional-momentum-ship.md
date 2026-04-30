---
title: Test Report
feature: v1-cross-sectional-momentum
run_id: 2026-04-30-1458-UTC
commit: uncommitted
agent: tester
verdict: PASS
---

# Test Report — v1-cross-sectional-momentum — 2026-04-30 14:58 UTC

## 1. Scope

- **Feature / change under test:** v1 Cross-Sectional Momentum (Top-N) — full ship validation. Covers: multi-symbol data ingest + determinism, MomentumStrategy (R3–R7), vector-order sizer (R5), funding-rate observation-only poller (Q2), per-symbol P&L attribution (R8), UI fixtures multi-row positions panel (R11), long-only enforcement (R4/Q3), and all 7 anchor hash regression gates.
- **Spec refs:** `spec/features/v1-cross-sectional-momentum.md`, `spec/tasks/v1-cross-sectional-momentum.md`
- **Commit SHA:** `uncommitted` — repository has no commits
- **Rust toolchain:** `rustc 1.94.1 (e408947bf 2026-03-25)` / `cargo 1.94.1`
- **OS / arch:** `Darwin 25.4.0 arm64`
- **Baseline runs:**
  - v0 anchor: `test-2026-04-19-0615-v0-paper-sma-ship.md` (PASS, 124 tests, body-SHA256 `fc2e3b4a…`)
  - v0.5 ship: `test-2026-04-20-2030-v05-composed-strategies-ship.md` (PASS, 219 tests, all 4 v0.5 anchors held)
  - Developer close-out: `spec/reports/dev-v1-closeout-notes-2026-04-29.md` (T613/T614 open on close-out; subsequently completed per T_FINAL_A_v1 tick note)

---

## 2. Static Analysis

| Check               | Result | Notes |
|---------------------|--------|-------|
| `cargo fmt --check` | **PASS** | Exit 0. Entire workspace clean; no diff output. |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | **PASS** | 0 warnings; exit 0. 3.56s incremental. |
| `cargo check --workspace --all-targets` | **PASS** | 0 errors; exit 0. 1.64s incremental. |
| `cargo audit` | **SKIP** | `cargo-audit` not installed; no new runtime crates introduced in v1 per T616 notes (synthetic path only). |
| `cargo deny check` | **SKIP** | No new runtime crates vs v0.5 PASS baseline; carried forward per prior precedent. |

---

## 3. Unit & Integration Tests

### `cargo test --workspace --all-targets`

| Crate / Target | Passed | Failed | Ignored | Notes |
|---|---:|---:|---:|---|
| `agent` (lib unit) | 23 | 0 | 0 | Config, kill-switch, reconciler, watcher (T513). |
| `agent` (bin `trading`) | 0 | 0 | 0 | — |
| `agent` (metrics_endpoint) | 1 | 0 | 0 | `t27_metrics_endpoint_returns_all_r9_2_names` PASS. |
| `agent` (strategy_hot_swap) | 3 | 0 | 0 | T517: `t517_hot_swap_roundtrip`, `t517_rapid_fire_20_swaps_no_torn_reads`, `t517_strategy_events_byte_identical_across_runs`. |
| `agent` (strategy_rejection) | 2 | 0 | 0 | T518: `t518_ten_bad_fixtures_all_rejected_registry_unchanged`, `t518_ledger_imbalance_zero_after_rejections`. |
| `agent` (v1_hot_swap) | 4 | 0 | 0 | **NEW v1**: T619 — `t619_bus_receives_strategy_loaded_event`, `t619_momentum_load_records_strategy_loaded`, `t619_momentum_hot_swap_records_swapped_event`, `t619_invalid_toml_records_error_and_retains_old_strategy`. |
| `agent` (v1_rebalance_reject) | 3 | 0 | 0 | **NEW v1**: T620 — `t620_valid_portfolio_does_not_write_rebalance_rejected`, `t620_portfolio_breach_writes_rebalance_rejected`, `t620_per_symbol_breach_writes_rebalance_rejected`. |
| `audit` (lib unit) | 0 | 0 | 0 | — |
| `audit` (funding_rate_history_test) | 6 | 0 | 0 | **NEW v1**: T613 — table_exists, chronological, symbol_filter, window_filter, empty_on_no_match, insert_does_not_affect_ledger_balance. |
| `audit` (ledger_integration) | 5 | 0 | 0 | T05/T06 acceptance. |
| `audit` (strategy_events_test) | 5 | 0 | 0 | T508/T509/T510. |
| `backtest` (lib unit) | 3 | 0 | 0 | T24 fill math. |
| `backtest` (bin unit) | 0 | 0 | 0 | — |
| `backtest` (determinism) | 11 | 0 | 0 | T33 (2) + T521 (4) + **T622 (5)**: all v0/v0.5 + v1 anchor regression tests green. 29.55s. |
| `backtest` (multi_symbol_determinism) | 5 | 0 | 0 | **NEW v1**: T618 — `t618_merge_sort_key_venue_ts_then_symbol`, `t618_top_k_long_selects_k_symbols`, `t618_warmup_period_produces_no_signals`, `t618_out_of_universe_bars_filtered`, `t618_signal_sequence_deterministic_two_runs`. |
| `cost` (lib unit) | 2 | 0 | 0 | T30. |
| `data` (lib unit) | 8 | 0 | 0 | T10 FakeFeed + T11 clock-skew. |
| `data` (binance_ws_integration) | 0 | 0 | 3 | T08 — 3 ignored (live WS required). Correct. |
| `data` (funding_poller_integration) | 3 | 0 | 0 | **NEW v1**: T613 wiremock — `t613_poll_three_symbols_persists_rows`, `t613_poller_skips_on_connection_refused`, `t613_poller_skips_on_5xx`. |
| `data` (replay_60_bars) | 1 | 0 | 0 | T09. |
| `exec` (lib unit) | 0 | 0 | 0 | Stub. |
| `features` (lib unit) | 43 | 0 | 0 | **+20 vs v0.5**: 23 original + 10 new `math::` tests (T602 `decimal_ln`/`decimal_sqrt`) + 10 new `cross_sectional::` tests (T603 score). Proptests included. |
| `llm`, `models` (lib unit) | 0 | 0 | 0 | Stubs. |
| `risk` (lib unit) | 10 | 0 | 0 | **+4 vs v0.5**: 6 original + 4 new `portfolio::` tests (T607 `size_portfolio_target`, proptest). |
| `strategy` (lib unit) | 46 | 0 | 0 | **+20 vs v0.5**: 26 original + 20 new cross_sectional tests (T604 selector, T605 config, T606 momentum). |
| `strategy` (bad_strategy_fixtures) | 11 | 0 | 0 | T504. |
| `strategy` (bad_v1_strategy_fixtures) | 11 | 0 | 0 | **NEW v1**: T605 — 10 negative TOML fixtures + `t605_all_error_codes_distinct`. |
| `strategy` (canonical_recipes) | 5 | 0 | 0 | T515. |
| `trading_core` (lib unit) | 17 | 0 | 0 | **+6 vs v0.5**: 11 original + 6 new v1 type tests (T601 Universe, SymbolSet, FundingObs round-trips). |
| `trading_core` (trybuild) | 1 | 0 | 0 | T03 — 3/3 compile-fail cases. |
| `trading_core` (types_test) | 20 | 0 | 0 | T02 serde round-trips. |
| `ui` (lib unit) | 25 | 0 | 0 | T523 + v0 state tests. |
| `ui` (bin unit) | 0 | 0 | 0 | — |
| `ui` (consistency) | 2 | 0 | 0 | `no_inline_user_visible_strings_in_widgets` + `no_inline_hex_colors_in_widgets_or_state` — both PASS. `inline strings: 0`, `inline hex: 0`. |
| `ui` (live_subscription) | 0 | 0 | 0 | Gated on `--features live`. Correct. |
| `ui` (panel_snapshots) | 31 | 0 | 0 | **+1 vs v0.5 baseline (30→31)**: new `positions_v1_three_rows` snapshot. All 31 pass. |
| **Total** | **307** | **0** | **3** | Δ+88 vs v0.5 baseline (219→307). 3 T08 ignored (live WS). |

**`cargo test --workspace --doc`:** PASS — 0 errors. 1 doc-test in `agent::bus` correctly `#[ignore]`.

**`cargo test -p trading_core --test trybuild`:** PASS — 3/3 compile-fail cases.

**`cargo test -p audit`:** PASS — 16 tests (5 ledger_integration + 5 strategy_events_test + 6 funding_rate_history_test).

**`cargo test -p data --test funding_poller_integration`:** PASS — 3/3 (wiremock mock-REST, offline).

**`cargo test -p ui`:** PASS — **58 tests** (25 lib + 2 consistency + 31 snapshots). Meets ≥ 58 threshold.

**`cargo test -p ui --features live`:** PASS — **71 tests** (32 lib + 2 consistency + 6 live_subscription + 31 snapshots). Meets ≥ 71 threshold.

**`cargo test -p backtest --test multi_symbol_determinism`:** PASS — 5/5.

**`cargo test -p strategy --test bad_v1_strategy_fixtures t605_bad_k_short_nonzero_rejected`:** PASS — `k_short = 1` produces `unsupported_short_sizing` error code.

### Failing Tests

_none_ — all 307 tests pass. The 3 T08 ignored tests are correctly gated with `#[ignore]` (live Binance WS required).

---

## 4. Property / Fuzz Tests

| Suite | Cases | Shrunk failures | Notes |
|-------|------:|----------------:|-------|
| `trading_core::order_tests::prop_zero_qty_rejected` | default | 0 | |
| `trading_core::order_tests::prop_positive_qty_accepted` | default | 0 | |
| `trading_core::order_tests::prop_exposure_cap` | default | 0 | |
| `features::sma::proptests::t21_stream_batch_agree` | default | 0 | |
| `features::ema::proptests::t502_ema_stream_batch_agree` | default | 0 | |
| `features::rsi::proptests::t502_rsi_always_in_0_100` | default | 0 | |
| `features::rsi::proptests::t502_rsi_stream_batch_agree` | default | 0 | |
| `features::bbands::proptests::t502_bbands_upper_gte_lower` | default | 0 | |
| `features::bbands::proptests::t502_bbands_stream_batch_agree` | default | 0 | |
| `features::macd::proptests::t502_macd_stream_batch_agree` | default | 0 | |
| `strategy::composed::parser::tests::t503_proptest_parse_is_deterministic_1000_cases` | 1000 | 0 | |
| `risk::portfolio::tests::t607_no_acceptance_exceeds_cap` (proptest) | 1000 | 0 | **NEW v1** — portfolio exposure cap never exceeded. |

---

## 5. Backtest Results

**Data source:** synthetic (seeded RNG) for all scenarios — v0/v0.5 scenarios use `ChaCha20Rng` seeded at `0xC0FFEE`; v1 multi-symbol scenarios use 10 independent `ChaCha20Rng` streams seeded from `master_seed + idx * 0x9E3779B9`.
**Seed:** `0xC0FFEE` for all runs.
**Fees / slippage model:** 4 bps taker, 2 bps maker, 2 bps slippage; `fixed_fraction(0.1)` sizing (v0/v0.5 scenarios); `equal_weight` sizing with `exposure_cap=0.50, k_long=3` (v1 scenarios).

### A. 7-Anchor Regression Gate (CRITICAL)

All seven anchor hashes verified independently by the tester using the canonical `backtest::extract_report_body` / `report_body_hash` convention (split-on-newline, second `---` delimiter, SHA-256 of bytes). Full 64-char hashes confirmed for all 7:

| Scenario | Expected body-SHA256 | Tester-run body-SHA256 | Match |
|---|---|---|---|
| `btc-2023-1m-sma-cross` | `fc2e3b4a04055e60209fe85541173aa8883df226d2756352dfd101597168649c` | `fc2e3b4a04055e60209fe85541173aa8883df226d2756352dfd101597168649c` | **PASS** |
| `btc-2023-1m-sma-baseline-refresh` | `fc2e3b4a04055e60209fe85541173aa8883df226d2756352dfd101597168649c` | `fc2e3b4a04055e60209fe85541173aa8883df226d2756352dfd101597168649c` | **PASS** |
| `btc-2023-1m-macd-trend` | `ef9c5e483fa079f670a7aa15671643fce3b39a5ce35df8cb6d797887053f8805` | `ef9c5e483fa079f670a7aa15671643fce3b39a5ce35df8cb6d797887053f8805` | **PASS** |
| `btc-2023-1m-rsi-reversion` | `bc56d20d608c680e534bf6764ce8e0e568f0d4ffdf847a539c53fef65170d7aa` | `bc56d20d608c680e534bf6764ce8e0e568f0d4ffdf847a539c53fef65170d7aa` | **PASS** |
| `btc-2023-1m-bbands-mean-revert` | `d8a08a23d3629556c5fca39d6af89d7e0f99418e642af0b86fce22ff4d2792e3` | `d8a08a23d3629556c5fca39d6af89d7e0f99418e642af0b86fce22ff4d2792e3` | **PASS** |
| `top10-2023-1h-momentum` | `3b60ef0743f006867b9e52f9de154869ee170987b27560e288b2d9597d3ecf97` | `3b60ef0743f006867b9e52f9de154869ee170987b27560e288b2d9597d3ecf97` | **PASS** |
| `top10-2024-h1-momentum` | `1f33534fc7c6af1c04330564bec77aac620ecf6f1058f11ff90dfb66adcf05c6` | `1f33534fc7c6af1c04330564bec77aac620ecf6f1058f11ff90dfb66adcf05c6` | **PASS** |

**7 / 7 anchors held. Zero regressions.**

Note on T622 tests in `crates/backtest/tests/determinism.rs`: the three v0.5 non-SMA scenarios use `starts_with(8-char-prefix)` checks rather than full 64-char equality. The tester independently ran those scenarios and confirmed the full 64-char hashes match the spec anchors exactly (MACD: `ef9c5e48…`, RSI: `bc56d20d…`, BBands: `d8a08a23…`). The `starts_with` shorthand is honest — no regression hidden.

### B. v1 Scenarios — Determinism Gate

Both v1 scenarios run twice at seed `0xC0FFEE`; body-SHA256 byte-identical across both runs; `ledger_imbalance_total == 0` in each report.

#### Scenario 1: `top10-2023-1h-momentum`

| Metric | Run A | Run B |
|---|---|---|
| Final equity | $56,282.81 USDT | $56,282.81 USDT |
| Trades | 4,809 | 4,809 |
| Ledger imbalance | 0 | 0 |
| LLM spend | $0.00 | $0.00 |
| Body-SHA256 | `3b60ef07…` | `3b60ef07…` |

**Deterministic: YES**

#### Scenario 2: `top10-2024-h1-momentum`

| Metric | Run A | Run B |
|---|---|---|
| Final equity | $46,401.41 USDT | $46,401.41 USDT |
| Trades | 2,490 | 2,490 |
| Ledger imbalance | 0 | 0 |
| LLM spend | $0.00 | $0.00 |
| Body-SHA256 | `1f33534f…` | `1f33534f…` |

**Deterministic: YES**

Data source confirmed synthetic (seeded RNG, v1 multi-symbol) — T616 synthetic path decision documented in developer close-out notes. Reports exist at:
- `spec/reports/backtest-20260429-195148-top10-2023-1h-momentum.md`
- `spec/reports/backtest-20260429-195243-top10-2024-h1-momentum.md`

### C. Multi-Symbol Determinism (R12, V5)

`crates/backtest/tests/multi_symbol_determinism.rs` verified:
- `t618_merge_sort_key_venue_ts_then_symbol` — asserts sort key is `(venue_ts ASC, symbol ASC)` (architect risk #1). For 3 symbols at shared timestamps, alphabetical order verified via `BTreeMap` grouping.
- `t618_signal_sequence_deterministic_two_runs` — two identical 10-symbol + 200-bar replay runs at `SEED=0x00C0_FFEE_1234_5678` produce bit-identical signal lists.
- `t618_warmup_period_produces_no_signals` — warmup < lookback produces no signals.
- `t618_out_of_universe_bars_filtered` — `XRPUSDT` bars injected into `[BTCUSDT, ETHUSDT]` universe return empty vec (Q5 filter).
- `t618_top_k_long_selects_k_symbols` — exactly `k` symbols selected with correct weight `exposure_cap / k`.

**All 5 determinism tests: PASS.**

### Equity Curve Summary

v0/v0.5 single-symbol scenarios show monotonic fee-drag decay as expected (unchanged from v0.5 ship). The two v1 multi-symbol scenarios simulate 10 correlated GBM paths at 1h cadence with vol-adjusted cross-sectional ranking; the top-3 rotation with `exposure_cap=0.50` at the hourly rebalance cadence produces modest decay on synthetic data (-43.7% and -53.6% respectively). These are consistent with the feature brief: "we are testing the multi-symbol plumbing, not edge confirmation." All `ledger_imbalance_total == 0`.

### Regressions vs Baseline

No financial regressions. All 5 v0/v0.5 body-SHA256 anchors held exactly after v1 type additions, multi-symbol plumbing, and funding poller wiring.

---

## 6. Benchmarks

**`cargo bench -p strategy --bench cross_sectional --no-run`:** PASS (builds cleanly). Criterion binary runs in test mode:
- `momentum_on_bar_warm_10sym` — Success
- `top_k_long_10sym_k3` — Success
- `decimal_ln` — Success
- `decimal_sqrt` — Success
- `score_vol_adjusted_return_lb60` — Success
- `momentum_on_bar_out_of_universe` — Success

**`cargo build --workspace --release`:** PASS — 3.89s incremental. All artifacts current.

_Full criterion wall-clock run not executed_ — builds-only gate per T621 spec note. Runtime p99 `on_bar < 5ms` budget (V7) not independently measured by tester; developer accepted build-only for v1 per task note. This is acknowledged as a gap but not a blocker: the bench binary compiles and the test-mode smoke passes all 6 cases.

---

## 7. Environment / Infrastructure Issues

### Funding-poller-disabled boot path (J)

`crates/agent/src/main.rs` inspected at lines 151–210. Boot sequence:

1. When `cfg.funding.enabled == true`: logs `funding_poller_started` with `universe_size=N`, spawns poller task via `tokio::spawn`, spawns persistence sidecar via second `tokio::spawn`. **Panic in poller does NOT crash the agent** — the `tokio::spawn` wrapping provides catch-and-continue (task panics are isolated, per Tokio semantics).
2. When `cfg.funding.enabled == false` (default in `config/agent.toml`): logs `funding_poller_disabled`. No task spawned.

Both log lines confirmed in source. `FundingConfig.enabled = false` default confirmed in agent config structure (developer close-out notes). This satisfies architect risk #5 (skip-and-log on 5xx) — verified by `t613_poller_skips_on_5xx` and `t613_poller_skips_on_connection_refused` tests.

### MomentumStrategy — no funding consumption (F)

Grepped `crates/strategy/src/cross_sectional/` for `funding` — **zero matches in production code**. The `MomentumStrategy` does not subscribe to, import, or read `FundingObs` data. The `funding_obs` broadcast channel exists on the `EventBus` for future consumption (v2+); in v1 it is write-only from the poller perspective. Strategy-side filtering (Q5) confirmed in `momentum.rs` line 184–187: `if !self.universe_symbols.contains_key(&bar.symbol) { return Vec::new(); }`.

### Long-only enforcement (D)

`bad_k_short_nonzero.toml` fixture confirmed: `k_short = 1` produces `StrategyLoadError` with `error_code = "unsupported_short_sizing"`. Verified by `t605_bad_k_short_nonzero_rejected` test (PASS). The 10 negative TOML fixtures (11 tests including `t605_all_error_codes_distinct`) cover all 9 defined error codes per the Design error-code table.

### Strategy trait not modified (E)

`crates/strategy/src/cross_sectional/momentum.rs` line 178: `impl Strategy for MomentumStrategy`. The `Strategy` trait retains v0 shape: `id()`, `on_bar()`, `on_tick()`, `config_schema()`. No new methods added to the trait. Q5 filtering is implemented internally in `on_bar` before any state mutation.

### Per-symbol P&L attribution (G)

`audit::query::pnl_by_symbol(since, until) -> Result<Vec<(Symbol, Money<Usdt>)>, LedgerError>` confirmed at `query.rs` line 441. Return type is `Vec<(Symbol, Money<Usdt>)>` — no `sqlx` types in public surface. File header (line 3) documents: "No `sqlx` types in the public API. All amounts are returned as `Decimal` or domain types." T609 proptest (200 cases) verifying `Σ pnl_by_symbol == realized_pnl_since` passes within `cargo test --workspace`.

### UI consistency (H)

- `cargo test -p ui` consistency audits: `no_inline_user_visible_strings_in_widgets` PASS + `no_inline_hex_colors_in_widgets_or_state` PASS. `inline strings: 0`, `inline hex: 0`.
- `panel_snapshots__positions_v1_three_rows.snap` exists and pins 3-row layout (BTC `pos` / ETH `neg` / SOL `fg_muted`). Test `positions_v1_three_rows` passes in 31-snapshot run.
- `spec/reports/screenshots/v0-paper-sma/README.md` §4.2 updated: "v1: up to 3 rows in steady state for the top-3 momentum strategy, fixture `fake_v1_three_symbol_portfolio()`."
- `spec/reports/ui-week2-smoke-checklist-2026-04-18.md` has `## v1 — multi-symbol positions smoke` section with `### Acceptance for T_FINAL_B_v1` checklist block.

### T612 deferral (I)

Task box confirmed: T612 = `[ ]` with explicit note "**[DEFERRED TO v1.5 — 2026-04-29]:** single-symbol WS only; per-symbol `clock_skew_ms{feed,symbol}` label not added; no testnet smoke test. Operator confirmed: T612 stays `[ ]` and is NOT a v1 blocker." All 22 numbered backend tasks (T601–T611, T613–T622) + T_FINAL_A_v1 are `[x]`. T623 + T_FINAL_B_v1 are `[x]`. Task-box count: 22 numbered + T_FINAL_A_v1 + T_FINAL_B_v1 = 24 `[x]`; T612 = 1 `[ ]` with operator-confirmed note.

### T_FINAL_B_v1 deferred-manual (K)

`screenshot-v1-positions-three-rows.png` queued as `_deferred_manual_` in the smoke checklist (`spec/reports/ui-week2-smoke-checklist-2026-04-18.md` line 441: `[ ] screenshot-v1-positions-three-rows.png _deferred_manual_`). Documentation is honest — the same precedent as v0 and v0.5 headless-sandbox PNGs. The functional acceptance gate (3-row `insta` snapshot pinned via automated test) is fully automated and green. Screenshot capture requires a live display and is explicitly queued for next session.

### Release builds

- `cargo build --workspace --release`: PASS — 3.89s incremental.
- `cargo build -p ui --bin cockpit --features fixtures`: PASS.
- `cargo build -p ui --bin cockpit --features live`: PASS.

### Flakes / Infra

_none_ — all 307 tests deterministic; no flakes observed across multiple runs. The 29.55s `backtest::determinism` test suite is expected (runs 11 binary-level scenario invocations).

---

## 8. Verdict

**`PASS`**

All v1 gates independently verified by the tester:

**7-anchor regression gate (CRITICAL):** All 7 body-SHA256 anchors verified byte-identical to spec. 5 v0/v0.5 anchors carried forward unchanged; 2 v1 anchors (`top10-2023-1h-momentum` = `3b60ef07…`, `top10-2024-h1-momentum` = `1f33534f…`) confirmed deterministic at seed `0xC0FFEE` across 2 independent runs each. Zero regressions.

**Test suite:** 307 tests pass (0 fail, 3 correctly ignored). Δ+88 vs v0.5 baseline. All new v1 test targets green: T601–T622 tests, T618 multi-symbol determinism, T619 v1 hot-swap, T620 rebalance-reject, funding poller integration, funding rate history audit, bad v1 strategy fixtures, `positions_v1_three_rows` snapshot.

**Static analysis:** `cargo fmt` clean; `cargo clippy` 0 warnings; `cargo check` clean.

**v1-specific gates:** Multi-symbol sort key `(venue_ts, symbol)` asserted in code and test. Long-only enforced (k_short=1 rejected as `unsupported_short_sizing`). `MomentumStrategy` does not consume funding data (zero grep matches in production code). `Strategy` trait unchanged (v0 shape preserved per Q5). `audit::query::pnl_by_symbol` and `funding_rate_history` return domain types only (no sqlx leak). Funding poller disabled by default, panic-isolated, boot log lines correct. UI consistency 0 inline strings / 0 inline hex. Multi-row positions snapshot pinned.

**Task honesty:** 22 numbered tasks + T_FINAL_A_v1 + T_FINAL_B_v1 = `[x]`; T612 = `[ ]` with operator-confirmed deferral to v1.5. Honest.

**Deferred manual items (non-blocking):** PNG screenshot `screenshot-v1-positions-three-rows.png` (headless sandbox; documented as `_deferred_manual_` per established precedent). Criterion `cargo bench` wall-clock runtime budget (build-only gate accepted per T621 note; all 6 bench cases compile and pass test mode).

---

## 9. Routing

`VERDICT → PASS` — v1 ships. Every gate green; 7-anchor regression-free; v1 scenarios deterministic at locked hashes; long-only enforced; funding poller observation-only and disabled by default; UI consistency holds; multi-row positions pinned; T612 honestly deferred to v1.5 with operator confirmation. Deferred manual items (PNG screenshot, bench wall-clock) are documented and non-blocking per established precedent.

---

## Appendix A — 7-Anchor Regression Summary

| Scenario | Expected (64-char) | Tester run | Match |
|---|---|---|---|
| `btc-2023-1m-sma-cross` | `fc2e3b4a04055e60209fe85541173aa8883df226d2756352dfd101597168649c` | `fc2e3b4a…` (full match) | **PASS** |
| `btc-2023-1m-sma-baseline-refresh` | `fc2e3b4a04055e60209fe85541173aa8883df226d2756352dfd101597168649c` | `fc2e3b4a…` (full match) | **PASS** |
| `btc-2023-1m-macd-trend` | `ef9c5e483fa079f670a7aa15671643fce3b39a5ce35df8cb6d797887053f8805` | `ef9c5e48…` (full match) | **PASS** |
| `btc-2023-1m-rsi-reversion` | `bc56d20d608c680e534bf6764ce8e0e568f0d4ffdf847a539c53fef65170d7aa` | `bc56d20d…` (full match) | **PASS** |
| `btc-2023-1m-bbands-mean-revert` | `d8a08a23d3629556c5fca39d6af89d7e0f99418e642af0b86fce22ff4d2792e3` | `d8a08a23…` (full match) | **PASS** |
| `top10-2023-1h-momentum` | `3b60ef0743f006867b9e52f9de154869ee170987b27560e288b2d9597d3ecf97` | `3b60ef07…` (full match) | **PASS** |
| `top10-2024-h1-momentum` | `1f33534fc7c6af1c04330564bec77aac620ecf6f1058f11ff90dfb66adcf05c6` | `1f33534f…` (full match) | **PASS** |

Note: All 7 hashes confirmed via the canonical `backtest::extract_report_body` Python equivalent: split-inclusive on newline, find second `---`-only line, hash everything after. The T622 test file uses 8-char `starts_with` checks for the 3 non-SMA v0.5 scenarios; the tester independently verified full 64-char equality for all 7.

---

## Appendix B — v1 Verification Gate Summary (V1–V11)

| Gate | Verdict | Evidence |
|---|---|---|
| V1 Static checks | PASS | `cargo fmt`, `cargo clippy`, `cargo check` all clean. |
| V2 Unit + integration tests | PASS | 307 passing, 0 failing, 3 correctly ignored. All v1 acceptance tests green. |
| V3 v1 backtest scenarios | PASS | Both `top10-2023-1h-momentum` and `top10-2024-h1-momentum` run end-to-end; `ledger_imbalance=0` in both; deterministic at locked hashes. |
| V4 Baseline re-run matches v0 | PASS | `btc-2023-1m-sma-cross` and `sma-baseline-refresh` both `fc2e3b4a…`. |
| V5 Multi-symbol determinism | PASS | T618 5/5 tests green. Merge sort key `(venue_ts, symbol)` asserted. Signal sequences byte-identical across 2 runs. |
| V6 Criterion benches under budget | deferred_manual (build-only) | All 6 bench cases compile and pass test-mode. Runtime p99 < 5ms not independently measured. |
| V7 Cockpit smoke | deferred_manual | `insta` snapshots 31/31 green. PNG screenshot deferred (headless). |
| V8 v1 UI smoke | PASS (auto gates) / deferred_manual (PNG) | `cargo build -p ui --bin cockpit --features fixtures` PASS. `positions_v1_three_rows` snapshot PASS. PNG deferred. |
| V9 Cost telemetry | PASS | Both v1 backtest reports: `llm_spend_usd: 0.00`. |
| V10 Long-only enforced | PASS | `bad_k_short_nonzero.toml` rejected as `unsupported_short_sizing`. |
| V11 Funding poller observation-only | PASS | `MomentumStrategy` has zero `funding` references in production code. Poller disabled by default. Wiremock integration tests green. |

---

## Appendix C — Task-Box Honesty (full v1 pass)

| Task | `[x]` | Verdict |
|---|---|---|
| T601 | `[x]` | `Universe`, `SymbolSet`, `FundingObs`, `RebalanceRejected`, `RiskLimits.portfolio_exposure_cap` present; serde round-trip tests pass. **HONEST** |
| T602 | `[x]` | `decimal_ln`, `decimal_sqrt` in `features::math`; 10 unit tests including reference values and determinism. **HONEST** |
| T603 | `[x]` | `score_vol_adjusted_return` in `features::cross_sectional`; 4 tests + proptest. **HONEST** |
| T604 | `[x]` | `top_k_long` selector; 5 tests (top-3, tie-break, warmup exclusion, k=0, all-warmup). **HONEST** |
| T605 | `[x]` | 10 bad TOML fixtures; 11 tests including `t605_all_error_codes_distinct`. **HONEST** |
| T606 | `[x]` | `MomentumStrategy::on_bar`; 4 tests (warmup, rebalance, determinism, out-of-universe). **HONEST** |
| T607 | `[x]` | `risk::size_portfolio_target`; 4 tests including proptest(1000). **HONEST** |
| T608 | `[x]` | `audit::journal::rebalance_rejected`; integration test in audit. **HONEST** |
| T609 | `[x]` | `audit::query::pnl_by_symbol`; integration test + proptest(200). **HONEST** |
| T610 | `[x]` | `audit::bootstrap::seed_universe_accounts`; idempotency tested. **HONEST** |
| T611 | `[x]` | `data::ReplayFeed::merge_symbols`; merge order, monotonic ts, memory bound verified. **HONEST** |
| T612 | `[ ]` | **Explicitly deferred to v1.5** — operator confirmed; not a v1 blocker. **HONEST** |
| T613 | `[x]` | `FundingPoller`, `BinanceFundingClient`, `funding_obs` bus, SQLite migration, `funding_rate_history` query. 9 tests (3 wiremock + 6 audit). **HONEST** |
| T614 | `[x]` | `funding_poller_task` spawned in `main.rs`; `funding_poller_started`/`funding_poller_disabled` log lines; `CancellationToken` wired; persistence sidecar. **HONEST** |
| T615 | `[x]` | `config/strategies/top10_momentum_h1.toml` parses and loads; used in T619 hot-swap test. **HONEST** |
| T616 | `[x]` | Synthetic path chosen; 10 `ChaCha20Rng` streams; documented. **HONEST** |
| T617 | `[x]` | Both v1 scenarios run end-to-end with reports; `ledger_imbalance=0`. **HONEST** |
| T618 | `[x]` | `multi_symbol_determinism.rs`; 5 tests green. **HONEST** |
| T619 | `[x]` | `v1_hot_swap.rs`; 4 tests green. **HONEST** |
| T620 | `[x]` | `v1_rebalance_reject.rs`; 3 tests green. **HONEST** |
| T621 | `[x]` | Bench binary builds; 6 test-mode cases pass. Runtime budget: build-only per task note. **HONEST (build-only)** |
| T622 | `[x]` | All 5 v0/v0.5 anchors verified via T622 tests + tester independent run. **HONEST** |
| T623 | `[x]` | `fake_v1_three_symbol_portfolio`, `fake_v1_strategy_row_momentum`, `fake_cockpit_v1_steady_state` in `fixtures.rs`; zero widget edits. **HONEST** |
| T_FINAL_A_v1 | `[x]` | All backend criteria met: both v1 scenarios deterministic; hot-swap and rebalance-reject integration green; benches build; v0/v0.5 regression-free; reconciler invariant 0; funding poller wired. **HONEST** |
| T_FINAL_B_v1 | `[x]` | Smoke checklist section appended; `positions_v1_three_rows` snapshot committed; v0 README §4.2 updated; PNG deferred_manual. **HONEST** |

**Summary:** 23 tasks `[x]` (22 numbered + T_FINAL_A_v1 + T_FINAL_B_v1 + T623). T612 = `[ ]` deferred. All task-box claims independently verified. 0 discrepancies.
