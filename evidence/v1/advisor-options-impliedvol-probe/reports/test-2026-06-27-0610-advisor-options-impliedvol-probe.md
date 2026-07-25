---
title: Test Report
feature: advisor-options-impliedvol-probe
run_id: 2026-06-27-0610-UTC
commit: 5c103abc7ff6f269d6f7b1f6607406beb25938e3
agent: tester
verdict: FAIL
---

# Test Report — advisor-options-impliedvol-probe — 2026-06-27 06:10 UTC

## 1. Scope

- **Feature / change under test:** ADR-0072 — Deribit DVOL implied-vol bake-off arm. Adds `DvolRegimeStrategy` (W=30 trailing median cut) as `v0.dvol_regime` in `default_field()`, including corpus loading with SHA-pin, `PitSeries::as_of_value` look-ahead-safe join, graceful skip for non-BTC/ETH symbols, and two CLAUDE.md non-negotiable day-1 gates.
- **Spec refs:** `spec/advisor-options-impliedvol-probe/feature.md`, `spec/advisor-options-impliedvol-probe/tasks.md`
- **Commit SHA:** `5c103abc7ff6f269d6f7b1f6607406beb25938e3`
- **Rust toolchain:** rustc 1.94.1 (e408947bf 2026-03-25)
- **OS / arch:** macOS 26.5.1 / arm64

## 2. Static Analysis

| Check               | Result   | Notes                                                        |
|---------------------|----------|--------------------------------------------------------------|
| `cargo fmt --check` | FIXED    | Developer submitted with 10 fmt diffs; tester ran `cargo fmt --all` → re-check PASS |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS | EXIT 0; 26m 32s compile; zero warnings across all workspace targets including new DVOL files |
| `cargo audit`       | N/A      | `cargo audit` not installed in this environment; non-blocking |
| `cargo deny`        | N/A      | Not configured for workspace                                  |
| `scripts/spec_lint.py` | PASS  | `spec-lint: PASS (0 violations)`                             |
| `scripts/verify_anchors.sh` | PASS | 119 / 119 anchors verified                             |

### Formatting note

`cargo fmt --check` returned EXIT 1 on the developer-submitted code (10 diff sites across `crates/backtest/src/dvol_data.rs`, `crates/backtest/tests/dvol_bakeoff_path_gate.rs`, and `crates/strategy/src/dvol_regime.rs`). The tester applied `cargo fmt --all` to bring the workspace into compliance before running further gates. This is a process gap in the developer's pre-submit checklist.

## 3. Unit & Integration Tests

### Mandatory CLAUDE.md gates (synthetic, always run)

| Test | Status | Duration |
|------|--------|----------|
| `dvol_regime_diverges_from_buyhold_by_at_least_1bp` (divergence e2e) | PASS | 0.00s |
| `warmup_no_dvol_matches_buyhold_on_flat_bars` (leak check) | PASS | 0.00s |
| `future_shifted_dvol_changes_decisions` (leak check) | PASS | 0.00s |

Both non-negotiable gates from CLAUDE.md confirmed: the strategy overlay diverges from buyhold by ≥1bp on a 90-bar synthetic, and future-shifted DVOL demonstrably changes decisions (causal join proven).

### Strategy unit tests

| Test | Status |
|------|--------|
| `dvol_regime::tests::odd_window_median_is_middle` | ok |
| `dvol_regime::tests::even_window_median_is_mean_of_two_middle` | ok |
| `dvol_regime::tests::calm_regime_emits_buy_when_flat` | ok |
| `dvol_regime::tests::stress_regime_emits_sell_when_long` | ok |
| `dvol_regime::tests::none_dvol_emits_hold` | ok |
| `dvol_regime::tests::warm_up_emits_hold` | ok |
| `dvol_regime::tests::repeated_close_not_added_to_ring` | ok |
| `dvol_regime::tests::config_schema_is_valid_json` | ok |
| `dvol_regime::tests::hold_after_sell_when_still_stress` | ok |
| `dvol_regime::tests::hold_after_buy_when_still_calm` | ok |
| `dvol_regime::tests::tie_resolves_to_cash` | ok |
| `dvol_regime::tests::thirty_distinct_closes_fill_ring` | ok |
| **Total** | **12 passed / 0 failed** |

### Corpus-dependent path gate tests (`#[ignore]`, real corpus required)

All run with `RobustnessMode::Skip` (wiring check, not bootstrap verdict).

| Test | Status |
|------|--------|
| `dvol_regime_bakeoff_differs_from_buyhold` (BTCUSDT) | PASS |
| `dvol_regime_bakeoff_eth_differs_from_buyhold` (ETHUSDT) | PASS |
| `solusdt_bakeoff_runs_clean_without_dvol_arm` (SOLUSDT skip gate) | PASS |
| DVOL corpus SHA smoke (`real_corpus_load_smoke`) | PASS — 182 rows, sha=8e6b8000e87dde1c1af59a378a4e29a4e68367d24b9784e9817215e34d4c402f |

### Corpus-dependent path gate tests — BTCUSDT Skip mode detailed results

```
v0.sma                 sharpe=-0.038   total_ret=-0.0007%  trades=115
v0.5.macd              sharpe= 0.966   total_ret=+0.0106%  trades=189
v0.5.rsi               sharpe= 0.478   total_ret=+0.0037%  trades=120
v0.5.bbands            sharpe=-1.423   total_ret=-0.0109%  trades=206
v0.donchian_break      sharpe=-1.083   total_ret=-0.0105%  trades=523
v0.donchian_floor      sharpe= 1.232   total_ret=+0.0441%  trades=1
v0.vol_breakout        sharpe=-1.478   total_ret=-0.0096%  trades=246
v0.roc_momentum        sharpe= 0.000   total_ret=+0.0000%  trades=0
v0.obv                 sharpe=-1.242   total_ret=-0.0190%  trades=499
v0.dvol_regime         sharpe=-0.190   total_ret=-0.0029%  trades=15
v0.buyhold             sharpe= 1.486   total_ret=+0.4778%  trades=0

|dvol_equity - buyhold_equity| = 48082.03 (32.54%)
outcome = BenchmarkWins (pre-registered expected null; Skip mode only)
```

### Failing test — CRITICAL

**`bakeoff_full_wired_advisor::t7_1_full_wired_advisor_bakeoff_real_data`** — FAILED

File: `crates/backtest/tests/bakeoff_e2e.rs:441`

```
thread 'bakeoff_full_wired_advisor::t7_1_full_wired_advisor_bakeoff_real_data' panicked at crates/backtest/tests/bakeoff_e2e.rs:441:9:
assertion `left == right` failed: T7.1: expected 13 candidates (12 field + 1 buyhold), got 19
  left: 19
 right: 13
```

**Root cause:** The developer added `v0.dvol_regime` to `default_field()` (now 10 entries) but did not update the hardcoded count assertion in the T7.1 test. The test uses `default_field() ∪ default_ensemble_field()` = 10 + 8 = 18 rule/ensemble arms + 1 buyhold = 19 candidates. The assertion at `bakeoff_e2e.rs:441` still says `13` (stale from before ADR-0067/ADR-0072 expansions). The docstrings at lines 302, 305, 313, 375, 440 also need updating to say 19 (= 18 + buyhold) and "18 arms before buyhold".

**Note:** Despite the assertion failure, the bootstrap computation completed successfully before the assertion check. The pre-failure leaderboard output provides the DECISIVE BTC bootstrap verdict (see §5).

## 4. Property / Fuzz Tests

_n/a_ — No proptest or cargo-fuzz suites for this feature.

## 5. Backtest Results — DECISIVE Bootstrap Verdict

The CLAUDE.md non-negotiable mandate requires the actual bootstrap-gated robustness flag (not `RobustnessMode::Skip`). Both BTC and ETH verdicts were obtained via 1000-path bootstrap runs.

### BTCUSDT H1_2024 — Bootstrap 1000 (from bospmgiay pre-assertion output)

**Period:** 2024-01-01 — 2024-07-01 UTC  
**Data:** BinanceCache / Deribit-DVOL corpus (182 rows, SHA-pinned)  
**Seed:** [0xC0, 0xFF, 0xEE, 0, …]; bootstrap seed: 0x0000000000eeffc0  
**Mode:** `RobustnessMode::Bootstrap { paths: 1000, seed: 0x0000000000eeffc0 }`

| Rank | Strategy | Sharpe | Return% | MaxDD% | Trades | RobustnessFlag |
|------|----------|-------:|--------:|-------:|-------:|---------------|
| 1 | v0.buyhold | 1.486 | +47.78 | 22.69 | 0 | **Fragile** (CROWN) |
| 2 | v0.donchian_floor | 1.232 | +4.42 | 3.59 | 1 | Fragile |
| 3 | v0.8.vote.trend_pair | 1.075 | +1.07 | 0.82 | 167 | Fragile |
| 4 | v0.5.macd | 0.966 | +1.06 | 0.86 | 189 | Fragile |
| 5 | v0.8.vote.k2of4 | 0.495 | +0.62 | 1.83 | 337 | Fragile |
| 6 | v0.5.rsi | 0.478 | +0.38 | 0.97 | 120 | Fragile |
| ... | ... | ... | ... | ... | ... | ... |
| 13 | **v0.dvol_regime** | **-0.190** | **-0.30** | **2.11** | **15** | **Fragile** |
| ... | ... | ... | ... | ... | ... | ... |

**v0.dvol_regime BTCUSDT Bootstrap verdict: FRAGILE** (ineligible to crown)  
**Crowned:** v0.buyhold  
**Outcome:** BenchmarkWins / BenchmarkUndefeated  

Note: ALL 19 candidates are FRAGILE on BTCUSDT H1_2024 (1000-path bootstrap). This is consistent with a bull-year where buy-hold dominates with bootstrapped consistency above the FRAGILE threshold.

### ETHUSDT H1_2024 — Bootstrap 1000 (tester_eth_dvol_bootstrap_verdict, EXIT 0)

**Period:** 2024-01-01 — 2024-07-01 UTC  
**Mode:** `RobustnessMode::Bootstrap { paths: 1000, seed: 0x0000000000eeffc0 }`

| Rank | Strategy | Sharpe | Return% | MaxDD% | Trades | RobustnessFlag |
|------|----------|-------:|--------:|-------:|-------:|---------------|
| 1 | v0.buyhold | 1.297 | +49.78 | 29.63 | 0 | **Fragile** (CROWN) |
| 2 | v0.roc_momentum | 1.685 | +0.61 | 0.10 | 4 | Fragile |
| 3 | v0.5.macd | 1.611 | +2.41 | 1.38 | 185 | Fragile |
| 4 | v0.sma | 1.169 | +2.77 | 2.39 | 99 | Fragile |
| 5 | v0.donchian_floor | 1.114 | +4.70 | 4.80 | 1 | Fragile |
| 6 | **v0.dvol_regime** | **0.397** | **+0.76** | **2.33** | **17** | **Fragile** |
| ... | ... | ... | ... | ... | ... | ... |

**v0.dvol_regime ETHUSDT Bootstrap verdict: FRAGILE** (ineligible to crown)  
**Crowned:** v0.buyhold  
**Outcome:** BenchmarkWins  

### Summary of bootstrap verdicts

| Symbol | v0.dvol_regime Sharpe | v0.dvol_regime RobustnessFlag | Crowned | Outcome |
|--------|----------------------:|-------------------------------|---------|---------|
| BTCUSDT H1_2024 | -0.190 | **Fragile** | v0.buyhold | BenchmarkWins |
| ETHUSDT H1_2024 | +0.397 | **Fragile** | v0.buyhold | BenchmarkWins |

**The pre-registered expected null is confirmed.** Both symbols produce FRAGILE + BenchmarkWins, exactly as predicted by the feature spec (`feature.md` §5). The DVOL W=30 daily median cut does not produce bootstrap-robust excess returns vs buy-and-hold on BTCUSDT H1_2024 or ETHUSDT H1_2024 — this is the correct null result that validates the probe design.

### write_report=false — anchor safety

All bakeoff runs above used `write_report=false` (the default). No new anchored report files were created. The 119 existing anchors are unaffected.

## 6. Benchmarks

_n/a_ — No benchmark suite changes in this feature. The new `DvolRegimeStrategy.on_bar()` path is O(1) ring-buffer lookup + median over W=30 items; no latency-sensitive hot path regression expected.

## 7. Environment / Infrastructure Issues

1. **macOS dyld startup hang** — large test binaries (67-68 MB, `backtest` with `--features realdata`) experienced multi-minute hangs in `_dyld_start` before the test binary could execute. Affected initial bootstrap runs. Resolved by restarting the cargo invocations (subsequent runs loaded from dyld cache). This is a known macOS arm64 issue with cold-start large Rust binaries; not a code defect.

2. **`cargo fmt --check` pre-submit failure** — 10 diff sites in developer-submitted code. Tester applied `cargo fmt --all` to unblock gate validation. Fix must be submitted by developer.

3. **`cargo audit` not available** — `cargo audit` binary not installed in tester environment. Non-blocking; no new dependencies added by this feature that would introduce new audit concerns.

## 8. Pre-existing spec debt

spec-lint: PASS (0 violations). No pre-existing spec debt.

## 9. Verdict

**`FAIL`**

The feature code is functionally correct and the bootstrap verdict is confirmed (v0.dvol_regime = FRAGILE on both BTCUSDT and ETHUSDT as pre-registered). All mandatory non-negotiable gates pass. However the integration test `t7_1_full_wired_advisor_bakeoff_real_data` in `crates/backtest/tests/bakeoff_e2e.rs` fails with a stale hardcoded count assertion. The developer added `v0.dvol_regime` (entry #10) to `default_field()` but did not update the T7.1 test's `assert_eq!(report.candidates.len(), 13, ...)` to `19` (10 rule engines + 8 vote ensembles + 1 buyhold). The docstrings on lines 302, 305, 313, 375, and 440 also need to be updated. This is a one-line fix but it is a genuine test failure that cannot be silently waived.

## 10. Routing

`HANDOFF → developer` — Fix `crates/backtest/tests/bakeoff_e2e.rs:441`: change `13` → `19` and update adjacent comments (lines 302, 305, 313, 375, 440) to reflect "19-arm" / "18 arms before buyhold" (10 default_field + 8 default_ensemble_field). Then delete the tester-only `crates/backtest/tests/_tester_dvol_eth_bootstrap.rs` file. No other changes needed.
