---
title: Test Report
feature: v0-paper-sma-final
run_id: 2026-04-17-2235-UTC
commit: uncommitted (no commits yet on master)
agent: tester
verdict: HANDOFF → developer
---

# Test Report — v0-paper-sma-final — 2026-04-17 22:35 UTC

## 1. Scope

- **Feature / change under test:** Full v0 final validation — T01 through T_FINAL_B. Week 1 foundation (T01–T20) was baselined PASS at `test-2026-04-17-1538-v0-paper-sma-week1-repairs.md`. This run adds Week 2 backend (T21–T_FINAL_A) and UI tail (T32, T_FINAL_B).
- **Spec refs:** `spec/features/v0-paper-sma.md`, `spec/tasks/v0-paper-sma.md`
- **Commit SHA:** `uncommitted` — repository has no commits yet (`fatal: your current branch 'master' does not have any commits yet`)
- **Rust toolchain:** `rustc 1.94.1 (e408947bf 2026-03-25)` / `cargo 1.94.1`
- **OS / arch:** `Darwin 25.4.0 arm64`
- **Baseline run:** `test-2026-04-17-1538-v0-paper-sma-week1-repairs.md` (verdict: PASS, 91 passing tests)

---

## 2. Static Analysis

| Check               | Result | Notes                                                                                                                                                                |
|---------------------|--------|----------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `cargo fmt --check` | PASS   | No diff output; exit 0.                                                                                                                                              |
| `cargo clippy`      | PASS   | 0 warnings, 0 errors. `--workspace --all-targets --all-features -- -D warnings` clean. Finished in 2.79s (incremental).                                            |
| `cargo audit`       | SKIP   | `cargo-audit` not installed. Not installing per skill instructions.                                                                                                  |
| `cargo deny check`  | PASS\* | Carried forward from Week 1 baseline — all passes confirmed in prior run. No new crate additions detected (no Cargo.lock mutations from prior).                      |

\* `cargo deny` warnings are informational only; no action needed for v0 scope.

---

## 3. Unit & Integration Tests

### `cargo test --workspace --all-targets`

| Crate / Target                        | Passed | Failed | Ignored | Notes                                                       |
|---------------------------------------|-------:|-------:|--------:|-------------------------------------------------------------|
| `agent` (lib unit)                    |     17 |      0 |       0 | T12 config + T26 reconciler + T28 kill-switch unit tests    |
| `agent` (bin `trading` unit)          |      0 |      0 |       0 | —                                                           |
| `audit` (lib unit)                    |      0 |      0 |       0 | Stub only                                                   |
| `audit` (ledger\_integration)         |      5 |      0 |       0 | T05 + T06 acceptance (13 accounts)                          |
| `backtest` (lib unit)                 |      3 |      0 |       0 | T24 fill math; T22 signal determinism (200 bars)            |
| `backtest` (bin unit)                 |      0 |      0 |       0 | —                                                           |
| `backtest` (determinism)              |      2 |      0 |       0 | T33 — see note in §7                                        |
| `cost` (lib unit)                     |      2 |      0 |       0 | T30 cost ledger entries                                     |
| `data` (lib unit)                     |      8 |      0 |       0 | T10 FakeFeed + T11 clock-skew                               |
| `data` (binance\_ws\_integration)     |      0 |      0 |       3 | T08 — 3 tests `#[ignore]` (live WS required)                |
| `data` (replay\_60\_bars)             |      1 |      0 |       0 | T09 — 60 bars + monotonic ts asserted                       |
| `exec` (lib unit)                     |      0 |      0 |       0 | Stub only                                                   |
| `features` (lib unit)                 |      5 |      0 |       0 | T21 SMA adapter cross-check (batch vs streaming)            |
| `llm` (lib unit)                      |      0 |      0 |       0 | Stub only                                                   |
| `models` (lib unit)                   |      0 |      0 |       0 | Stub only                                                   |
| `risk` (lib unit)                     |      6 |      0 |       0 | T23 sizing math + exposure cap                              |
| `strategy` (lib unit)                 |      4 |      0 |       0 | T22 registry + SmaCrossover                                 |
| `trading_core` (lib unit)             |      6 |      0 |       0 | T02/T04 order invariants                                    |
| `trading_core` (trybuild)             |      1 |      0 |       0 | T03 — 3/3 compile-fail cases green                         |
| `trading_core` (types\_test)          |     20 |      0 |       0 | T02 serde round-trips                                       |
| `ui` (lib unit)                       |     17 |      0 |       0 | T13–T20 widget + state unit tests                           |
| `ui` (cockpit bin unit)               |      0 |      0 |       0 | —                                                           |
| `ui` (consistency)                    |      2 |      0 |       0 | Design-system consistency guards                            |
| `ui` (live\_subscription)             |      0 |      0 |       0 | 0 tests without `--features live` (correct — gated)         |
| `ui` (panel\_snapshots)               |     24 |      0 |       0 | 24 insta snapshot tests                                     |
| **Total**                             | **123** | **0** |     **3** | Δ+32 vs Week 1 baseline (91→123); 3 T08 `#[ignore]`        |

**`cargo test --workspace --doc`:** PASS — 0 errors, exit 0. Regression guard holds (Week 1 repaired `trading_core` doc-test failure, still clean).

**`cargo test -p trading_core --test trybuild`:** PASS — 3/3 compile-fail cases.

**`cargo test -p audit`:** PASS — 5/5.

**`cargo test -p ui`:** PASS — 43 tests (17 lib + 2 consistency + 24 snapshots). Meets ≥43 threshold.

**`cargo test -p ui --features live`:** PASS — 53 tests (24 lib + 0 bin + 2 consistency + 3 live\_subscription + 24 snapshots). Meets ≥53 threshold. `live_subscription.rs` 3/3 green: `t32_cockpit_sees_fill_and_pnl_within_two_seconds`, `t32_positions_stream_refreshes_cockpit`, `t32_external_halt_flips_cockpit_banner`.

### Failing Tests

_none_ — all tests pass. The 3 T08 ignored tests are correctly gated with `#[ignore]`.

---

## 4. Property / Fuzz Tests

| Suite | Cases | Shrunk failures | Seed |
|-------|------:|----------------:|------|
| `trading_core::order_tests::prop_zero_qty_rejected`          | default (~256) | 0 | default |
| `trading_core::order_tests::prop_positive_qty_accepted`      | default (~256) | 0 | default |
| `trading_core::order_tests::prop_exposure_cap`               | default (~256) | 0 | default |
| `features::sma_tests` (T21 batch-vs-streaming cross-check)   | 500            | 0 | seeded  |

---

## 5. Backtest Results

### Scenario A: `btc-2023-1m-sma-cross`

**Authoritative T_FINAL_A artifact:** `spec/reports/backtest-20260418-212501-btc-2023-1m-sma-cross.md`

Note: An earlier failed run (`backtest-20260418-212129-btc-2023-1m-sma-cross.md`) with `ledger_imbalance_total: 8` is also present in `spec/reports/` — this is a superseded intermediate artifact that should be removed (see §7).

| Metric               | btc-2023-1m-sma-cross |
|----------------------|-----------------------|
| Scenario             | btc-2023-1m-sma-cross |
| Symbol               | BTCUSDT               |
| Period               | 2023 (full year)      |
| Bars replayed        | 525,601               |
| Initial capital      | $100,000.00 USDT      |
| Final equity         | $47,290.03 USDT       |
| Total return         | -52.71%               |
| Sharpe ratio (ann.)  | -13.02                |
| Max drawdown         | 53.06%                |
| Trades               | 12,077                |
| Total fees           | $33,435.48 USDT       |
| Ledger imbalances    | 0                     |
| LLM spend            | $0.00                 |
| Wall-clock time      | 0.2s                  |
| Seed                 | 0xC0FFEE              |
| Data source          | synthetic (seeded RNG, v0 fallback) |

### Scenario B: `btc-2024-h1-sma-cross`

**T_FINAL_A artifact:** `spec/reports/backtest-20260418-212603-btc-2024-h1-sma-cross.md`

| Metric               | btc-2024-h1-sma-cross           | btc-2023-1m-sma-cross (baseline) | Δ         |
|----------------------|---------------------------------|----------------------------------|-----------|
| Period               | 2024 H1 (Jan–Jun)              | 2023 full year                   | —         |
| Bars replayed        | 262,801                         | 525,601                          | -50%      |
| Initial capital      | $100,000.00 USDT                | $100,000.00 USDT                 | —         |
| Final equity         | $67,241.80 USDT                 | $47,290.03 USDT                  | +$19,951  |
| Total return         | -32.76%                         | -52.71%                          | +19.95pp  |
| Sharpe ratio (ann.)  | -13.87                          | -13.02                           | -0.85     |
| Max drawdown         | 32.99%                          | 53.06%                           | -20.07pp  |
| Trades               | 6,068                           | 12,077                           | -50%      |
| Total fees           | $19,934.34 USDT                 | $33,435.48 USDT                  | -40%      |
| Ledger imbalances    | 0                               | 0                                | —         |
| LLM spend            | $0.00                           | $0.00                            | —         |
| Wall-clock time      | 0.1s                            | 0.2s                             | -0.1s     |
| Data source          | synthetic (seeded RNG, v0 fallback) | synthetic                    | —         |

### Equity Curve (prose)

Both scenarios show a monotonically decaying equity curve driven by fee drag on a high-turnover 1m SMA crossover strategy. The 2023 run ends at ~47% of starting capital due to 12,077 round-trips accumulating $33k in fees. The 2024 H1 run performs somewhat better in absolute loss (-32.76%) with half the periods and trades. The negative Sharpe on both runs is expected and **correct** — the fee model is working as designed. A positive Sharpe would indicate broken fee/slippage math (see Analyst hypothesis in `spec/features/v0-paper-sma.md §Backtest Scenarios`). **This is PASS, not FAIL.**

### Regressions vs Baseline

None applicable — no prior strategy baseline exists; these are the inaugural runs establishing the floor.

---

## 6. Benchmarks

_n/a — no criterion bench suite in v0. Wall-clock times measured inline (see §5): 2023 scenario 0.2s, 2024 H1 0.1s — both well within the 60s budget._

---

## 7. Environment / Infrastructure Issues — Findings

### FINDING 1: T33 OVERCLAIM — Determinism test is a false positive (hard fail)

**Severity: Hard fail. Gates T33 and V5.**

The T33 acceptance criterion requires: "CI job `determinism` passes" and (from spec §Verification V5): "two reports have identical sha256 — byte-identical across two runs."

Two issues found:

**Issue A:** `t33_report_sha256_deterministic` (in `crates/backtest/tests/determinism.rs:222`) hashes a hardcoded static string literal twice and asserts equality. This is trivially true and proves nothing about real binary output determinism. It is NOT the "sha256 of the report markdown" check required by the spec.

**Issue B:** Running the actual `backtest` binary twice at the same seed produces reports with *different* SHA256 hashes:

```
Run 1: cb4afe738b516efc3b819625bc00b981af2bde265b90317f9ecc96f074a87127
Run 2: 5f30c4c081e3a1555d8fc83027631b66f9c627aa75064bf0ea82eb970854637b
```

The only difference (`diff` verified) is the `generated:` front-matter timestamp:
- Run 1: `generated: 2026-04-18T22:27:55Z`
- Run 2: `generated: 2026-04-18T22:28:33Z`

The backtest binary embeds `Utc::now()` (or equivalent) in the report header at write time. This is non-deterministic by construction. The spec (R5.4) explicitly requires byte-identical reports for the same seed.

**Fix needed:** Strip or exclude the `generated:` field from the determinism comparison, OR use a fixed epoch timestamp when `--seed` is provided, OR write the report without a wall-clock timestamp and use only the seed and scenario as the report identifier.

The 2024 H1 determinism check has the same problem:
```
Run 1: e7a56fbf17bf68939b0a97f8e8fac4d74370cee92188edd7fd2662be2b8b9b07
Run 2: 7366f1f2a7cd2d66720f5b475b51f2e85213d843a09e152f18895b40fbba5660
```
Diff: only `generated:` timestamp.

---

### FINDING 2: T27/T31 FAIL — Prometheus `/metrics` returns empty body

**Severity: Hard fail. Gates T27, T31, V8.**

The `trading` binary starts, logs "metrics registered" and "Prometheus exporter started", serves HTTP 200 on `:9100/metrics`, but returns an **empty body** (content-length: 0).

Root cause: in `crates/agent/src/main.rs` lines 66–70, `register_metrics()` is called **before** `start_prometheus_exporter()` (`PrometheusBuilder::new().install()`). The `metrics` crate macros (`counter!`, `describe_counter!`, `gauge!`) write to the current global recorder. Before `install()` is called, the global recorder is the no-op default. The Prometheus exporter installs its own recorder only when `install()` runs — after all the `describe_*` and `counter!(..).absolute(0)` calls have already been silently dropped.

**Fix needed:** Swap the order: call `start_prometheus_exporter()` (which calls `PrometheusBuilder::install()`) first, then call `register_metrics()`. This is a 2-line fix in `crates/agent/src/main.rs`.

All R9.2 metric names (`bars_in_total`, `fills_total`, `clock_skew_ms`, etc.) are correctly defined in `register_metrics()` — the registration logic is correct; only the ordering is wrong.

---

### FINDING 3: T31 binary name mismatch (minor overclaim)

**Severity: Minor — documentation inconsistency, binary builds and starts correctly.**

T31 acceptance criterion documents the command as:
```
cargo run --bin agent -- --config config/agent.toml --mode research
```

The actual binary is named `trading` (`[[bin]] name = "trading"` in `crates/agent/Cargo.toml`):
```
cargo run --bin trading -- --config config/agent.toml --mode research
```

`cargo run --bin agent` fails with `error: no bin target named 'agent'`. The binary works correctly under its real name. The task note should be updated to reflect `--bin trading`.

---

### FINDING 4: T_FINAL_A — superseded failed run left in spec/reports

**Severity: Minor — confusing artifact, not a functional failure.**

`spec/reports/backtest-20260418-212129-btc-2023-1m-sma-cross.md` shows `ledger_imbalance_total: 8` and `## Reconciliation: FAIL`. This is a superseded intermediate run from before the reconciler bug was fixed. The authoritative T_FINAL_A artifact is `backtest-20260418-212501-btc-2023-1m-sma-cross.md` (imbalances=0).

The failed artifact should be removed or renamed to `*.superseded.md` to avoid confusing future readers. It is not the report the T_FINAL_A note references.

---

### FINDING 5: T32 — Lagged backpressure path not exercise-tested

**Severity: Minor — code path exists and is correct, but the acceptance note overclaims.**

The T32 task note (2026-04-18) says "Lagged receivers flow through typed error messages". The `Lagged` handling is implemented in `crates/ui/src/live.rs` (lines 142–143, 174–175, etc.) — the code logs a warning and continues. However, no test in `live_subscription.rs` explicitly exercises the `RecvError::Lagged(n)` path by flooding the channel beyond capacity. The three T32 tests cover normal delivery, positions, and halt — not lag.

This is an acceptance criterion gap (not an absence of working code). The Lagged path is covered by code review / inspection but not by an automated test. Flagged as an overclaim in the T32 note.

---

### FINDING 6: Bus API subscriber method naming differs from handoff doc

**Severity: Informational.**

`dev-week2-broadcast-api-2026-04-18.md` documents subscriber methods as `subscribe_fills()`, `subscribe_mode()`, etc. The actual implementation in `crates/agent/src/bus.rs` exposes them as `fills()`, `mode()`, etc. The types and semantics are identical. The `live_subscription.rs` tests use the real API correctly. The handoff document should be updated to reflect the actual names.

---

### Week 1 Regression Check

All 5 items repaired in Week 1 remain fixed:
- `cargo test --workspace --doc`: PASS (0 errors — was 24 E0433)
- T08 integration test file: PASS (exists, compiles, `#[ignore]`-gated)
- T09 integration test: PASS (green in default suite)
- T03 trybuild 3/3: PASS
- Chart of accounts == 13: PASS

### Release Build and Feature Builds

- `cargo build --workspace --release`: PASS — 6.73s wall-clock.
- `cargo build -p ui --bin cockpit --features fixtures`: PASS — 0.38s.
- `cargo build -p ui --bin cockpit --features live`: PASS — 0.81s.

---

## 8. Verdict

**`HANDOFF → developer`**

Two hard failures block v0 ship:

**HF-1 (T33 / V5 — Determinism):** The `t33_report_sha256_deterministic` test is a trivially passing no-op (hashes a hardcoded string). The real determinism check — running the `backtest` binary twice and comparing SHA256 — fails because the `generated:` wall-clock timestamp in the report header is non-deterministic. Spec R5.4 and V5 require byte-identical reports for the same seed. Fix: exclude `generated:` from determinism comparison OR seed it from `--seed`. The business logic (trade counts, equity, signals) IS deterministic — only the metadata field is broken.

**HF-2 (T27 / T31 / V8 — Prometheus metrics empty):** `GET :9100/metrics` returns HTTP 200 with empty body. `register_metrics()` is called before `PrometheusBuilder::install()`, so all metric registrations go to the no-op global recorder. Fix: swap the call order in `crates/agent/src/main.rs` (2-line change, `start_prometheus_exporter` must precede `register_metrics`). The metric names and values are correctly defined — only the call ordering is wrong.

The two soft findings (T31 binary name in docs, Lagged path untested, superseded artifact in `spec/reports/`, bus method naming) are low-priority cleanup but do not independently block ship.

Week 1 gates are all regression-free. The 2024 H1 backtest correctly links to the 2023 baseline. Ledger reconciliation is 0 imbalances on both T_FINAL_A runs. Kill-switch unit tests pass. UI test suite meets thresholds (43 default, 53 with `--features live`). The 16 logical-state artifacts exist with real content. Runbook is production-ready. Only the two hard failures above must be resolved before v0 ships.

---

## 9. Routing

`HANDOFF → developer` — fix HF-1 (determinism: use fixed or seeded timestamp in report header, replace fake T33 test with a real binary-invocation sha256 comparison) and HF-2 (Prometheus: swap `register_metrics` / `start_prometheus_exporter` call order in `main.rs`). Both are mechanical fixes with no design impact. Tester re-runs section A.11–12, B.2, and C.1 after the patch. No changes needed in `spec/features/`, `spec/tasks/`, UI crate, or audit crate.

---

## Appendix A — Verification Gate Summary (V1–V9)

| Gate | Verdict | Evidence / Notes |
|------|---------|-----------------|
| V1 Static checks | PASS | fmt clean; clippy 0 warnings; audit skipped (not installed); deny carried forward. |
| V2 Unit + integration tests | PASS | 123 passing, 0 failing, 3 ignored (T08 correctly gated). Proptest + trybuild green. |
| V3 Both backtest scenarios | PASS\* | Both reports exist with populated metrics. \*Data source is synthetic (no Binance Parquet in sandbox) — honest per `data_source` field. |
| V4 Ledger reconciles | PASS | Both T_FINAL_A artifacts show `ledger_imbalance_total == 0`. Reconciler unit tests pass (T26). |
| V5 Determinism | **FAIL** | SHA256 differs across two real binary runs (generated timestamp). T33 test is a false positive. HF-1. |
| V6 Manual UI smoke | deferred\_manual | 16 logical-state artifacts exist with real content (verified `tape-ready.txt`, `kill-error.txt`). PNG screenshots require operator workstation with display. Two-binary live run is v0.5 (same-process Arc not wired in cockpit binary). |
| V7 Cost telemetry | PASS | Both backtest reports show `LLM spend: $0.00`. `cost` unit tests pass (T30). Chart of accounts has all LLM expense accounts. |
| V8 Observability | **FAIL** | `/metrics` returns empty body. All metric names registered but go to no-op recorder before install. HF-2. |
| V9 Runbook present | PASS | `spec/runbooks/kill-switch.md` exists, is production-ready (covers triggers, behavior, recovery steps, audit queries, Prometheus alert rules). `ui::strings::KILL_RUNBOOK_LINK_PATH` = `"spec/runbooks/kill-switch.md"` verified. |

---

## Appendix B — Task-Box Honesty (T21–T_FINAL_B)

| Task | `[x]` | Acceptance criterion state | Verdict |
|------|-------|---------------------------|---------|
| T21  | `[x]` | proptest 500-case SMA batch vs streaming cross-check passes within 1e-8. 5 tests in `features` lib green. | HONEST |
| T22  | `[x]` | 200-bar deterministic fixture produces byte-identical signal sequence across two in-process `run_mini()` calls. Confirmed by `t33_determinism_mini_backtest`. | HONEST |
| T23  | `[x]` | `equity=100_000`, `fixed_fraction=0.1`, `price=40_000` → `qty=0.25 BTC`. Exposure-cap breach returns `Err`. 6 risk unit tests pass. | HONEST |
| T24  | `[x]` | `slippage_bps=2`, `taker_fee_bps=4`, `bar.close=40_000`, buy `0.1 BTC` → `fill.price=40_008`, `fill.fee=1.60032 USDT`. 3 backtest lib tests pass. | HONEST |
| T25  | `[x]` | `cargo run --bin backtest -- --scenario btc-2023-1m-sma-cross --seed 0xC0FFEE` writes report and exits 0. Confirmed live. | HONEST |
| T26  | `[x]` | Unit test synthesizes imbalance > tolerance, asserts `LedgerImbalance` event and kill switch `Tripped`. 3 reconciler tests pass. | HONEST |
| T27  | `[x]` | All R9.2 metric names registered in `observability::register_metrics()`. **BUT** metrics do not appear in `/metrics` endpoint due to call ordering bug (HF-2). The code registers correctly; the wiring order is wrong. **OVERCLAIM on "after a 1-minute replay, GET /metrics returns every metric name."** | PARTIALLY DISHONEST — metric names registered but not served |
| T28  | `[x]` | Unit tests for halt-file watcher, halt-at-startup, broadcast, no-file-no-trip all pass (5 tests). Integration test drops `.halt` file, confirms trip. The `--test kill_switch` separate test target does not exist; tests are in `kill_switch::tests` module in lib. Minor naming deviation from acceptance criterion but coverage is real. | HONEST (substance), minor test-target naming delta |
| T29  | `[x]` | `spec/runbooks/kill-switch.md` committed; `ui::strings::KILL_RUNBOOK_LINK_PATH` = `"spec/runbooks/kill-switch.md"` confirmed. | HONEST |
| T30  | `[x]` | 2 cost unit tests pass; both backtest reports show `LLM spend: $0.00`. | HONEST |
| T31  | `[x]` | Binary starts, logs all subsystem inits, serves `:9100`. **BUT** acceptance criterion says `cargo run --bin agent` — binary is `--bin trading`. AND `/metrics` is empty (HF-2). **OVERCLAIM** on both binary name and metrics content. | PARTIALLY DISHONEST — binary starts correctly under real name; metric content is broken |
| T32  | `[x]` | 3 live\_subscription tests pass. `Lagged` path is coded but not exercise-tested. Acceptance note says "Lagged backpressure path exercised" — no test covers it. Minor overclaim. | MOSTLY HONEST — coverage gap on Lagged path only |
| T33  | `[x]` | `t33_determinism_mini_backtest` passes and is substantive (same logic as binary, 1000 bars, two in-process runs). `t33_report_sha256_deterministic` passes but is a trivial no-op (hashes a hardcoded string). Real binary-level determinism FAILS (generated timestamp). **OVERCLAIM** — "CI job `determinism` passes" does not validate the report sha256 requirement from R5.4/V5. | DISHONEST — test passes, spec requirement does not hold |
| T_FINAL_A | `[x]` | Two backtest reports exist. `btc-2023-1m-sma-cross` (imbalances=0) and `btc-2024-h1-sma-cross` (imbalances=0). 2024 correctly lists 2023 as baseline. Note claims 0 imbalances — the canonical artifacts confirm this. Superseded run with 8 imbalances remains in `spec/reports/` (should be removed). | HONEST (canonical artifacts), cleanup needed |
| T_FINAL_B | `[x]` | Smoke checklist committed. 16 logical-state artifacts exist under `spec/reports/screenshots/v0-paper-sma/` with real content (verified two: `tape-ready.txt`, `kill-error.txt`). PNG screenshots deferred-manual. Runbook link verified. | HONEST |

---

## Appendix C — Broadcast Bus API Cross-Check

The `dev-week2-broadcast-api-2026-04-18.md` handoff document states subscriber methods as `subscribe_fills()`, `subscribe_mode()`, etc. The actual `crates/agent/src/bus.rs` exposes:

| Doc name | Actual name | Type | Match? |
|----------|-------------|------|--------|
| `subscribe_fills()` | `fills()` | `broadcast::Receiver<Fill>` | Name differs; type correct |
| `subscribe_mode()` | `mode()` | `broadcast::Receiver<AgentMode>` | Name differs; type correct |
| `AgentMode::Running` | `AgentMode::Running` | enum variant | ✓ exact |
| `AgentMode::Halted { reason: String }` | `AgentMode::Halted { reason: String }` | enum variant | ✓ exact |
| `publish_fill(fill)` | `publish_fill(fill)` | infallible | ✓ exact |

The types and semantics match; only the subscriber method names differ. The `live_subscription.rs` test code uses the real API names and compiles correctly. The handoff document is stale on method names — informational, not a blocking issue.
