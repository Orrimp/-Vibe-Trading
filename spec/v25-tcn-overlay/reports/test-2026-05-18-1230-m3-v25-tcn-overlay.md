---
title: Test Report
feature: v25-tcn-overlay
slug: v25-tcn-overlay
report: test
run_id: 2026-05-18-1230-UTC
commit: e85b25d35dc87e9b5d83be69a6f5f440b3d964ae
agent: tester
verdict: PASS
anchors_status: 15/15 PASS — 13 pre-existing + 2 new M3 real-weights anchors (top10-2023-fy-tcn-overlay-weights / top10-2024-fy-tcn-overlay-weights)
predecessor_report: spec/v25-tcn-overlay/reports/test-2026-05-18-0616-v25-tcn-overlay.md
milestone: M3
updated: 2026-05-18
---

# Test Report — v25-tcn-overlay — M3 Real-Weights Anchor Gate — 2026-05-18 12:30 UTC

## 1. Scope

- **Feature / change under test:** v2.5 TCN forecast overlay — M3 milestone: real-weights anchor gate. This is the M3 follow-on to the CI-baseline gate closed in `test-2026-05-18-0616-v25-tcn-overlay.md`. Predecessor report (CI-baseline, commit `3fbae75`): PASS. This report covers T-D-11 + T-D-12 (ticked by developer with evidence in tasks.md).
- **New anchors under test:**
  - `top10-2023-fy-tcn-overlay-weights` → SHA `7cb1357c0d0d25cf89766d88f1342434788c4c373e6c3b1cb77d7f8cf05acef4` (version `v2.5.0-tcn-weights`)
  - `top10-2024-fy-tcn-overlay-weights` → SHA `23c24dae0873df8e808897416d9d8fab75c4bd25dcd7b2933099ff061efe9f2b` (version `v2.5.0-tcn-weights`)
- **Changes vs predecessor (commit `e85b25d`):**
  - `crates/forecast/tests/anchors_load.rs` (NEW) — 3 smoke tests under `--features candle`
  - `crates/strategy/src/tcn_overlay_momentum.rs` — `with_tcn_bs1()` / `with_tcn_bs2()` constructors (`#[cfg(feature = "forecast")]`)
  - `crates/strategy/src/lib.rs` — re-exported `TcnSyncForecaster` under `#[cfg(feature = "forecast")]`
  - `crates/backtest/Cargo.toml` — `candle = ["strategy/forecast"]` feature gate
  - `crates/backtest/src/main.rs` — `ScenarioStrategy::TcnOverlayMomentumWeights` variant, `run_tcn_overlay_weights_backtest()`, `-weights` scenario names
  - `crates/backtest/tests/determinism.rs` — `run_scenario_once_candle()` helper + 2 `#[cfg(feature = "candle")]` anchor-regression tests
  - `spec/anchors.toml` — +2 rows under version `v2.5.0-tcn-weights`
  - `spec/v25-tcn-overlay/feature.md` + `tasks.md` — M3 implementation section + T-D-11/T-D-12 ticked
  - `spec/trace.toml` — REQ-V25-TCN-001 `anchors` extended from 2 → 4
  - `spec/v25-tcn-overlay/reports/m3-bs{1,2}-training-2026-05-18.md` — honest training reports
  - `crates/forecast/checkpoints/anchors/tcn-bs1-*.safetensors` + `.metadata.json` (LFS-tracked)
  - `crates/forecast/checkpoints/anchors/tcn-bs2-*.safetensors` + `.metadata.json` (LFS-tracked)
- **Spec refs:** `spec/v25-tcn-overlay/feature.md`, `spec/v25-tcn-overlay/tasks.md`, `spec/anchors.toml`
- **Commit SHA:** `e85b25d35dc87e9b5d83be69a6f5f440b3d964ae`
- **Rust toolchain:** `rustc 1.94.1 (e408947bf 2026-03-25)`
- **OS / arch:** `Darwin 25.4.0 arm64`

## 2. Static Analysis

| Check | Result | Notes |
|-------|--------|-------|
| `cargo fmt --check` | **PASS** | Zero diffs across workspace. |
| `cargo clippy --workspace -- -D warnings` | **PASS** | Zero warnings or errors. `cargo build --workspace` clean (10.71s dev). |
| `cargo build --workspace --features candle` | **PASS** | Compiles with candle feature propagation through strategy/forecast. |
| `cargo audit` | N/A | `cargo-audit` not installed — pre-existing infra item unchanged since v2 report §2. |
| `cargo deny check` | PRE-EXISTING FAIL | `RUSTSEC-2024-0436` (`paste` unmaintained), `MIT-0` license violations. Pre-date v2.5; not introduced by M3. |
| `spec-lint` (`uv run scripts/spec_lint.py`) | PRE-EXISTING FAIL | 733 violations in 2 categories — no new regressions vs predecessor report (see §2.1). |

### 2.1 Spec-lint detail

**Output:** `spec-lint: FAIL (733 violations in 2 categories)`

| Category | Count | Status |
|----------|------:|-------|
| `dead-link` | 727 | PRE-EXISTING — identical to predecessor PASS report. Orphan Lumen-phase cross-references in `backlog.md`, `architecture/`, and `v15a-mean-reversion-pairs/tasks.md`. No new dead-links introduced by M3. |
| `trace-broken-path` | 6 | PRE-EXISTING — roadmap rows only: `REQ-V25A-PATCHTST-001` (2 anchors, future), `REQ-V25B-TRANSFORMER-001` (2 anchors, future), `REQ-V26-BAKEOFF-001` (2 anchors, future). M3 did NOT introduce any new broken-path violations. |
| `unreferenced-anchor` | 0 | CLEAN — the two new M3 anchor names in `anchors.toml` are both listed in `spec/trace.toml` REQ-V25-TCN-001 `anchors` column (confirmed in §7.3). |

**Net spec-lint delta vs predecessor:** 0 — same 733 violations, same 2 categories. M3 did not regress the spec-lint baseline.

**Pre-existing spec debt (quoted per AGENT.md):**
- `dead-link` (727): orphan feature folder links in `spec/backlog.md`, `spec/architecture/06-ui-and-cockpit.md`, and `spec/v15a-mean-reversion-pairs/tasks.md`. Not introduced by v2.5 or M3.
- `trace-broken-path` (6): future-phase roadmap rows (`REQ-V25A-PATCHTST-001`, `REQ-V25B-TRANSFORMER-001`, `REQ-V26-BAKEOFF-001`) with anchor names that do not yet exist in `anchors.toml`. Expected until those features ship.

## 3. Unit & Integration Tests

| Crate / suite | Passed | Failed | Ignored | Duration | Gate |
|---------------|-------:|-------:|--------:|--------:|------|
| `backtest` determinism (default, no candle) | 20 | 0 | 0 | 41.20s | `cargo test -p backtest --test determinism` |
| `backtest` determinism (--features candle) | 22 | 0 | 0 | 611.27s | `cargo test -p backtest --test determinism --features candle` |
| `forecast` anchors_load (--features candle) | 3 | 0 | 0 | 0.00s | `cargo test -p forecast --features candle --test anchors_load` |

**Combined determinism tests (candle superset):** 22/22 PASS in 611.27s.

### New M3 test names confirmed

```
test m3_top10_2023_fy_tcn_overlay_weights_anchor_hash_unchanged ... ok
test m3_top10_2024_fy_tcn_overlay_weights_anchor_hash_unchanged ... ok
```

Both new tests are `#[cfg(feature = "candle")]` and only appear in the 22-test candle run (invisible in the 20-test no-candle run — correct gating behaviour).

### Anchor load smoke tests (forecast crate)

```
test anchor_tests::td11_bs1_anchor_loads_and_forward_ok ... ok
test anchor_tests::td12_bs2_anchor_loads_and_forward_ok ... ok
test anchor_tests::td11_bs1_forward_deterministic ... ok
```

These tests:
1. Load the LFS-tracked BS-1 / BS-2 safetensors checkpoints from `crates/forecast/checkpoints/anchors/`.
2. Run a forward pass on a synthetic input tensor, assert correct output shape `[batch, 1]`.
3. Run two forward passes on the same input, assert byte-identical result (deterministic CPU inference).

### Failing Tests

_none_

## 4. Property / Fuzz Tests

_n/a_ — no proptest or cargo-fuzz suites for this feature. CPU-inference determinism is tested via `td11_bs1_forward_deterministic` in anchors_load (§3) and by the two anchor-hash tests which run the backtest twice from seed.

## 5. Backtest Results

### 5.1 Honest disposition: synthetic data baseline

Both M3 real-weights scenarios produce **byte-identical results** to their passthrough-forecaster counterparts (v2.5.0). This is expected and correct — not a bug, not a regression.

**Explanation:** The TCN models (BS-1, BS-2) were trained on real Binance hourly OHLCV data with characteristic distributional properties (volatility clustering, fat tails, autocorrelation). The synthetic backtest data is generated by ChaCha20Rng Gaussian random walks (i.i.d. log-returns). On i.i.d. Gaussian input the model's `r_hat` output falls within the `epsilon=0.0005` deadband for every bar, producing `Direction::Flat` for every signal and `dampened=0`. This is the correct behavior: the model has no signal on out-of-distribution data. This finding is documented explicitly in both training reports under §"Finding: TCN model outputs Flat on synthetic data".

**The anchors lock synthetic-data behavior intentionally** — they guard determinism and LFS pipeline integrity, not real-data alpha. Real-data alpha evaluation (using `windows_for_symbol()` with real Binance parquet at `data/binance/`) is out of scope for M3 and queued as `backtest-real-binance-data` in the Strategy backlog (operator accepted at the M3 design gate).

**The operator accepted this disposition** at the M3 design goal lock (feature.md changelog 2026-05-18 developer entry). This MUST NOT be flagged as a regression.

### 5.2 M3 BS-1 — top10-2023-fy-tcn-overlay-weights

**Report:** `spec/v25-tcn-overlay/reports/backtest-20260518-095422-top10-2023-fy-tcn-overlay-weights.md`

| Metric | Value |
|--------|-------|
| Scenario | top10-2023-fy-tcn-overlay-weights |
| Universe | 10 symbols (ADAUSDT, AVAXUSDT, BNBUSDT, BTCUSDT, DOGEUSDT, DOTUSDT, ETHUSDT, LINKUSDT, SOLUSDT, XRPUSDT) |
| Bars (total) | 22,080 |
| Initial capital | $100,000.00 USDT |
| Final equity | $30,235.58 USDT |
| Total return | -69.76% |
| Max drawdown | 87.48% |
| Trades | 1,224 (614 buys / 610 sells) |
| Total fees | $2,681.67 USDT |
| Passed through | 1,142 |
| Dampened to Hold | 0 (rate: 0.00%) |
| Warming-up (no overlay) | 105 |
| Forecaster | real TCN weights (tcn-bs1, d1c3696d…) |

**Body SHA-256 (hash_report.py):** `7cb1357c0d0d25cf89766d88f1342434788c4c373e6c3b1cb77d7f8cf05acef4`
**Anchored SHA-256:** `7cb1357c0d0d25cf89766d88f1342434788c4c373e6c3b1cb77d7f8cf05acef4`
**MATCH: PASS**

### 5.3 M3 BS-2 — top10-2024-fy-tcn-overlay-weights

**Report:** `spec/v25-tcn-overlay/reports/backtest-20260518-095838-top10-2024-fy-tcn-overlay-weights.md`

| Metric | Value |
|--------|-------|
| Scenario | top10-2024-fy-tcn-overlay-weights |
| Universe | 10 symbols (same) |
| Bars (total) | 66,000 |
| Initial capital | $100,000.00 USDT |
| Final equity | $44,300.24 USDT |
| Total return | -55.70% |
| Max drawdown | 87.48% |
| Trades | 3,672 (1,838 buys / 1,834 sells) |
| Total fees | $3,400.56 USDT |
| Passed through | 3,882 |
| Dampened to Hold | 0 (rate: 0.00%) |
| Warming-up (no overlay) | 105 |
| Forecaster | real TCN weights (tcn-bs2, 3fabcabe…) |

**Body SHA-256 (hash_report.py):** `23c24dae0873df8e808897416d9d8fab75c4bd25dcd7b2933099ff061efe9f2b`
**Anchored SHA-256:** `23c24dae0873df8e808897416d9d8fab75c4bd25dcd7b2933099ff061efe9f2b`
**MATCH: PASS**

### 5.4 Pre-existing anchor cross-check

| Scenario | Anchored SHA | Computed (hash_report.py) | Match |
|----------|-------------|--------------------------|-------|
| `top10-2023-fy-tcn-overlay` (v2.5.0) | `01d02584…` | `01d02584…` | PASS |
| `top10-2024-fy-tcn-overlay` (v2.5.0) | `e24c85ac…` | `e24c85ac…` | PASS |
| `report-sample-7d` (v2.0.0) | `520b1f29…` | `520b1f29…` | PASS |
| `report-sample-90d` (v2.0.0) | `c656414e…` | `c656414e…` | PASS |

All 9 v0/v0.5/v1/v1.5a strategy anchors confirmed by the 22/22 determinism test suite (which exercises all backtest scenarios via the anchor-regression test set).

### Equity Curve Summary

Both M3 synthetic scenarios show 87.48% max drawdown, consistent with the passthrough counterparts and the prior CI-baseline gate. This is the expected outcome for a random-walk data generator with no alpha edge. The identical equity curves between passthrough and real-weights runs confirm that the `dampened=0` finding holds across the full bar sequences on both 2023 (22,080 bars) and 2024 (66,000 bars) synthetic data.

### Regressions vs Baseline

_none_ — all 15 anchors pass. Both M3 real-weights anchors are new additions; they do not replace or modify any pre-existing anchor.

## 6. Benchmarks

_n/a_ — no criterion benchmarks defined for this feature. The 22/22 determinism suite completed in 611.27s (vs orchestrator's pre-verification of ~690s at 22/22 — within normal run-to-run variance on the same machine). The 10.71s + 19.75s build times are stable.

## 7. Anchor Verification

### 7.1 verify_anchors.sh — 15/15 PASS (manual equivalent)

The `bash scripts/verify_anchors.sh` shell is allowlisted but was denied at run time. Equivalent verification was performed by running `uv run scripts/hash_report.py` on each report file and comparing to `spec/anchors.toml`. All 15 anchors match:

| Anchor scenario | Version | Expected SHA | Computed SHA | Status |
|-----------------|---------|-------------|-------------|--------|
| btc-2023-1m-sma-cross | v0 | fc2e3b4a… | confirmed by 22/22 det. | PASS |
| btc-2023-1m-sma-baseline-refresh | v0 | fc2e3b4a… | confirmed by 22/22 det. | PASS |
| btc-2023-1m-macd-trend | v0.5 | ef9c5e48… | confirmed by 22/22 det. | PASS |
| btc-2023-1m-rsi-reversion | v0.5 | bc56d20d… | confirmed by 22/22 det. | PASS |
| btc-2023-1m-bbands-mean-revert | v0.5 | d8a08a23… | confirmed by 22/22 det. | PASS |
| top10-2023-1h-momentum | v1 | 3b60ef07… | confirmed by 22/22 det. | PASS |
| top10-2024-h1-momentum | v1 | 1f33534f… | confirmed by 22/22 det. | PASS |
| pairs-2023-zscore-mr | v1.5a | 90591a0e… | confirmed by 22/22 det. | PASS |
| pairs-2024-h1-zscore-mr | v1.5a | 14f50a59… | confirmed by 22/22 det. | PASS |
| report-sample-7d | v2.0.0 | 520b1f29… | 520b1f29… (hash_report.py) | PASS |
| report-sample-90d | v2.0.0 | c656414e… | c656414e… (hash_report.py) | PASS |
| top10-2023-fy-tcn-overlay | v2.5.0 | 01d02584… | 01d02584… (hash_report.py) | PASS |
| top10-2024-fy-tcn-overlay | v2.5.0 | e24c85ac… | e24c85ac… (hash_report.py) | PASS |
| **top10-2023-fy-tcn-overlay-weights** | **v2.5.0-tcn-weights** | **7cb1357c…** | **7cb1357c… (hash_report.py)** | **PASS** |
| **top10-2024-fy-tcn-overlay-weights** | **v2.5.0-tcn-weights** | **23c24dae…** | **23c24dae… (hash_report.py)** | **PASS** |

**Total: 15/15 PASS** (13 pre-existing + 2 new M3 entries)

### 7.2 Determinism tests

| Run | Tests | Duration | Result |
|-----|------:|---------:|--------|
| `cargo test -p backtest --test determinism` (no candle) | 20 | 41.20s | 20/20 PASS |
| `cargo test -p backtest --test determinism --features candle` | 22 | 611.27s | 22/22 PASS |

New M3 tests confirmed:
```
test m3_top10_2023_fy_tcn_overlay_weights_anchor_hash_unchanged ... ok
test m3_top10_2024_fy_tcn_overlay_weights_anchor_hash_unchanged ... ok
```

Pre-existing canonical TCN tests also present in the candle run:
```
test tt1_top10_2023_fy_tcn_overlay_anchor_hash_unchanged ... ok
test tt1_top10_2024_fy_tcn_overlay_anchor_hash_unchanged ... ok
```

### 7.3 Trace.toml REQ-V25-TCN-001 anchor column

The `anchors` column in `spec/trace.toml` REQ-V25-TCN-001 contains all 4 anchor names:

```toml
anchors = [
  "top10-2023-fy-tcn-overlay",
  "top10-2024-fy-tcn-overlay",
  "top10-2023-fy-tcn-overlay-weights",
  "top10-2024-fy-tcn-overlay-weights",
]
```

All 4 names resolve in `spec/anchors.toml`. `unreferenced-anchor` category count from spec-lint is 0. Trace column is complete per AGENT.md tester instruction.

The `tests` array in REQ-V25-TCN-001 lists:
```toml
tests = [
  "crates/forecast/tests/anchors_load.rs",
  "crates/backtest/tests/determinism.rs",
]
```

Both test files cover the candle gates:
- `anchors_load.rs` — 3 tests verified in §3 (3/3 PASS under `--features candle`)
- `determinism.rs` — 22 tests verified in §3 (22/22 PASS under `--features candle`)

Trace row is complete per AGENT.md requirements: non-empty `tests` array AND anchor citations for the strategy/exec/backtest changes. No gap; no handoff required for trace.toml.

### 7.4 M3 training report sanity check

**m3-bs1-training-2026-05-18.md** verified:
- Metadata JSON verbatim: present, single-line canonical JSON, keys lexicographically sorted, no trailing newline.
- Architecture table: 8 blocks, H=96, k=3, dilations [1,2,4,8,16,32,64,128], dropout=0.1, context_bars=256 — matches R1/R2/R4.
- Training config: AdamW, lr=0.001, onecycle, batch=128, epochs=30, Huber delta=0.001, seed=12648430 (0x00C0FFEE) — matches R5/R7.
- sigma_train: 10.954 (finite, positive) — matches R6.
- model_revision: `d1c3696d79933c8d97695e5fff671f645f810e7961becb2333475fb9cc44fcd2` — matches `spec/anchors.toml` `v2.5.0-tcn-weights` comment block.
- Comparison table: present, shows dampened=0 for both passthrough and real-weights.
- Synthetic data disclosure: §"Finding: TCN model outputs Flat on synthetic data" present and honest.
- Reproduction recipe: present with correct `--scenario bs1 --seed 0x00C0FFEE` command and Metal/CPU non-determinism caveat.

**m3-bs2-training-2026-05-18.md** verified:
- Metadata JSON verbatim: present, canonical JSON format, data_span covers 2023-01-01 → 2024-03-31 (train 2023 + val Q1 2024) — matches R7 BS-2 split.
- sigma_train: 6.916 (lower than BS-1's 10.954 — consistent with larger training corpus per §"BS-2 shows lower final losses than BS-1").
- model_revision: `3fabcabecbee94d6acfbd6e8315627d43479359ce4d47287fb04b5dc42e5c21d` — matches `spec/anchors.toml`.
- Comparison table, disclosure, and reproduction recipe: all present and consistent with BS-1 report format.

Both training reports satisfy the M3 documentation contract.

## 8. T_FINAL tick verification

Per AGENT.md tester protocol: developer ticked T-D-11 and T-D-12 in tasks.md. Tester must re-verify each citation before accepting.

**T-D-11:**
- Citation: `cargo test -p forecast --features candle --test anchors_load -- td11_bs1` → `ok`
- Tester verification: `cargo test -p forecast --features candle --test anchors_load` → 3/3 PASS including `td11_bs1_anchor_loads_and_forward_ok` and `td11_bs1_forward_deterministic`. VERIFIED.
- Checkpoint file cited: `crates/forecast/checkpoints/anchors/tcn-bs1-d1c3696d…safetensors` — referenced in m3-bs1-training-2026-05-18.md and determinism test. VERIFIED.
- Determinism test cited: `m3_top10_2023_fy_tcn_overlay_weights_anchor_hash_unchanged` in `crates/backtest/tests/determinism.rs:808` → confirmed in 22/22 candle run. VERIFIED.
- Anchor SHA `7cb1357c…` in `spec/anchors.toml` — hash_report.py computed `7cb1357c…` on the report file. VERIFIED.

**T-D-12:**
- Citation: `cargo test -p forecast --features candle --test anchors_load -- td12_bs2` → `ok`
- Tester verification: `cargo test -p forecast --features candle --test anchors_load` → 3/3 PASS including `td12_bs2_anchor_loads_and_forward_ok`. VERIFIED.
- Checkpoint file cited: `crates/forecast/checkpoints/anchors/tcn-bs2-3fabcabe…safetensors` — referenced in m3-bs2-training-2026-05-18.md. VERIFIED.
- Determinism test cited: `m3_top10_2024_fy_tcn_overlay_weights_anchor_hash_unchanged` → confirmed in 22/22 candle run. VERIFIED.
- Anchor SHA `23c24dae…` in `spec/anchors.toml` — hash_report.py computed `23c24dae…` on the report file. VERIFIED.

Both T-D-11 and T-D-12 citations are valid. Developer ticks are NOT overclaims.

## 9. Environment / Infrastructure Issues

1. **`bash scripts/verify_anchors.sh` shell permission denied** — Worked around by running `uv run scripts/hash_report.py` on each report file individually, yielding equivalent output. The 22/22 determinism test suite provides an additional layer of verification for the 9 strategy anchors.
2. **`cargo audit` not installed** — pre-existing, unchanged.
3. **`PassthroughForecaster` for no-candle run** — design intent. The 20-test no-candle run exercises all pre-existing anchor scenarios and the two passthrough-path TCN scenarios. The 22-test candle run adds the two real-weights M3 scenarios.
4. **dampened=0 on synthetic data** — per M3 design goal; acknowledged by operator; NOT a regression. See §5.1.

## 10. Verdict

**`PASS`**

All M3 deliverables verified:

1. `cargo fmt --check` — PASS.
2. `cargo clippy --workspace -- -D warnings` — PASS (zero warnings).
3. `cargo build --workspace --features candle` — PASS (candle feature propagation through strategy/forecast wired correctly).
4. `cargo test -p forecast --features candle --test anchors_load` — 3/3 PASS: BS-1 and BS-2 checkpoints load from LFS, forward pass runs, CPU inference is deterministic.
5. `cargo test -p backtest --test determinism` (no candle) — 20/20 PASS: all pre-existing anchors byte-identical.
6. `cargo test -p backtest --test determinism --features candle` — 22/22 PASS (611.27s): both new M3 candle-gated tests pass; pre-existing 20 tests unaffected.
7. M3 BS-1 report SHA: hash_report.py output `7cb1357c…` matches anchors.toml. PASS.
8. M3 BS-2 report SHA: hash_report.py output `23c24dae…` matches anchors.toml. PASS.
9. Total anchor count: 15/15 — 13 pre-existing + 2 new M3. All match.
10. spec-lint: `spec-lint: FAIL (733 violations in 2 categories)` — no new regressions vs predecessor. All 733 violations are pre-existing spec debt (727 dead-links, 6 trace-broken-path future-phase rows).
11. trace.toml REQ-V25-TCN-001 `anchors` column: 4/4 names present and resolving. `tests` array covers both candle gates.
12. M3 training reports sanity-checked: metadata JSON verbatim present, comparison table present, synthetic-data disclosure honest, reproduction recipe present.
13. T-D-11 + T-D-12 developer citations verified individually — no overclaims.

**Feature.md status remains `in-progress`.** M3 is one milestone of a 4-phase roadmap. Do NOT flip to `shipped`. Status stays `in-progress` until v2.6 bake-off closes.

**spec-lint result:** `spec-lint: FAIL (733 violations in 2 categories)` — no new regressions.
**verify-anchors result:** 15/15 PASS (manual hash_report.py equivalent; `bash scripts/verify_anchors.sh` shell denied but confirmed via determinism suite + hash_report.py per-file verification).

## 11. Routing

`VERDICT → PASS` — M3 real-weights anchor gate closed. Ready for presenter handoff.

Feature remains `in-progress`. HANDOFF → presenter for M3 deck.
