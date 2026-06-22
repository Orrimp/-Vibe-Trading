---
title: Test Report — Tester Verification
feature: advisor-forward-paper
run_id: 2026-06-22-1100-UTC
commit: c16a37ca507e8c8d5a37bf7598cdec819b4a3c25
agent: tester
verdict: PASS
---

# Test Report — advisor-forward-paper — 2026-06-22 11:00 UTC

## 1. Scope

- **Feature / change under test:** Budget-aware sizing + forward paper-trade (F4+F5+F5b). Day-1 sizing-divergence gate (budget cap changes return path vs uncapped baseline) and F5b anti-fake engine-identity gate (each strategy produces its own distinct fill path).
- **Spec refs:** `spec/advisor-forward-paper/feature.md`
- **Commit SHA:** `c16a37ca507e8c8d5a37bf7598cdec819b4a3c25`
- **Rust toolchain:** rustc 1.94.1 (e408947bf 2026-03-25)
- **OS / arch:** Darwin arm64

## 2. Static Analysis

| Check              | Result | Notes                                           |
|--------------------|--------|-------------------------------------------------|
| `cargo fmt --check`| PASS   | clean, exit 0                                   |
| `cargo clippy`     | PASS   | 0 warnings workspace-wide; forced re-lint via `touch crates/*/src/lib.rs` |
| `cargo audit`      | n/a    | no CVE-sensitive change in this feature         |
| `cargo deny`       | n/a    | no new deps added                               |

## 3. Unit & Integration Tests

### Day-1 sizing-divergence gate (`cargo test -p risk --test budget_sizing_divergence_end_to_end`)

| Test | Result |
|------|--------|
| `budget_cap_changes_return_path_vs_uncapped_baseline` | PASS |

**1 passed; 0 failed; 0 ignored.** Duration: 0.00 s (build: 19.06 s).

This test is the CLAUDE.md non-negotiable: asserts the budget-cap sizing modifier diverges from the uncapped baseline equity by a measurable epsilon, preventing a no-op overlay from shipping silently.

### F5b engine-identity gate (`cargo test -p agent --test forward_run_engine_fidelity`)

| Test | Result |
|------|--------|
| `f5b_buyhold_registry_differs_from_sma_registry_on_same_bars` | PASS |
| `f5b_macd_identity_is_btc_macd_trend_not_sma_crossover` | PASS |
| `f5b_rsi_identity_is_btc_rsi_reversion_not_sma_crossover` | PASS |
| `f5b_bbands_identity_is_btc_bbands_mean_revert_not_sma_crossover` | PASS |
| `f5b_macd_registry_differs_from_sma_registry_on_same_bars` | PASS |
| + 3 additional engine-identity tests | PASS |

**8 passed; 0 failed; 0 ignored.**

### Failing Tests

_none_

## 4. Property / Fuzz Tests

_n/a_ — no proptest/fuzz suites for this feature.

## 5. Backtest Results

_n/a_ — no new anchored backtest scenario for this feature; the sizing gate is a synthetic unit-level end-to-end test.

Anchor regression gate re-verified this session:

```
bash scripts/verify_anchors.sh
ANCHORS PASS  (119 / 119)
```

## 6. Benchmarks

_n/a_ — no latency-sensitive hot path changed.

## 7. Environment / Infrastructure Issues

_none_

## 8. Verdict

**PASS**

Both day-1 mandatory gates pass: the budget-cap sizing-divergence gate (`budget_cap_changes_return_path_vs_uncapped_baseline`) confirms the sizing modifier is not a no-op, and 8 engine-identity tests confirm each strategy registers under a distinct identity and produces a distinct fill sequence. Static analysis clean workspace-wide.

## 9. Routing

`VERDICT → PASS` — ready; feature.md status bumped to `shipped`.
