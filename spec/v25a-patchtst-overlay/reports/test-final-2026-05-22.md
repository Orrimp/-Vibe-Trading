---
title: Test Report — M-FINAL
feature: v25a-patchtst-overlay
run_id: 2026-05-22-0000-UTC
commit: a0dee41ebc1352891518cd8ac11d6826cd8992de
agent: tester
verdict: FAIL
---

# Test Report — v25a-patchtst-overlay — 2026-05-22 00:00 UTC

## 1. Scope

- **Feature / change under test:** v2.5a PatchTST forecast overlay v0.1.0 — phase 2 of 4 DL roadmap.
  Wave A (model + scaffold + 4 unit tests) + Wave B (BS-1 training, 30 epochs, sigma_train=0.007053,
  7h 45min wall-clock) + Wave D (alpha-investigation + strategy + backtest + Sharpe-comparison).
- **Spec refs:** `spec/v25a-patchtst-overlay/feature.md`, `spec/v25a-patchtst-overlay/tasks.md`,
  `spec/v25a-patchtst-overlay/decomp.md`, ADR-0036.
- **Commit SHA:** `a0dee41ebc1352891518cd8ac11d6826cd8992de`
- **Rust toolchain:** `rustc 1.94.1 (e408947bf 2026-03-25)`
- **OS / arch:** `darwin 25.4.0 / Apple Silicon`

---

## 2. Static Analysis

| Check                                     | Result | Notes                                              |
|-------------------------------------------|--------|----------------------------------------------------|
| `cargo fmt --check`                       | PASS   | No diff. Zero formatting violations.               |
| `cargo clippy --workspace -- -D warnings` | PASS   | `Finished dev profile [unoptimized + debuginfo] target(s) in 1.00s` |
| `cargo clippy -p forecast --features candle -- -D warnings` | PASS | `Finished dev profile [unoptimized + debuginfo] target(s) in 0.23s` |
| `cargo clippy -p backtest --features "candle realdata" -- -D warnings` | PASS | `Finished dev profile [unoptimized + debuginfo] target(s) in 0.24s` |
| `cargo clippy -p strategy --features forecast,forecast-audit-tick -- -D warnings` | PASS | `Finished dev profile [unoptimized + debuginfo] target(s) in 3.09s` |
| `cargo audit`                             | NOT RUN | Skipped (no security-affecting changes; no new dependencies added). |
| `cargo deny`                              | NOT RUN | Skipped (no new dependencies added). |

**T-F1 through T-F4: PASS.**

---

## 3. Unit & Integration Tests

### T-F5: Workspace lib tests

```
$ cargo test --workspace --lib
test result: ok. 311 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.54s
```

**PASS. 311/311.** (baseline Phase F = 311+; count maintained.)

### T-F6a: `forward_determinism_patchtst` (K2 determinism)

```
$ cargo test -p forecast --features candle --test forward_determinism_patchtst
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.45s
```

**PASS.**

### T-F6b: `sigma_train_not_in_safetensors_patchtst` (ADR-0035 § D4)

```
$ cargo test -p forecast --features candle --test sigma_train_not_in_safetensors_patchtst
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

**PASS.**

### T-F6c: `tcn_byte_identity` (K6 scope-creep guard)

```
$ cargo test -p forecast --features candle --test tcn_byte_identity
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.19s
```

**PASS.**

### T-F6d: `forecast_distribution_verdict` (ADR-0033 § D3 immutability)

```
$ cargo test -p forecast --features candle --bin sharpe_comparison
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s
```

**PASS. 8/8 unit tests including F-verdict algorithm tests.**

### T-F7: Benchmark smoke test

```
$ cargo bench -p reflection --bench trail_mirror -- --list
    Finished `bench` profile [optimized] target(s) in 0.17s
     Running benches/trail_mirror.rs
Gnuplot not found, using plotters backend
trail_mirror/trail_mirror_open: benchmark
```

**PASS.** Reflection bench crate compiles and lists benchmarks correctly.

### T-F12 / T-T-1.i: `patchtst_overlay_neutrality` (K4 anchor-neutrality gate) — **FAIL**

```
$ cargo test -p forecast --features candle --test patchtst_overlay_neutrality -- --ignored --nocapture
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.23s
     Running tests/patchtst_overlay_neutrality.rs

running 1 test
[patchtst_overlay_neutrality] Running top10-2023-fy-tcn-overlay-realdata scenario...

thread 'patchtst_overlay_does_not_regress_tcn_scenario' panicked at
crates/forecast/tests/patchtst_overlay_neutrality.rs:136:9:
backtest run failed (exit 101)
stdout:
stderr:
error: `cargo run` could not determine which binary to run.
Use the `--bin` option to specify a binary, or the `default-run` manifest key.
available binaries: backtest, threshold_sweep

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.28s
```

**FAIL — test infrastructure defect.**

**Root cause:** `crates/forecast/tests/patchtst_overlay_neutrality.rs:107-123` passes
`["run", "-p", "backtest", "--release", "--features", "candle realdata", "--", ...]` to
`Command::new("cargo")` without specifying `--bin backtest`. The `backtest` crate has TWO
binaries (`backtest` + `threshold_sweep`) since the `v25-tcn-threshold-tuning` Wave B commit
(`42e084e`) added `crates/backtest/src/bin/threshold_sweep.rs`. The K4 test was written at
Wave A time (before the second binary existed) and was never updated when `threshold_sweep`
was added to the crate.

**Not a strategy regression.** The underlying body-SHA for `top10-2023-fy-tcn-overlay-realdata`
remains `8fa47f49e887df480509f30dfc08afcb9febecdb6a5bbdbb04023f241a9d9642`, confirmed by
`verify_anchors.sh` (PASS 30/30 including this scenario). The scenario output itself is intact;
only the test runner invocation is broken.

**Fix required:** In `patchtst_overlay_neutrality.rs:109-121`, insert `"--bin"` and `"backtest"`
into the args slice after `"--release"`, so that `cargo run` is unambiguous.

| Crate | Passed | Failed | Ignored | Duration |
|-------|-------:|-------:|--------:|---------:|
| `forecast` (lib tests, part of workspace) | 311 | 0 | 0 | 0.54s |
| `forward_determinism_patchtst` | 2 | 0 | 0 | 0.45s |
| `sigma_train_not_in_safetensors_patchtst` | 1 | 0 | 1 | 0.00s |
| `tcn_byte_identity` | 1 | 0 | 0 | 1.19s |
| `sharpe_comparison` (bin unit tests) | 8 | 0 | 0 | 0.05s |
| `patchtst_overlay_neutrality` (K4, --ignored) | 0 | **1** | 0 | 4.28s |
| **Total** | **323** | **1** | **1** | |

### Failing Test

**`patchtst_overlay_does_not_regress_tcn_scenario`**
File: `crates/forecast/tests/patchtst_overlay_neutrality.rs:51`
Failure at line 136: `panic!("backtest run failed (exit 101)\n...")`
Cargo stderr: `error: could not determine which binary to run. Use --bin option.`
Fix: add `"--bin", "backtest"` args to the `Command::new("cargo")` invocation at line 109.

---

## 4. Property / Fuzz Tests

_n/a_ — no proptest / cargo-fuzz suites added for this feature.

---

## 5. Backtest Results

**Universe:** 10 symbols (ADAUSDT, AVAXUSDT, BNBUSDT, BTCUSDT, DOGEUSDT, DOTUSDT, ETHUSDT, LINKUSDT, SOLUSDT, XRPUSDT)
**Period:** 2023-01-01 .. 2024-01-01
**Data source:** Real Binance Vision hourly OHLCV (`data/binance/`, REVISION.toml SHA `3a8b96c4…`)
**Fees / slippage model:** Taker fee 4 bps + slippage 2 bps; equal-weight, exposure_cap=50%, k_long=3

| Metric               | PatchTST BS-1 (current) | v1 passthrough baseline | Delta        |
|----------------------|------------------------:|------------------------:|-------------:|
| Total return 2023-FY | +31.13%                 | +13.48%                 | +17.65 pp    |
| Max drawdown         | 77.97%                  | 73.73%                  | +4.24 pp     |
| Trades               | 3187                    | 6203                    | -3016        |
| Dampen rate          | 28.96%                  | 0.00%                   | +28.96 pp    |
| Sharpe (ann)         | 0.009243                | 0.003098                | **+0.006144** |
| Sortino (ann)        | 0.013133                | 0.004380                | +0.008753    |
| Calmar               | 0.035234                | 0.017263                | +0.017971    |
| Final equity         | $131,125.07             | $113,480 approx         | —            |
| Fees total           | $11,157.70              | ~$17k approx            | -$5.8k       |

**Sharpe delta vs T-ALPHA-UNLOCKED threshold:** +0.006144 vs +0.10 required. **Delta = -0.093856 below threshold.**

### Equity Curve

PatchTST BS-1 equity grows from $100k to $131k over 2023-FY, outperforming the passthrough baseline
($113k) in absolute terms due to the 28.96% dampen rate reducing trade count from 6203 to 3187 and
cutting fees by ~$5.8k. The higher total return is therefore primarily a fee-reduction effect, not
alpha extraction. The max drawdown of 77.97% is worse than the passthrough's 73.73%, indicating
PatchTST's gate is selectively suppressing trades (particularly in low-confidence windows) but not
improving risk-adjusted returns. The worst drawdown window occurs in the same mid-2023 regime as
the passthrough baseline.

### F-Verdict Analysis

Per ADR-0033 § D3 (immutable priority tree):

| Field             | Value                                        |
|-------------------|----------------------------------------------|
| F-verdict         | **F4** (no predictive signal; distributional failure) |
| sigma_train       | 0.007053 (post-training; canonical; ADR-0035 § D1 compliant) |
| epsilon (deadband) | 0.000500 |
| tau (confidence gate) | 0.600000 |
| Gate-survival rate | 55.79% (frac_passes_confidence_gate = 0.557942688) |
| frac_inside_epsilon | 0.054883 (< 0.5 F3 threshold; F4 confirmed) |
| std/sigma_train ratio | 1.000 (correctly calibrated; not mis-inflated per ADR-0035) |
| abs_p95           | 0.014274920 |
| Inferences        | 76,800 (10 symbols × 7,680 windows) |
| Wall clock        | 404.8s |

**Interpretation:** sigma_train = std(r_hat) = 1.0 confirms the PatchTST model is correctly
calibrated (no inflation artifact). The gate-survival rate of 55.8% at tau=0.6 shows the model
emits high-confidence predictions (it's not collapsing to mid-range outputs), but frac_inside_epsilon
= 0.054883 << 0.5 means only 5.49% of predictions fall within the ε=0.0005 deadband. The model
is predicting non-trivial magnitudes but they are systematically wrong as directional signals at 24h
horizon.

### 3-Way Comparison Table: TCN vs PatchTST (operator-decision-grade signal)

| Ship                         | Model     | Horizon | Sharpe-delta | F-verdict | T-classifier  |
|------------------------------|-----------|---------|-------------:|-----------|---------------|
| v25-tcn-threshold-tuning BS-1 | TCN       | 1h      | +0.018       | F4        | T-MARGINAL    |
| v25-tcn-threshold-tuning BS-2 | TCN       | 1h      | +0.045       | F4        | T-MARGINAL    |
| **v25a-patchtst-overlay BS-1** | **PatchTST** | **24h** | **+0.006144** | **F4** | **T-MARGINAL** |

**Joint verdict across 3 checkpoints: F4 / F4 / F4. PatchTST at 24h delivers LOWER Sharpe-delta
(+0.006) than TCN at 1h (+0.018 / +0.045). Neither paradigm approaches the +0.10 T-ALPHA-UNLOCKED
threshold.**

### Regressions vs Baseline

No regression in underlying backtest scenario body bytes — all 30 anchors PASS (verify_anchors.sh
30/30). PatchTST equity curve higher in absolute returns but this is a fee-reduction effect, not
a risk-adjusted improvement. Max drawdown marginally worse (+4.24 pp) — not a blocking regression
per ADR-0033 criteria (F-verdict drives routing, not drawdown delta in isolation).

---

## 6. Benchmarks

_n/a_ — no hot paths modified. `cargo bench -p reflection --bench trail_mirror -- --list`
compiled successfully (`Finished bench profile [optimized] target(s) in 0.17s`).

---

## 7. Anchor Gate

### PRE-lock (28 original anchors)

```
$ bash scripts/verify_anchors.sh
ANCHORS PASS  (28 / 28)
```

All 28 originals byte-identical. The 2 previously failing glob-collision anchors
(`forecast-distribution-bs1-realdata`, `forecast-distribution-bs2-realdata`) now PASS
because `scripts/verify_anchors.sh` was fixed at commit `09fb962` (digit-only timestamp suffix
requirement). No pre-existing FAIL baseline — this is an improvement over the analyst-captured
baseline of 26 PASS + 2 FAIL.

### POST-lock (30 anchors after adding 2 new PatchTST entries)

```
$ bash scripts/verify_anchors.sh
ANCHORS PASS  (30 / 30)
```

2 new anchors locked under version `v2.5a.0-patchtst`:
- `forecast-distribution-patchtst-bs1-realdata` → SHA `c55c6c5178374f230f5273df1e20d121589ff0b879c20062ee6cbdca7f4646dd`
- `top10-2023-fy-patchtst-overlay-realdata` → SHA `5f303cc0812d421e6efdc40c0f412dd8cc0625891c677442bf2d7d2d5336ab4c`

`sharpe-comparison-patchtst-bs1-realdata` NOT anchored at v0.1.0 per decomp.md § line 193
("defer to v2.6 bake-off to keep v0.1.0 lean").

### TCN Checkpoint Byte-Identity (K6 — T-F10)

```
$ git diff HEAD -- crates/forecast/src/tcn.rs
(empty)
$ git diff HEAD -- "crates/forecast/checkpoints/anchors/tcn-*.safetensors" \
    "crates/forecast/checkpoints/anchors/tcn-*.metadata.json" \
    "crates/forecast/checkpoints/anchors/tcn-*.metadata.recalibrated.json"
(empty)
```

8 TCN files byte-identical: `tcn-bs1-d1c3696d…{safetensors,metadata.json,metadata.recalibrated.json}` +
`tcn-bs2-3fabcabe…{safetensors,metadata.json,metadata.recalibrated.json}`. K6 PASS.

### 2-Run Byte-Identity (T-F8)

Tester re-verified report body SHAs against developer-reported values (T-D-N21/N25):

| Report                                           | Tester-computed SHA                                                | Developer-reported SHA | Match |
|--------------------------------------------------|---------------------------------------------------------------------|------------------------|-------|
| `forecast-distribution-patchtst-bs1-realdata-20260521.md` | `c55c6c5178374f230f5273df1e20d121589ff0b879c20062ee6cbdca7f4646dd` | same | PASS |
| `backtest-20260521-220035-top10-2023-fy-patchtst-overlay-realdata.md` | `5f303cc0812d421e6efdc40c0f412dd8cc0625891c677442bf2d7d2d5336ab4c` | same | PASS |
| `sharpe-comparison-patchtst-bs1-realdata-20260521.md` | `45140833cf13a9bcdcbe464684f61d1a8566c9d5d28b7667c2dc056b1063bfb9` | same | PASS |

Invocations: `python3 scripts/hash_report.py spec/v25a-patchtst-overlay/reports/<file>` for each.
Dev reported 2-run and 3-run byte-identity; tester confirms SHA values match on independent
re-hash of the files on disk.

---

## 8. Spec-Lint Gate

```
$ uv run scripts/spec_lint.py
spec-lint: FAIL (86 violations in 3 categories)
dead-link (81):
shipped-no-tests (1):
trace-broken-path (4):
```

**Comparison to previous tester report baseline (threshold-tuning, 2026-05-21):**
- Prior baseline: 87 violations / 2 categories (`dead-link` + `trace-broken-path`)
- Current: 86 violations / 3 categories (`dead-link=81`, `shipped-no-tests=1`, `trace-broken-path=4`)

**Analysis of delta:**
- `dead-link`: 81 (down from implied ~81 — count stable; note prior report quoted 81 dead-links explicitly)
- `shipped-no-tests` (NEW category): 1 violation — `spec/v25-tcn-horizon-bump-or-retire/feature.md`
  has `status: shipped` but no `.md` test report. This feature was intentionally zero-code-change
  (operator-decide only); the violation is pre-existing debt, not introduced by this feature.
  `v25-tcn-horizon-bump-or-retire` was committed at `status: shipped` (commit `9632a61`) BEFORE
  the prior tester report was written. The prior report did not detect it (2-category output
  suggests the `shipped-no-tests` check may trigger only when `spec_lint.py` resolves certain
  edge cases). This is pre-existing baseline debt.
- `trace-broken-path`: 4 (down from 6 in prior report) — `REQ-V25A-PATCHTST-001` anchor refs
  are now resolved (2 new anchors locked at M-FINAL). Remaining 4 are `REQ-V25B-TRANSFORMER-001`
  (x2) + `REQ-V26-BAKEOFF-001` (x2) — future-feature stubs.

**New regressions introduced by this feature: 0.** The `shipped-no-tests` category is pre-existing
debt triggered by a prior ship, not by PatchTST code. Total violation count decreased (86 < 87).

**spec-lint: PASS-WITH-PREEXISTING-DEBT** (0 new categories or count growth from this feature).

## 8a. Pre-existing Spec Debt (quoted per spec-lint gate rule)

Carried-over violations that do NOT block PASS but are quoted for visibility:

1. **81 dead-link violations** — dominated by stale roadmap links (v25-kronos, iced-aw, chart-canvas,
   v0-paper-sma screenshot README, v25-tcn-alpha-investigation/tasks.md → template link). None
   introduced by this feature.
2. **1 shipped-no-tests violation** — `spec/v25-tcn-horizon-bump-or-retire/feature.md`: zero-code-change
   operator-decide feature shipped intentionally without a test report. Pre-existing; not introduced
   by this feature.
3. **4 trace-broken-path violations** — `REQ-V25B-TRANSFORMER-001` (x2) + `REQ-V26-BAKEOFF-001` (x2)
   reference future anchors not yet in `anchors.toml`. Backlog stubs for phases 3 and 4. None
   introduced by this feature.

---

## 9. Task Row Verification

### T-T-1.a / T-F9 PRE: verify_anchors.sh PRE-lock

```
$ bash scripts/verify_anchors.sh 2>&1 | tail -1
ANCHORS PASS  (28 / 28)
```

All 28 originals byte-identical. PASS.

### T-T-1.b: Lock 2 new PatchTST anchors

Anchors locked in `spec/anchors.toml` under version `v2.5a.0-patchtst`. DONE.

### T-T-1.c: Static analysis gates

`cargo fmt --check` + all 4 clippy invocations PASS. DONE.

### T-T-1.d: `cargo test --workspace --lib`

311/311 PASS. DONE.

### T-T-1.e / T-F6b: `sigma_train_not_in_safetensors_patchtst`

1 passed, 1 ignored. PASS.

### T-T-1.f / T-F6d: `forecast_distribution_verdict`

8 passed (bin unit tests for sharpe_comparison which exercise the F-verdict). PASS.

### T-T-1.g / T-F6a: `forward_determinism_patchtst`

2 passed. PASS.

### T-T-1.h / T-F6c: `tcn_byte_identity`

1 passed. PASS.

### T-T-1.i / T-F12: `patchtst_overlay_neutrality` (K4)

**FAIL.** See § 3 for root cause and fix description.

### T-T-1.j: `git diff HEAD -- crates/forecast/src/tcn.rs`

Empty diff. PASS.

### T-T-1.k: TCN checkpoint byte-identity

Empty diff for all 8 TCN checkpoint files. PASS.

### T-T-1.l / T-F8: 2-run byte-identity determinism

All 3 report SHAs confirmed. PASS.

### T-T-1.m / T-F11: spec-lint

86 violations / 3 categories. 0 new regressions from this feature. PASS-WITH-PREEXISTING-DEBT.

### T-T-1.n: Joint advisory verdict

Recorded in § 10 (Routing) below. DONE.

### T-T-1.o: Trace row columns

`REQ-V25A-PATCHTST-001` `anchors` column filled with 2 entries. `state` left as `in-progress`
pending K4 fix; will flip to `tester-pass` on re-verification. PARTIAL (awaiting K4 fix).

---

## 10. Verdict

**`FAIL`**

All hard static-analysis gates PASS (fmt, 4 clippy invocations). Workspace lib test suite PASS
(311/311). Four targeted integration tests PASS (forward_determinism_patchtst, sigma_train_not_
in_safetensors_patchtst, tcn_byte_identity, forecast_distribution_verdict/sharpe_comparison
unit tests). Anchor gate PASS 30/30 post-lock. 2-run byte-identity for all 3 new reports
PASS. 28 original anchor body-SHAs byte-identical. TCN checkpoint files byte-identical.

One gate FAILS: **T-F12 / T-T-1.i — `patchtst_overlay_neutrality` K4 test.** The test
invokes `cargo run -p backtest` without specifying `--bin backtest`; the `backtest` crate has
two binaries since the `v25-tcn-threshold-tuning` Wave B ship (`42e084e`). The fix is a
1-line change to `crates/forecast/tests/patchtst_overlay_neutrality.rs:109-121` to insert
`"--bin", "backtest"` into the args slice. The underlying scenario body-SHA is intact (verified
by anchor gate PASS for `top10-2023-fy-tcn-overlay-realdata`).

This is a test infrastructure defect, not a strategy regression. The F4 signal, all anchors,
and all strategy/backtest code are sound. The fail is in the test harness only.

---

## 11. Routing

**HANDOFF → developer** — fix `crates/forecast/tests/patchtst_overlay_neutrality.rs` to add
`"--bin", "backtest"` in the `cargo run` args (line ~109). One-line change. Re-run
`cargo test -p forecast --features candle --test patchtst_overlay_neutrality -- --ignored --nocapture`
and confirm `test result: ok. 1 passed`. Return to tester for M-FINAL re-sweep (T-F12 only needs
re-verification; all other gates stay PASS).

**After K4 fix is confirmed PASS, the pending presenter routing is:**
`HANDOFF → presenter` — all hard gates green; F4 joint verdict (TCN BS-1 +0.018 / TCN BS-2
+0.045 / PatchTST BS-1 +0.006144) surfaces the operator-decision-grade signal below. Presenter
assembles the deck per T-P-1 routing table (H1 falsified / F4 → retire/pivot recommendation).

---

## 12. Operator-Decision-Grade Signal: Joint F4 Verdict

The 4-phase DL forecast overlay roadmap now has 3 completed checkpoints:

| Ship | Model | Horizon | Sharpe-delta | F-verdict | T-classifier |
|------|-------|---------|-------------:|-----------|--------------|
| v25-tcn-threshold-tuning BS-1 | TCN (dilated conv) | 1h | +0.018 | F4 | T-MARGINAL |
| v25-tcn-threshold-tuning BS-2 | TCN (dilated conv) | 1h | +0.045 | F4 | T-MARGINAL |
| **v25a-patchtst-overlay BS-1** | **PatchTST (patch-attention)** | **24h** | **+0.006144** | **F4** | **T-MARGINAL** |

**Joint verdict: F4 / F4 / F4 across 2 model families and 2 horizons.**

PatchTST at 24h delivered LOWER Sharpe-delta than TCN at 1h. Both paradigms
(convolutional + patch-attention) fail on this data / overlay shape. Three routing options
for the operator to decide at the presenter:

- **(a) Retire entire 4-phase DL forecast overlay project.** Both families F4. The v2.5b
  vanilla decoder Transformer (phase 3) is unlikely to perform materially better given the
  joint evidence. Retire the direction and begin v2.6 bake-off as a closure exercise —
  pick the highest-equity-curve F4 model (PatchTST BS-1 at +31.13% total return) as the
  canonical reference and mark the DL overlay project terminal. Frees ~3-5 weeks of compute
  budget for other research directions.
- **(b) Continue to v2.5b vanilla decoder Transformer.** Give one more architecture family
  a chance before the v2.6 bake-off. ~3-5 weeks compute commitment for a direction where
  3 prior ships converge on F4. Low expected value per the joint evidence.
- **(c) Pivot strategy-side.** Accept that 1h/24h log-return point prediction is the wrong
  task. Reformulate as volatility forecasting, regime classification, or longer-horizon
  trend signal (weekly). This is an analyst/architect pivot, not a model-family pivot, and
  preserves the candle + PatchTST infrastructure already built.

Surface clearly to operator: option (a) is the highest-signal routing given 3x F4 evidence.
Option (c) is the highest-upside pivot if the operator believes the data contains extractable
signal that the current task formulation misses.
