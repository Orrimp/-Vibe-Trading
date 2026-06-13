---
title: Test Report
feature: simple-strategies-realdata
run_id: 2026-06-13-1200-UTC
commit: edfaafec91c02080a583b18914825d60506ad3dd (uncommitted on disk)
agent: tester
verdict: PASS
---

# Test Report — simple-strategies-realdata — 2026-06-13

## 1. Scope

- **Feature / change under test:** Simple single-symbol strategies (v0.sma / v0.5.macd / v0.5.rsi / v0.5.bbands) on real pinned Binance hourly data in the Lab via a three-way data-source toggle (Synthetic / Yahoo / Binance). Adds `ScenarioDataSource::BinanceCache` + `LabDataSource::BinanceCache` + `preload_binance_bars` + `LabBinanceBarSource` trait + `spawn_preload_on_rt<S: LabBarSource>` generalization + `binance` cargo feature. UN-ANCHORED; 119/119 untouched by construction.
- **Spec refs:** `spec/simple-strategies-realdata/feature.md`, `spec/simple-strategies-realdata/tasks.md`
- **Commit SHA:** `edfaafec91c02080a583b18914825d60506ad3dd` (changes uncommitted on disk at test time)
- **Rust toolchain:** `rustc 1.94.1 (e408947bf 2026-03-25)`
- **OS / arch:** darwin arm64
- **Trace row:** `REQ-SIMPLE-STRATEGIES-REALDATA-001` — `state = "proposed"` → `"tester-done"`

## 2. Static Analysis

| Check               | Result | Notes                                                                                             |
|---------------------|--------|---------------------------------------------------------------------------------------------------|
| `cargo fmt --check` | PASS   | `cargo fmt --check -p backtest -p ui` — no diff                                                  |
| `cargo clippy` (prod) | PASS | `cargo clippy -p backtest -p ui` (without `--tests`) — zero errors, zero warnings in touched crates. Pre-existing `Screen::Home` deprecations + `state.rs` expect/unwrap in non-test code are carried from prior runs (same baseline). No new warning introduced by this feature. |
| `cargo clippy --tests` | WARN (pre-existing) | `--tests` exposes 31 pre-existing `clippy::unwrap_in_result` lint errors in `crates/ui/tests/training_poller_subscription.rs` and a vendored `features` crate. Zero new errors attributable to this feature's test files; the new test files carry `#![allow(clippy::unwrap_used)]` appropriately. |
| `cargo audit`       | N/A    | Not run (no new dependencies; feature re-uses existing `data::ReplayFeed` + `data::revision`)    |
| `cargo deny`        | N/A    | No new deps; no new license surface                                                               |
| **spec-lint**       | PASS (no new) | `python3 scripts/spec_lint.py` → 70 violations in 2 categories (dead-link: 65, trace-broken-path: 5). **Baseline (audit-2026-06-12): 71 violations (66 + 5). Improved by 1. Zero new violations attributable to this feature.** |

### Pre-existing spec debt

- `dead-link` (65): all pre-existing links to archived/removed files (`v25-kronos-forecast-overlay/`, `crates/forecast/`, `/tmp/orch-diag/`, etc.) — carried from prior runs, unchanged in character.
- `trace-broken-path` (5): `REQ-VISUAL-FAIL-HTML-REPORTER-001` (2 test paths), `REQ-LAB-YAHOO-REALDATA-V0-1-4-001` (1 arch path), `REQ-QUEUE-STALENESS-RECONCILIATION-001` (1), `REQ-OPERATOR-LEDGER-SCHEMA-LINT-001` (1) — all pre-existing.

## 3. Unit & Integration Tests

### Suite matrix

| Suite / invocation | Passed | Failed | Ignored | Duration |
|---|---:|---:|---:|---:|
| `cargo test -p backtest --test binance_cache_dispatch` (9 tests — 4 accept, 4 reject, 1 divergence) | 9 | 0 | 0 | 0.05s |
| `cargo test -p ui --lib` (456 lib unit tests) | 456 | 0 | 0 | 0.68s |
| `cargo test -p ui --features fixtures` (all fixtures-gated suites) | 72 | 0 | 2 (ignored) | ~5s |
| `cargo test -p ui --features binance` (binance-gated suites: lib + binance integration) | 97 | 0 | 2 (ignored) | ~7s |
| `cargo test -p ui --features live --test lab_run_engine` (H3 round-trip regression) | 1 | 0 | 0 | 2.07s |
| `cargo test -p ui --test live_equity_render` (15 render tests) | 15 | 0 | 0 | 1.19s |
| `cargo test -p ui --test panel_snapshots` (103 snapshot tests) | 103 | 0 | 0 | 0.29s |
| `cargo test -p ui --no-default-features --features live --test lab_source_toggle_no_binance` (AC8 explicit no-binance) | 1 | 0 | 0 | 1.09s |
| `cargo test -p ui --features live --test lab_runner_preload_callthrough_e2e` (ADR-0050 callthrough) | 2 | 0 | 0 | 0.00s |
| **Total** | **756+** | **0** | **2** | |

### New tests introduced by this feature

**`crates/backtest/tests/binance_cache_dispatch.rs`** (9 tests):
- `binance_cache_accepted_by_sma_arm_label_is_binance` — T-A1/AC1: `ScenarioDataSource::BinanceCache` accepted by v0.sma; report body contains `"binance"` label
- `binance_cache_accepted_by_macd_arm` — T-A1/AC1: v0.5.macd accepts BinanceCache
- `binance_cache_accepted_by_rsi_arm` — T-A1/AC1: v0.5.rsi accepts BinanceCache
- `binance_cache_accepted_by_bbands_arm` — T-A1/AC1: v0.5.bbands accepts BinanceCache
- `binance_cache_rejected_by_momentum_arm` — T-A2/AC2: v1.momentum rejects BinanceCache → `UnsupportedDataSource`
- `binance_cache_rejected_by_pairs_arm` — T-A2/AC2: v1.5a.pairs rejects BinanceCache
- `binance_cache_rejected_by_tcn_arm` — T-A2/AC2: v2.5.tcn rejects BinanceCache
- `binance_cache_rejected_by_tcn_weights_arm` — T-A2/AC2: v2.5.tcn.weights rejects BinanceCache
- `binance_cache_real_bars_diverge_from_synthetic_baseline` — T-C1/AC4: real Binance v0.sma equity diverges ≥ 1 USD from synthetic baseline (runs with on-disk corpus; PASS)

**`crates/ui/tests/lab_binance_divergence.rs`** (3 tests, `#[cfg(all(feature = "live", feature = "binance"))]`):
- `loader_returns_nonempty_hourly_bars_with_revision_sha` — AC3: loader returns non-empty hourly bars + revision SHA
- `binance_run_diverges_from_synthetic_baseline` — AC4 (UI-seam path): Binance vs synthetic equity delta ≥ 1 USD; series non-identical
- `loader_missing_corpus_returns_typed_err_not_synthetic` — AC4 design-side: ZZZUSDT → typed Err, NEVER Ok bars

**`crates/ui/tests/lab_binance_render.rs`** (3 tests, `#[cfg(all(feature = "live", feature = "binance"))]`):
- `three_way_toggle_active_chip_marches_right` — AC7/T-B1: ACCENT highlight marches Synthetic < Yahoo < Binance
- `binance_chip_renders_visible_highlight` — AC7/T-B1: Binance chip has ≥ 50 px accent band
- `binance_sourced_equity_curve_rasterizes` — AC7/T-C4: equity curve paints visible ACCENT_2 polyline

**`crates/ui/tests/lab_binance_persist_compare.rs`** (1 test, `#[cfg(all(feature = "live", feature = "binance"))]`):
- `binance_run_persists_and_round_trips_through_compare` — AC5/T-C2: `.md` + CSV written; `EquityCache` element-by-element round-trip; `scan_spec_tree` CachedCell builds

**`crates/ui/tests/lab_source_toggle_no_binance.rs`** (1 test, `#[cfg(not(feature = "binance"))]`):
- `no_binance_feature_renders_two_chips` — AC8: no-`binance` build renders exactly two chips (Synthetic + Yahoo)

### Failing Tests

_none_

## 4. Property / Fuzz Tests

_n/a_ — No proptest or fuzz suite for this feature. The divergence guard (`binance_cache_real_bars_diverge_from_synthetic_baseline`) serves the analogous determinism-verification role.

## 5. Backtest Results

_n/a (strategy logic unchanged)_ — This feature is a data-source / evaluation-tooling change. The four strategies (v0.sma / v0.5.macd / v0.5.rsi / v0.5.bbands) are byte-unchanged. No overlay or sizing modifier added. The CLAUDE.md baseline-equity-divergence gate does NOT apply as written.

**AC4 analog (no-op-source divergence guard) — the purpose-built gate:**
- Test: `binance_cache_real_bars_diverge_from_synthetic_baseline` (`crates/backtest/tests/binance_cache_dispatch.rs:326`)
- Method: run `v0.sma × BTCUSDT × Jan 2023` on real Binance hourly bars AND on synthetic bars with the SAME `(strategy, symbol, seed)`. Assert final equity delta ≥ epsilon.
- **Epsilon: 1 USD (Decimal::ONE)**
- **Result: PASS** — real Binance bars (from `data/binance/BTCUSDT/2023/01.parquet`) reached the strategy; equity curves diverged by well above 1 USD.
- **No-silent-fallback (design-side):** `loader_missing_corpus_returns_typed_err_not_synthetic` (`crates/ui/tests/lab_binance_divergence.rs:274`) — ZZZUSDT → typed cache-miss `Err` with symbol name + re-fetch hint, NEVER `Ok(bars)`. PASS.

**Anchor safety:** UN-ANCHORED by construction. No `spec/anchors.toml` row added. No `spec/*/reports/` committed body mutated.

## 6. Benchmarks

_n/a_ — No hot-path changes. `preload_binance_bars` is a one-shot parquet read on the Lab dispatch path, not latency-sensitive.

## 7. AC Matrix

| AC | Description | Test(s) | Result |
|----|-------------|---------|--------|
| AC1 | `BinanceCache` accepted by 4 single-symbol arms; `data_source` label = `"binance"` | `binance_cache_accepted_by_{sma,macd,rsi,bbands}_arm` (backtest) | PASS |
| AC2 | 4 cross-sectional arms reject `BinanceCache` → `UnsupportedDataSource` | `binance_cache_rejected_by_{momentum,pairs,tcn,tcn_weights}_arm` (backtest) | PASS |
| AC3 | Loader returns non-empty hourly bars + revision SHA; revision-mismatch → loud Err | `loader_returns_nonempty_hourly_bars_with_revision_sha` (ui/binance) | PASS |
| **AC4** | **No-op-source guard: Binance equity diverges ≥ 1 USD from synthetic baseline (same seed)** | `binance_cache_real_bars_diverge_from_synthetic_baseline` (backtest); `binance_run_diverges_from_synthetic_baseline` (ui/binance); **epsilon = 1 USD (Decimal)** | **PASS** |
| AC4 design | Loader NEVER synthesizes on miss → typed Err | `loader_missing_corpus_returns_typed_err_not_synthetic` (`crates/ui/tests/lab_binance_divergence.rs:274`) | PASS |
| AC5 | Persist + Compare round-trip: `.md` + CSV written; EquityCache element-by-element; scan_spec_tree CachedCell | `binance_run_persists_and_round_trips_through_compare` (ui/binance) | PASS |
| AC6 | Anchor tripwire: 119/119 unchanged | `scripts/verify_anchors.sh` → `ANCHORS PASS (119 / 119)` | PASS |
| AC7 | Three-way toggle render-layer verified; Binance equity curve rasterizes | `three_way_toggle_active_chip_marches_right`, `binance_chip_renders_visible_highlight`, `binance_sourced_equity_curve_rasterizes` (ui/binance) | PASS |
| **AC8** | **No-binance build shows two chips (explicit invocation)** | `cargo test -p ui --no-default-features --features live --test lab_source_toggle_no_binance` → `no_binance_feature_renders_two_chips` | **PASS** |

## 8. No-Regression Checks

| Check | Description | Result |
|-------|-------------|--------|
| **H3 STILL PASSES** | `spawn_preload_on_rt` generalization did not break in-memory == cached-disk equity round-trip | `lab_run_engine::inner::h3_in_memory_equals_cached_disk` PASS |
| **ADR-0050 callthrough STILL PASSES** | Both the no-panic gate AND the direct-await-panics proof pass post-generalization | `preload_callthrough_with_spawn_blocking_does_not_panic` PASS; `direct_await_without_rt_spawn_panics` PASS |
| **Anchor tripwire** | 119/119 unchanged | `scripts/verify_anchors.sh` → PASS (119/119) |
| `cargo build cockpit_live` | Build clean | `cargo build -p ui --bin cockpit_live --features live` — zero errors |
| `cargo build cockpit` | Build clean | `cargo build -p ui --bin cockpit --features fixtures` — zero errors |

## 9. Composition Review (file:line)

| Contract | Location | Status |
|----------|----------|--------|
| `ScenarioDataSource::BinanceCache` serde wire = `"binance_cache"` | `crates/backtest/src/engine.rs:186` | VERIFIED |
| Engine label = `"binance"` in all 4 single-symbol arms | `engine.rs:1134, 1206, 1278, 1353` | VERIFIED |
| Cross-sectional reject: `matches!(.., YahooCache \| BinanceCache)` | `engine.rs:896, 941, 983, 1043` | VERIFIED |
| Hourly pinned in loader: `Timeframe::OneHour` | `crates/ui/src/lab/runner.rs:681` | VERIFIED |
| NO engine timeframe field | `ScenarioConfig` — no timeframe field added | VERIFIED |
| `Decimal` never `f64` in loader/CSV path | `runner.rs:39,65,81,122,139,1486,1495` — all `rust_decimal::Decimal` | VERIFIED |
| Binance chip is `#[cfg(feature = "binance")]`-gated | `runner.rs:574,585,651,735,1118,1429` | VERIFIED |
| `LabDataSource::BinanceCache` serde = `"binance_cache"` | `crates/ui/src/lab/state.rs:50, 609` | VERIFIED |

## 10. Environment / Infrastructure Issues

_none_ — Binance parquet corpus (`data/binance/`) IS present on this machine. All 9 `binance_cache_dispatch` tests ran for real (not skipped). Visual baselines were pre-rebased by the orchestrator before this test run.

## 11. Verdict

**`PASS`**

All 9 AC matrix items verified. The no-op-source divergence guard (AC4) is the headline gate: `binance_cache_real_bars_diverge_from_synthetic_baseline` (epsilon = 1 USD Decimal) and `binance_run_diverges_from_synthetic_baseline` both PASS — real Binance hourly bars (pinned parquet revision `3a8b96c4…`) reached the v0.sma strategy and produced an equity curve that diverges from the synthetic GBM baseline by well above the epsilon floor. The AC8 explicit no-binance invocation (`--no-default-features --features live`) proves the toggle shows exactly two chips when the feature is absent. H3 still passes post-`spawn_preload_on_rt` generalization. ADR-0050 callthrough (both the no-panic gate and the direct-await-panics proof) passes. Anchor gate: 119/119 (unchanged by construction). spec-lint: 70 violations, 0 new (improved from 71 baseline). `cargo fmt --check` clean. Production-code clippy clean. Both binary builds clean.

## 12. Routing

`VERDICT → PASS` — feature is ready to ship. Tester has ticked T-C1 through T-C5, updated `spec/trace.toml` row `REQ-SIMPLE-STRATEGIES-REALDATA-001` to `state = "tester-done"` + filled `crates` / `tests` columns, and updated `feature.md` + `tasks.md` to `status: tester-done`.

```toml
[handoff]
from         = "tester"
to           = "presenter"
feature      = "simple-strategies-realdata"
trace_refs   = ["REQ-SIMPLE-STRATEGIES-REALDATA-001"]
verdict      = "PASS"
priority     = "normal"

[inputs]
brief        = "/tmp/brief-simple-strategies-realdata.md"
artifacts    = [
  "spec/simple-strategies-realdata/feature.md",
  "spec/simple-strategies-realdata/tasks.md",
  "spec/trace.toml",
]

[outputs]
spec_files   = [
  "spec/simple-strategies-realdata/reports/test-2026-06-13-simple-strategies-realdata.md",
  "spec/simple-strategies-realdata/feature.md",
  "spec/simple-strategies-realdata/tasks.md",
  "spec/trace.toml",
]
lint_result  = "spec-lint: 70 violations in 2 categories — 0 new (baseline 71). No new violation introduced by this feature."
anchors_result = "scripts/verify_anchors.sh → ANCHORS PASS (119 / 119). UN-ANCHORED by construction; no anchors.toml row added."

[open_questions]
items = []

[assumptions]
items = [
  "Binance parquet corpus (data/binance/) present on this machine; divergence tests ran for real (not skipped).",
  "11 visual baselines pre-rebased by orchestrator before this run; visual baseline changes are intentional (3-way toggle chip added).",
  "Pre-existing clippy lint debt in training_poller_subscription.rs and vendored features crate is not attributable to this feature.",
]
```
