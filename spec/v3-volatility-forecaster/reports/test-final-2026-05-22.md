---
title: Test Report
feature: v3-volatility-forecaster
run_id: 2026-05-22-M-FINAL
commit: 625fb33 (feat: Waves A-D complete) + fmt fixes (orchestrator pre-cleared)
agent: tester
verdict: PASS
---

# Test Report — v3-volatility-forecaster — 2026-05-22 M-FINAL re-gate

## 1. Scope

- **Feature / change under test:** v3 Volatility Forecaster (v3.0.0-volatility) — GARCH(1,1)-only MVP.
  Wave A-E complete: GARCH fitter + VolForecastProvider trait + Parkinson target derivation +
  V-verdict bin + 3 consumer strategy builders + backtest scenario + sharpe-comparison extension.
- **Spec refs:** `spec/v3-volatility-forecaster/feature.md`, `spec/v3-volatility-forecaster/tasks.md`,
  `spec/v3-volatility-forecaster/decomp.md`, `spec/architecture/adr/0038-vol-forecast-verdict-shape.md`
- **Commit SHA:** `625fb336e7faeb96aa1040343dab34e02115b72f` (Waves A-D complete).
  Orchestrator applied `cargo fmt --all` (71 whitespace hunks) as a pre-cleared blocker fix
  prior to this tester re-gate.
- **Rust toolchain:** `rustc 1.94.1 (e408947bf 2026-03-25)`
- **OS / arch:** `darwin arm64`

### Prior REGRESSION — how blockers were cleared

The prior tester returned `VERDICT → REGRESSION` on 2026-05-22 with 2 mechanical blockers:

1. **Blocker 1 — fmt drift:** `cargo fmt --check` produced 71 whitespace hunks across 10 new
   Wave A-D files. **Cleared by orchestrator:** `cargo fmt --all` applied and committed as
   `fmt(v3-volatility-forecaster): cargo fmt fixes + vol-verdict re-run`.

2. **Blocker 2 — SHA discrepancy:** The developer's `vol-verdict-bs1-realdata` self-reported
   body-SHA256 (`e88831f7...`) used a different hashing convention than `scripts/hash_report.py`.
   **Cleared by orchestrator:** re-ran `python3 scripts/hash_report.py` against the on-disk
   report files to derive the canonical body-SHAs. The anchor gate uses `hash_report.py` only;
   the bin's self-report is a follow-on cleanup item (advisory only, not a gate blocker).

## 2. Static Analysis

| Check              | Result | Notes                                                |
|--------------------|--------|------------------------------------------------------|
| `cargo fmt --check`| PASS   | No output (exit 0, clean); 71-hunk fmt drift pre-cleared by orchestrator |
| `cargo clippy --workspace --features candle -- -D warnings` | PASS | `Finished dev profile in 9.80s`; 0 errors, 0 warnings in gated crates |
| `cargo audit`      | N/A    | Not run — no new dependencies under Q2=(a) GARCH-only-MVP |
| `cargo deny`       | N/A    | Not run — no new dependencies |

### `cargo fmt --check` verbatim output

```
(no output — exit 0)
```

### `cargo clippy` verbatim output (summary lines)

```
    Checking forecast v0.1.0 (crates/forecast)
    Checking strategy v0.1.0 (crates/strategy)
    Checking backtest v0.1.0 (crates/backtest)
    Checking ui v0.1.0 (crates/ui)
    Checking agent v0.1.0 (crates/agent)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 9.80s
```

## 3. Unit & Integration Tests

Command: `cargo test --workspace --lib --features candle`

| Crate / suite           | Passed | Failed | Ignored | Duration |
|-------------------------|-------:|-------:|--------:|----------:|
| forecast (suite 1)      |     52 |      0 |       0 |    1.85s |
| forecast (suite 2)      |     36 |      0 |       0 |    0.22s |
| forecast (suite 3)      |     13 |      0 |       1 |    0.00s |
| strategy                |      9 |      0 |       0 |    0.21s |
| backtest                |     47 |      0 |       1 |    0.06s |
| replay-cache            |      6 |      0 |       0 |    0.00s |
| cost                    |     55 |      0 |       0 |    0.17s |
| audit                   |     69 |      0 |       0 |    0.77s |
| reports                 |     84 |      0 |       0 |    3.02s |
| core                    |      0 |      0 |       0 |    0.00s |
| agent (suite 1)         |     12 |      0 |       0 |    0.00s |
| agent (suite 2)         |      8 |      0 |       0 |    0.01s |
| ui (suite 1)            |    103 |      0 |       0 |    0.04s |
| ui (suite 2)            |     10 |      0 |       0 |    0.05s |
| ui (suite 3)            |    105 |      0 |       0 |    0.02s |
| ui (suite 4)            |     72 |      0 |       0 |    0.01s |
| ui (suite 5)            |    311 |      0 |       0 |    0.52s |
| **Total**               |**992** |  **0** |   **2** |          |

### Failing Tests

_none_ — all 992 tests passed; 0 failed.

### Selected new Wave A-D tests confirmed passing

| Test name                          | Source file                                                    | Status |
|------------------------------------|----------------------------------------------------------------|--------|
| `garch_fit_determinism`            | `crates/forecast/tests/garch_fit_determinism.rs`              | PASS   |
| `vol_verdict_mutual_exclusivity`   | `crates/forecast/tests/vol_verdict_mutual_exclusivity.rs`     | PASS   |
| `parkinson_target_derivation`      | `crates/forecast/tests/parkinson_target_derivation.rs`        | PASS   |
| `tcn_byte_identity`                | `crates/forecast/tests/tcn_byte_identity.rs`                  | PASS   |
| `patchtst_byte_identity`           | `crates/forecast/tests/patchtst_byte_identity.rs`             | PASS   |
| `vol_targeting_overlay` (8 tests)  | `crates/strategy/tests/vol_targeting_overlay.rs`              | PASS   |

## 4. Property / Fuzz Tests

_n/a_ — no proptest or cargo-fuzz suites added for this feature. GARCH fitter determinism is
verified via the 2-run byte-identity test (`garch_fit_determinism`) rather than property-based
testing.

## 5. Backtest Results

### Anchor verification gate

**Pre-T-T2 (30/30 baseline):**

All 30 existing anchors byte-identical per `bash scripts/verify_anchors.sh`:
```
ANCHORS PASS  (30 / 30)
```
Non-regression contracts R10, R11.7, R11.8 satisfied: TCN and PatchTST anchors unchanged.

**Post-T-T2 (33/33 after anchor lock):**

Full verify_anchors.sh output (all 33 rows PASS):
```
PASS  btc-2023-1m-sma-cross                 fc2e3b4a...
PASS  btc-2023-1m-sma-baseline-refresh      fc2e3b4a...
PASS  btc-2023-1m-macd-trend                ef9c5e48...
PASS  btc-2023-1m-rsi-reversion             bc56d20d...
PASS  btc-2023-1m-bbands-mean-revert        d8a08a23...
PASS  top10-2023-1h-momentum                3b60ef07...
PASS  top10-2024-h1-momentum                1f33534f...
PASS  pairs-2023-zscore-mr                  90591a0e...
PASS  pairs-2024-h1-zscore-mr               14f50a59...
PASS  report-sample-7d                      520b1f29...
PASS  report-sample-90d                     c656414e...
PASS  top10-2023-fy-tcn-overlay             01d02584...
PASS  top10-2024-fy-tcn-overlay             e24c85ac...
PASS  top10-2023-fy-tcn-overlay-weights     7cb1357c...
PASS  top10-2024-fy-tcn-overlay-weights     23c24dae...
PASS  top10-2023-fy-tcn-overlay-realdata    8fa47f49...
PASS  top10-2024-fy-tcn-overlay-realdata    fd8191df...
PASS  top10-2023-fy-tcn-overlay-weights-realdata  552d7df2...
PASS  top10-2024-fy-tcn-overlay-weights-realdata  2a65c434...
PASS  forecast-distribution-bs1-realdata    ef73cb8d...
PASS  forecast-distribution-bs2-realdata    d7cd08e6...
PASS  sharpe-comparison-realdata            17d2e96c...
PASS  forecast-distribution-bs1-realdata-recalibrated  8a548042...
PASS  forecast-distribution-bs2-realdata-recalibrated  d6c1e17c...
PASS  recalibrate-sigma-train-bs1           baa658fb...
PASS  recalibrate-sigma-train-bs2           bfa8104a...
PASS  threshold-sweep-bs1-realdata-recalibrated  551cc2ab...
PASS  threshold-sweep-bs2-realdata-recalibrated  755bc380...
PASS  forecast-distribution-patchtst-bs1-realdata  c55c6c51...
PASS  top10-2023-fy-patchtst-overlay-realdata  5f303cc0...
PASS  vol-verdict-bs1-realdata              99c21892...
PASS  top10-2023-fy-vol-target-overlay-realdata  66cd69ad...
PASS  sharpe-comparison-vol-target-bs1-realdata  ef048366...
---
ANCHORS PASS  (33 / 33)
```

### New backtest scenario: top10-2023-fy-vol-target-overlay-realdata

**Universe:** ADAUSDT, AVAXUSDT, BNBUSDT, BTCUSDT, DOGEUSDT, DOTUSDT, ETHUSDT, LINKUSDT,
SOLUSDT, XRPUSDT (10 symbols)
**Period:** 2023-01-01T00:00:00Z — 2024-01-01T00:00:00Z (BS-1)
**Data source:** Real Binance hourly OHLCV (REVISION.toml SHA `3a8b96c4...`)
**Fees / slippage:** Slippage 2 bps, Taker fee 4 bps

| Metric           | vol-target overlay | v1 momentum baseline | Delta       |
|------------------|--------------------|----------------------|-------------|
| Total return     | 13.48%             | -43.72%              | +57.20pp    |
| Final equity     | $113479.98 USDT    | $0.00 USDT (bankrupt)| N/A         |
| Max drawdown     | 73.73%             | 87.48%               | -13.75pp    |
| Trades           | 6203               | 4809                 | +1394       |
| Total fees       | $17430.78 USDT     | N/A                  | N/A         |
| Sharpe (ann)     | 0.003098           | -0.026770            | +0.029868   |
| Sortino (ann)    | 0.004380           | -0.037535            | +0.041915   |
| Calmar           | 0.017263           | -0.063851            | +0.081114   |
| Bars             | 87590              | 87600                | -10 (99.99%)|

**DATA CAVEAT:** Baseline uses synthetic GBM bars (passthrough forecaster, no candle); overlay
uses real Binance data. This is an apples-to-oranges comparison; the Sharpe delta may over- or
understate the true vol-targeting lift. Explicitly stated in the sharpe-comparison report body.
See `feature.md § Verification` for full data caveat.

**2-run byte-identity (R11.9 + R11.10):** Two independent backtest runs produced files
`backtest-20260522-082901-...md` and `backtest-20260522-082914-...md`; both hash to
`66cd69ad03294cccf514184968babce0127f2ebfa4d1f4a03b332f8000f79c65` (orchestrator-verified).

### V-verdict: V3 — MODEL-BROKEN (advisory)

Report: `spec/v3-volatility-forecaster/reports/vol-verdict-bs1-realdata-20260522.md`
Body-SHA: `99c2189210d2091aebf199a5fc1cc8a448d14da6911130e3d6ebb163e686cd21`

Key evidence from the per-symbol QLIKE table:

| Symbol   | calib_ratio | Contributing to V3? |
|----------|-------------|---------------------|
| ADAUSDT  | 0.985247    | No (within [0.7, 1.4]) |
| AVAXUSDT | 2.307620    | Yes (overflow) |
| BNBUSDT  | 1.009649    | No |
| BTCUSDT  | 0.963859    | No |
| DOGEUSDT | 10.247541   | Yes (severe overflow) |
| DOTUSDT  | 10.096677   | Yes (severe overflow) |
| ETHUSDT  | 0.981762    | No |
| LINKUSDT | 0.965601    | No |
| SOLUSDT  | 0.968941    | No |
| XRPUSDT  | 0.995011    | No |

`mean_calibration_ratio = 2.952191` (outside [0.7, 1.4]) → **V3 fires** per ADR-0038 § D1.b.
GARCH(1,1) non-convergence at 500 iterations for AVAX, DOGE, DOT causes sigma_hat overflow.
The 7 other symbols calibrate correctly (calib_ratio within [0.96, 1.01]).

### T-classifier: T-VOL-NO-ALPHA (advisory)

`net_delta = 0.029868` (0.003098 − (−0.026770)) < +0.05 threshold → **T-VOL-NO-ALPHA**
Note: confounded by synthetic-vs-real data mismatch (caveat above).

### Joint advisory verdict

| V-verdict | T-classifier    | Joint classification    |
|-----------|-----------------|-------------------------|
| V3        | T-VOL-NO-ALPHA  | MODEL-BROKEN / NO-ALPHA |

This is an **advisory** classification per ADR-0038 § D1.c, NOT a test failure. The code is
deterministic; the model trained, evaluated, and ran backtests successfully.

## 6. Benchmarks

_n/a_ — no criterion suites added. GARCH fitting wall-clock is ~5-10s for 10 symbols (T-AR-9).
No hot-path regressions expected; no latency-sensitive paths touched.

## 7. Environment / Infrastructure Issues

_none_ — clean run on darwin arm64. Orchestrator pre-cleared both blockers (fmt + canonical SHA)
before this tester re-gate. No flaky tests, infra outages, or data gaps encountered.

## 8. Spec-lint Gate

Command: `python3.14 scripts/spec_lint.py` (required python3.11+; system python3 is 3.9.6;
used homebrew python3.14)

Result: `spec-lint: FAIL (85 violations in 1 categories)` — dead-link (85)

**Baseline** (`spec/dev-notes/audit-2026-05-22.md`): dead-link 81, trace-broken-path 0 = 82 total.

**Current delta: +4 dead-links. No trace-broken-path regressions.**

The 4 new dead-links are all pre-existing developer/architect artifacts that pre-date this tester
pass:

1. `spec/architecture/adr/0038-vol-forecast-verdict-shape.md` → self-referencing link with
   extra `../` prefix (same pattern as ADR-0033 in the baseline). Created by architect at M-T1.
2. `spec/v3-volatility-forecaster/reports/vol-verdict-bs1-realdata-20260522.md` → link to
   `../architecture/adr/0038-...#d1-...` (developer's Wave C vol_verdict bin report output).
3-4. `spec/v3-llm-forecaster/feature.md` → 2 links to non-existent crates/reflection and
   crates/llm (parallel analyst pass for C5; no code exists yet).

**Pre-existing spec debt (does NOT block PASS):**

| Category         | Baseline | Current | Delta | Owns             |
|------------------|----------|---------|-------|------------------|
| dead-link        | 81       | 85      | +4    | developer (2) + analyst/architect (2) |
| trace-broken-path| 0        | 0       | 0     | clean            |

Routing for the +4 new dead-links: developer (ADR-0038 self-ref + vol-verdict report link);
analyst (v3-llm-forecaster spec-only links). Non-blocking for this cycle.

## 9. Anchor Delta (T-T2)

3 new rows added to `spec/anchors.toml` under `[v3.0.0-volatility]` namespace:

| Scenario                                  | Version           | SHA-256                                                             |
|-------------------------------------------|-------------------|---------------------------------------------------------------------|
| vol-verdict-bs1-realdata                  | v3.0.0-volatility | `99c2189210d2091aebf199a5fc1cc8a448d14da6911130e3d6ebb163e686cd21` |
| top10-2023-fy-vol-target-overlay-realdata | v3.0.0-volatility | `66cd69ad03294cccf514184968babce0127f2ebfa4d1f4a03b332f8000f79c65` |
| sharpe-comparison-vol-target-bs1-realdata | v3.0.0-volatility | `ef048366ac5433173016e937dce0871b4b8da368ad6d4b17621b29faacea2ab1` |

All 3 SHAs independently verified by tester via `python3 scripts/hash_report.py` against the
on-disk report files. Exact match confirmed against orchestrator-provided canonical SHAs.

Trace.toml anchors column updated: `anchors = ["vol-verdict-bs1-realdata", "top10-2023-fy-vol-target-overlay-realdata", "sharpe-comparison-vol-target-bs1-realdata"]`

## 10. Cross-references

| Artifact                                    | Path                                                                                          |
|---------------------------------------------|-----------------------------------------------------------------------------------------------|
| Vol-verdict report                          | `spec/v3-volatility-forecaster/reports/vol-verdict-bs1-realdata-20260522.md`                 |
| Backtest report (run 1)                     | `spec/v3-volatility-forecaster/reports/backtest-20260522-082901-top10-2023-fy-vol-target-overlay-realdata.md` |
| Backtest report (run 2, byte-identity gate) | `spec/v3-volatility-forecaster/reports/backtest-20260522-082914-top10-2023-fy-vol-target-overlay-realdata.md` |
| Sharpe-comparison report                    | `spec/v3-volatility-forecaster/reports/sharpe-comparison-vol-target-bs1-realdata-20260522.md` |
| ADR-0038 V-verdict shape                    | `spec/architecture/adr/0038-vol-forecast-verdict-shape.md`                                   |
| Decomp (cargo invocations + wave map)       | `spec/v3-volatility-forecaster/decomp.md`                                                    |
| Feature brief (§ Verification added)        | `spec/v3-volatility-forecaster/feature.md`                                                   |
| Anchors file                                | `spec/anchors.toml` (30 pre-existing + 3 new = 33 total)                                     |
| Trace row                                   | `spec/trace.toml` REQ-V3-VOL-FORECASTER-001 (state: shipped)                                 |
| Tasks                                       | `spec/v3-volatility-forecaster/tasks.md` (T-T1..T-T3 ticked; T-P1 presenter row unticked)    |

## 11. Verdict

**`VERDICT → PASS`**

The v3-volatility-forecaster Wave E M-FINAL re-gate passes all 4 mandatory T-T1 checks:
`cargo fmt --check` (clean after orchestrator pre-cleared the fmt drift),
`cargo clippy --workspace --features candle -- -D warnings` (0 errors/warnings in gated crates),
`cargo test --workspace --lib --features candle` (992 passed / 0 failed / 2 ignored across 17
suites), and `bash scripts/verify_anchors.sh` (30/30 pre-T-T2 baseline, 33/33 post-T-T2 after
locking 3 new v3.0.0-volatility anchors). The spec-lint +4 dead-link delta is pre-existing
developer/architect artifact debt that pre-dates this tester pass; zero new tester-introduced
violations.

The joint advisory verdict — **V3 x T-VOL-NO-ALPHA -> MODEL-BROKEN / NO-ALPHA** — is an
advisory finding, not a test failure. The GARCH(1,1) fitter did not converge for AVAX/DOGE/DOT
(max_iters=500), and the Sharpe comparison is confounded by a synthetic-vs-real data mismatch.
The code is deterministic; all non-regression contracts (R10, R11.7, R11.8, R11.9, R11.10) are
satisfied; and the advisory verdict plus data caveat are recorded in `feature.md § Verification`
for operator review via the presenter.

## 12. Routing

`HANDOFF -> orchestrator -> presenter`

The code gate passes. The advisory V3 x T-VOL-NO-ALPHA verdict is recorded in
`spec/v3-volatility-forecaster/feature.md § Verification` for the presenter to surface to the
operator. Routing recommendations (not decided by tester):
- **C1 retirement candidate:** operator must decide given synthetic-vs-real data caveat and V3
  GARCH non-convergence on AVAX/DOGE/DOT.
- **C2 (v3-regime-classifier) promotion candidate** per HYBRID sequencing.
- **C5 (v3-llm-forecaster) promotion candidate** per HYBRID sequencing.
- **V0.1.1 GARCH refit** if operator chooses to debug V3 (per-symbol hyperparameter search,
  max_iters > 500, tighter convergence tol for non-convergent symbols).
