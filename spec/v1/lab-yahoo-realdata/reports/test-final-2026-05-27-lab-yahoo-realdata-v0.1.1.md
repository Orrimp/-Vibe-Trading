---
title: Test Report — lab-yahoo-realdata v0.1.1
feature: lab-yahoo-realdata
run_id: 2026-05-27-1520-UTC
commit: bb14e1119e6ed6149adec3f03dcb34d02259e29e
agent: tester
verdict: FAIL
---

# Test Report — lab-yahoo-realdata v0.1.1 — 2026-05-27 15:20 UTC

## 1. Scope

- **Feature / change under test:** lab-yahoo-realdata v0.1.1 — Yahoo anchor lock (68 → 69), H1/H2 hypothesis discharge, `run_yahoo_sma` binary, REVISION.toml wiring
- **Spec refs:** `spec/lab-yahoo-realdata/feature.md`, `spec/lab-yahoo-realdata/tasks.md`
- **Commit SHA:** `bb14e1119e6ed6149adec3f03dcb34d02259e29e`
- **Rust toolchain:** `rustc 1.94.1 (e408947bf 2026-03-25)`
- **OS / arch:** `Darwin 25.5.0 arm64 (Apple Silicon)`

### Working-tree context

**IMPORTANT:** At time of tester run, the working tree contains uncommitted in-flight changes from a SEPARATE feature, `cockpit-toast-queue` (developer agent `a9702781045e3289b`). These changes are:

- `crates/ui/src/widgets/toast_tray.rs` — untracked new file
- `crates/ui/src/widgets/mod.rs` — modified (+5 lines, adds `pub mod toast_tray;`)
- `crates/ui/src/state.rs` — modified (+255 lines, toast-queue Wave A)
- `crates/ui/src/bin/cockpit_live.rs`, `crates/ui/src/live.rs`, `crates/ui/src/shell.rs`, `crates/ui/src/strings.rs` — modified

The v0.1.1 commit (`bb14e11`) explicitly excludes these (noted in commit message: "NOT included in this commit (owned by in-flight cockpit-toast-queue dev agent a9702781045e3289b): crates/ui/src/state.rs"). All static-analysis and test failures that trace to `toast_tray` / `toast_tray.rs` / `state.rs` toast additions are **cockpit-toast-queue pre-existing in-flight work, NOT regressions traceable to v0.1.1**.

The tester's responsibility is to assess failures attributable to v0.1.1 specifically. Failures attributable solely to cockpit-toast-queue in-flight work are noted and attributed to that feature's developer.

---

## 2. Static Analysis

| Check                                                      | Result           | Notes                                                                 |
|------------------------------------------------------------|------------------|-----------------------------------------------------------------------|
| `cargo fmt --all --check`                                  | FAIL             | Diff in `crates/ui/src/state.rs` (toast-queue in-flight code). v0.1.1 crates (`backtest`, spec files) are format-clean. |
| `cargo clippy -p backtest --features yahoo --bin run_yahoo_sma -- -D warnings` | PASS | 0 warnings, 0 errors. |
| `cargo clippy --workspace --features candle,realdata,live -- -D warnings` | PASS at bb14e11 (dev-reported) | Current working-tree: not re-run workspace-wide due to toast-tray compile errors in UI crate. |
| `cargo audit`                                              | not run          | No new external deps added in v0.1.1 (yahoo_finance_api was already added in v0.1.0 Wave C-2). |
| `cargo deny`                                               | not run          | No new deps in v0.1.1. |

### `cargo fmt --check` failure detail

Diffs are exclusively in `crates/ui/src/state.rs` lines 4177 and 4188 — toast-queue test functions added by the `cockpit-toast-queue` in-flight developer. The two formatting violations are:

1. `c.toast_queue.iter().all(|t| t.severity == ...)` — needs line-break per rustfmt width rules.
2. `update(&mut c, Message::ShowToast(...))` — needs line-break.

These are NOT in any file touched by bb14e11 (which only touches `crates/backtest/`, `crates/ui/tests/lab_yahoo_anchor.rs`, and `spec/` files). Attribution: cockpit-toast-queue developer.

**v0.1.1-specific static analysis: PASS.** The `run_yahoo_sma` binary and all `crates/backtest/` changes are clippy-clean and format-clean.

---

## 3. Unit and Integration Tests

### Workspace `--lib` results

| Crate         | Passed | Failed | Ignored | Duration | Notes |
|---------------|-------:|-------:|--------:|----------:|-------|
| `trading_core`| 69     | 0      | 0       | 0.91s    |       |
| `data`        | 56     | 0      | 1       | 0.06s    | 1 network-gated ignored |
| `backtest`    | 103    | 0      | 0       | 0.07s    |       |
| `strategy`    | 105    | 0      | 0       | 0.02s    |       |
| `exec`        | 55     | 0      | 0       | 0.27s    |       |
| `audit`       | 84     | 0      | 0       | 0.17s    |       |
| `agent`       | 63     | 0      | 0       | 0.23s    |       |
| `ui`          | 396    | 1      | 0       | 0.43s    | 1 failure — see below |
| other crates  | 256    | 0      | 6       |          | core, llm, forecast, etc. |
| **Total**     | **1187**| **1** | **7**   |           |       |

Dev claimed at commit time: `cargo test --workspace --lib → 1184 passed, 0 failed`. Current run: 1187 passed, 1 failed. The 3 additional passes come from cockpit-toast-queue test additions. The 1 failure is:

### Failing Test

**`gallery::tests::every_widget_mod_is_listed_in_expected_widgets`** in `crates/ui/src/gallery/mod.rs:231`

```
thread 'gallery::tests::every_widget_mod_is_listed_in_expected_widgets' panicked:
widgets/mod.rs has `pub mod` entries not listed in EXPECTED_WIDGETS: ["toast_tray"]
Add them to `gallery::routes::EXPECTED_WIDGETS` and author a GalleryCell.
```

**Attribution: cockpit-toast-queue in-flight developer, NOT v0.1.1.**

Root cause: The cockpit-toast-queue developer added `pub mod toast_tray;` to `widgets/mod.rs` (working-tree modification) but has not yet added `"toast_tray"` to `EXPECTED_WIDGETS` in `gallery/routes.rs`, nor authored a `GalleryCell`. At commit `bb14e11`, `widgets/mod.rs` had no `toast_tray` reference — the gallery test was passing (dev-reported: 1184 passed 0 failed).

This failure is a cockpit-toast-queue gap, not a v0.1.1 regression.

### v0.1.1-specific tests

| Test | Result |
|------|--------|
| `cargo test -p backtest --features yahoo` | PASS — all backtest tests including new binary path |
| `cargo test -p data --features yahoo --lib yahoo` | PASS (9 passed) |
| `cargo test -p data --features yahoo --test yahoo_revision_verify` | PASS (5 passed, 1 ignored) |
| `cargo test -p ui --features yahoo --test lab_yahoo_dispatch` | PASS (7 passed) |
| `cargo test -p ui --lib lab::universe::tests::yahoo_crypto_universe_has_10_entries` | PASS |
| `cargo test -p trading_core --lib venue::tests::venue_yahoo_display_parse_serde` | PASS |

---

## 4. Property / Fuzz Tests

_n/a — no proptest or cargo-fuzz suites added in v0.1.1._

---

## 5. Backtest Results

### Step 2 (required): Verify-anchors gate

`bash scripts/verify_anchors.sh` → **ANCHORS PASS (69 / 69)**

All 69 anchors pass. The new anchor `btc-yahoo-2024-1d-sma-cross` (row 69, added by v0.1.1) passes. All 68 prior anchors remain byte-identical.

### Determinism check (required by v0.1.1 scope)

Binary: `cargo run --release -p backtest --features yahoo --bin run_yahoo_sma -- --cache-root data/yahoo --reports-dir /tmp/yahoo-det-test`

| Run | Report filename | Body SHA-256 |
|-----|-----------------|-------------|
| Run 1 | `backtest-20260527-144822-btc-yahoo-2024-1d-sma-cross.md` | `8045623b4c9b7d9e25e3b53156bd64363d87e575a2f9c4cb0d8b291ae7bb4867` |
| Run 2 | `backtest-20260527-144836-btc-yahoo-2024-1d-sma-cross.md` | `8045623b4c9b7d9e25e3b53156bd64363d87e575a2f9c4cb0d8b291ae7bb4867` |
| Anchored | `backtest-20260527-143420-btc-yahoo-2024-1d-sma-cross.md` | `8045623b4c9b7d9e25e3b53156bd64363d87e575a2f9c4cb0d8b291ae7bb4867` |

**DETERMINISM PASS.** SHA matches the anchored value `8045623b4c9b7d9e25e3b53156bd64363d87e575a2f9c4cb0d8b291ae7bb4867` across 2 independent tester runs.

### New Yahoo anchor metrics

**Universe:** BTC-USD  
**Period:** 2024-01-01 → 2024-12-31  
**Data source:** Yahoo Finance parquet cache (REVISION.toml SHA `7b33166e1eb80dc0e0076dcde89ca56f36b9b0d695d21aed8effcb2e052ef5d7`)  
**Cache state:** 12 monthly parquets (01.parquet–12.parquet), verified on disk at `data/yahoo/BTC-USD/1d/2024/`  
**Strategy:** SMA crossover (fast=20, slow=50), fixed fraction 10%, slippage 2 bps, taker fee 4 bps  
**Seed:** 0xC0FFEE

| Metric           | Value              |
|------------------|-------------------:|
| Bars replayed    | 367                |
| Trades           | 7                  |
| Final equity     | $104,560.08 USDT   |
| Total return     | +4.56%             |
| Sharpe (ann.)    | 34.3359            |
| Max drawdown     | 4.83%              |
| Total fees       | $28.20 USDT        |
| Ledger imbalance | 0 (PASS)           |

### H1 hypothesis re-verification

**H1:** Yahoo daily BTC-USD 2024 equity series diverges from Binance hourly BTCUSDT on the same span by < 30%.

Comparison basis: H1 2024 (2024-01-01 → 2024-07-01)

| Source | Period | Cadence | Bars | Trades | Final equity |
|--------|--------|---------|------|--------|--------------|
| Yahoo BTC-USD (parquet cache) | H1 2024 | 1d | 182 | 4 | $101,202.81 |
| Binance BTCUSDT (real Binance Vision) | H1 2024 | 1h | 17,544 | 441 | $111,248.17 |

Tester verification of H1 arithmetic:

```
|101,202.81 - 111,248.17| / 111,248.17 = 10,045.36 / 111,248.17 = 9.03%
```

**9.03% < 30% threshold → H1 PASS.** Binance figure sourced from `spec/v0-paper-sma/reports/backtest-20260527-143549-btc-2024-h1-sma-cross.md` (final equity $111,248.17). Dev-note cited $111,248.16 — 1-cent rounding difference has no material impact.

Expected divergence explanation (from dev-note, confirmed by tester):
- Cadence mismatch: 1d (182 bars) vs 1h (17,544 bars) generates 4 vs 441 trades
- Both strategies are profitable in BTC's H1 2024 bull run ($43k → $65k+)
- Daily path misses intraday crossovers, capturing slower trend moves only

### H2 hypothesis re-verification

**H2:** `yahoo_finance_api 4.1.x` fetch success rate > 95% over a 7-day window.

Operator's fetch run 2026-05-27: 1/1 invocations successful (366/366 bars returned).  
**H2 PASS (trivially at scale=1).** The hypothesis is satisfied at the available evidence scale. At-scale re-measurement deferred to v0.2.0 auto-refresh feature if implemented.

### REVISION.toml byte-immutability check (Step 10 required)

`data/yahoo/REVISION.toml` aggregate SHA: `7b33166e1eb80dc0e0076dcde89ca56f36b9b0d695d21aed8effcb2e052ef5d7`

Anchored report `data_source` field: `yahoo-cache:BTC-USD/1d/2024 rev=7b33166e1eb8`

First 12 characters of SHA: `7b33166e1eb8` — **matches.** The anchored report and the on-disk REVISION.toml are consistent.

### Baseline comparison (existing anchors)

All 68 pre-v0.1.1 anchors remain byte-identical. `ANCHORS PASS (69/69)` confirms no regression on existing anchored scenarios.

---

## 6. Benchmarks

_n/a — v0.1.1 adds a new batch backtest binary on a parquet-cache hot path. No latency-sensitive hot-path changes. No criterion benchmarks applicable._

---

## 7. Environment / Infrastructure Issues

- **Yahoo parquet cache present:** `data/yahoo/BTC-USD/1d/2024/` — 12 files (01.parquet–12.parquet). Cache was operator-populated on 2026-05-27. No network egress during tester verification (H3 satisfied by architecture).
- **Working-tree contamination from cockpit-toast-queue:** See Section 1 context. In-flight changes cause `cargo fmt` FAIL and 1 gallery test failure, both attributable to cockpit-toast-queue. No flakiness from v0.1.1 changes themselves.
- **spec_lint.py Python version:** requires Python 3.11+ (tomllib). Run via `uv run python3` (confirmed working).

---

## 8. Spec-Lint Gate

`uv run python3 scripts/spec_lint.py` → `spec-lint: FAIL (73 violations in 5 categories)`

Baseline from `spec/dev-notes/audit-2026-05-25.md`: **61 violations in 1 category (dead-link only)**.

Category delta table:

| Category             | Baseline (2026-05-25 audit) | This run | Δ | Attribution |
|----------------------|----------------------------:|--------:|---|-------------|
| dead-link            | 61 | 69 | +8 | 1 from v0.1.1 (`dev-notes/v0_1_1-fetch-plan-2026-05-25.md` ADR-0040 wrong relative path). 7 from features landed after 2026-05-25 audit (lab-polish-round-2, cockpit-toast-queue arch docs, etc.). |
| missing-frontmatter  | 0  | 1  | +1 | `spec/lab-polish-round-2/tasks.md` — landed 2026-05-25T23:05, AFTER the audit (2026-05-25T15:39). NOT v0.1.1. |
| unreferenced-anchor  | 0  | 1  | +1 | **`btc-yahoo-2024-1d-sma-cross` not cited by any `trace.toml` row.** NEW from v0.1.1. Root cause: developer used a file path (not scenario name) in `trace.toml anchors[]`. |
| shipped-no-tests     | 0  | 1  | +1 | `spec/lab-end-to-end-v2/feature.md` — was shipped with no test report prior to the audit; audit previously showed 0 due to spec_lint.py behavior change (commit 18d9066, 2026-05-26). NOT v0.1.1. |
| trace-broken-path    | 0  | 1  | +1 | **`REQ-LAB-YAHOO-REALDATA-001` anchors: file path not in anchors.toml.** NEW from v0.1.1. Same root cause as `unreferenced-anchor`. |
| **TOTAL**            | **61** | **73** | **+12** | |

### New regressions traceable to v0.1.1 (BLOCKS PASS)

1. **`unreferenced-anchor`**: `spec/anchors.toml` anchor `btc-yahoo-2024-1d-sma-cross` not cited by any `trace.toml` row.
2. **`trace-broken-path`**: `spec/trace.toml` row `REQ-LAB-YAHOO-REALDATA-001` lists `"spec/lab-yahoo-realdata/reports/backtest-20260527-143420-btc-yahoo-2024-1d-sma-cross.md"` in `anchors[]`. The spec-lint expects **scenario names** (matching `spec/anchors.toml` `scenario` keys), not file paths.

**Fix required:** In `spec/trace.toml`, row `REQ-LAB-YAHOO-REALDATA-001`, change the `anchors` array from the file path to the scenario name:

```toml
# WRONG (current):
anchors = [
  "spec/lab-yahoo-realdata/reports/backtest-20260527-143420-btc-yahoo-2024-1d-sma-cross.md",
]

# CORRECT (required):
anchors = [
  "btc-yahoo-2024-1d-sma-cross",
]
```

### Pre-existing spec debt (not blocking PASS when fixed)

- 61 dead-link violations (carry-over from prior audits; none in strategy/exec/backtest/anchor code)
- `missing-frontmatter` for `spec/lab-polish-round-2/tasks.md` (not v0.1.1)
- `shipped-no-tests` for `spec/lab-end-to-end-v2/feature.md` (pre-existing; spec_lint.py behavior change exposed it)

---

## 9. Anchor Verification Gate (verify-anchors)

**Crates touched by v0.1.1:** `crates/backtest/` — REQUIRES anchor verification per the non-negotiable gate.

`bash scripts/verify_anchors.sh` → **ANCHORS PASS (69 / 69)**

All 69 anchors verified. New anchor `btc-yahoo-2024-1d-sma-cross` at row 69:

```
PASS  btc-yahoo-2024-1d-sma-cross  8045623b4c9b7d9e25e3b53156bd64363d87e575a2f9c4cb0d8b291ae7bb4867
```

All 68 prior anchors remain byte-identical (no mutations from v0.1.1 changes).

---

## 10. Verdict

**`FAIL`**

### What passed

- Verify-anchors: **PASS 69/69** — the critical backtest gate is clean
- Determinism: **PASS** — body SHA `8045623b...` reproduced across 2 independent tester runs, matching the anchored value
- `cargo build -p backtest --features yahoo --bin run_yahoo_sma` — **PASS**
- `cargo clippy -p backtest --features yahoo --bin run_yahoo_sma -- -D warnings` — **PASS** (0 warnings)
- H1 (Yahoo vs Binance divergence): **PASS** — 9.03% < 30% threshold. Tester independently verified the arithmetic.
- H2 (fetch success rate): **PASS** — 100% > 95% threshold (trivially at scale=1)
- Yahoo parquet cache present and consistent with anchored report's REVISION.toml SHA prefix
- Workspace lib tests (v0.1.1-scoped crates): all passing

### What failed (blocks PASS)

1. **`spec-lint: FAIL` — 2 new violations traceable to v0.1.1 (not pre-existing baseline)**:
   - `unreferenced-anchor`: `btc-yahoo-2024-1d-sma-cross` anchor declared in `spec/anchors.toml` but not cited by any `trace.toml [[req]]` row
   - `trace-broken-path`: `REQ-LAB-YAHOO-REALDATA-001` `anchors[]` column contains a file path instead of a scenario name

   The fix is a 1-line edit in `spec/trace.toml`: replace the file path with the scenario name `"btc-yahoo-2024-1d-sma-cross"`.

2. **`cargo fmt --check`: FAIL** — formatting diff in `crates/ui/src/state.rs`. Attribution is the cockpit-toast-queue in-flight developer, NOT v0.1.1. However, the workspace `cargo fmt` check fails as observed, which per CLAUDE.md non-negotiables is a hard gate. The developer responsible for cockpit-toast-queue must run `cargo fmt` before the tester can clear this check.

### What is NOT blocking (deferred per operator/tester scope agreement)

- T-T5 (cockpit-smoke): requires live macOS window runtime; offline tester context
- T-T8 (idle-CPU check): requires live cockpit runtime
- T-D2 (cache-state badge widget): UI-designer deliverable, deferred to v0.1.2
- T-D4 (visual consistency review): deferred
- T-D5 (panel-snapshot refresh planning): deferred

---

## 11. Open Items

| Item | Disposition |
|------|------------|
| T-D2 — cache-state badge widget | Deferred — see backlog; v0.1.2 follow-on |
| T-D4 — visual consistency review | Deferred — see backlog |
| T-D5 — panel-snapshot refresh planning | Deferred — see backlog |
| T-T5 — cockpit-smoke | Deferred — requires live macOS runtime; operator-run only |
| T-T8 — idle-CPU check | Deferred — requires live cockpit runtime; operator-run only |

---

## 12. Routing

`HANDOFF → developer`

**Two issues require developer action before PASS:**

1. **trace.toml wiring bug (v0.1.1):** `spec/trace.toml` row `REQ-LAB-YAHOO-REALDATA-001` `anchors[]` must be changed from the report file path to the scenario name `"btc-yahoo-2024-1d-sma-cross"`. This resolves both `unreferenced-anchor` and `trace-broken-path` spec-lint violations simultaneously.

2. **`cargo fmt` failure (cockpit-toast-queue in-flight):** The cockpit-toast-queue developer must run `cargo fmt` on `crates/ui/src/state.rs` before committing. This is a cockpit-toast-queue blocker, not a v0.1.1 blocker per-se, but it causes the workspace-level `cargo fmt --check` to fail. Once the cockpit-toast-queue developer formats their code, the tester can re-verify.

Once both issues are resolved, re-run tester with:
- `uv run python3 scripts/spec_lint.py` — should show `unreferenced-anchor: 0` and `trace-broken-path: 0`
- `cargo fmt --all --check` — should be clean
- No re-run of determinism or verify-anchors required (those passed)
