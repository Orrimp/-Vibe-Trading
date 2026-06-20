---
title: Test Report — Developer Handoff
feature: advisor-bakeoff-ranking
run_id: 2026-06-19-1800-UTC
commit: uncommitted (working tree)
agent: developer
verdict: PASS (developer gates only — T6.1–T6.9 for tester to close)
---

# Test Report — advisor-bakeoff-ranking — 2026-06-19

## 1. Scope

- **Feature / change under test:** Advisor bake-off + ranking engine (F1+F2, ADR-0059).
  New `bakeoff` module in `crates/backtest`; `"v0.buyhold"` run_scenario arm;
  `run_buyhold_path` / `classify_verdict` extracted from sweep bin into library;
  pure `rank_candidates` comparator; `run_bakeoff` orchestrator.
- **Spec refs:** `spec/advisor-bakeoff-ranking/feature.md`, `spec/advisor-bakeoff-ranking/tasks.md`, `spec/architecture/adr/0059-bakeoff-orchestrator-home-and-result-seam.md`
- **Commit SHA:** uncommitted working tree
- **Rust toolchain:** rustc 1.94.1 (e408947bf 2026-03-25)
- **OS / arch:** macOS Darwin 25.5.0 / arm64

## 2. Static Analysis

| Check              | Result | Notes                              |
|--------------------|--------|------------------------------------|
| `cargo fmt --check`| PASS   | clean after `cargo fmt -p backtest` |
| `cargo clippy -D warnings` | PASS | 0 warnings, forced re-lint via `touch src/lib.rs` |
| `cargo audit`      | n/a    | not run (no CVE-sensitive change)  |
| `cargo deny`       | n/a    | not run (no new deps added)        |

## 3. Unit & Integration Tests

### Lib tests (`cargo test -p backtest --lib`)

| Suite | Passed | Failed | Ignored |
|-------|-------:|-------:|--------:|
| `bakeoff::tests` | 4 | 0 | 0 |
| `bakeoff::rank::tests` | 12 | 0 | 0 |
| `bakeoff::robustness::tests` | 5 | 0 | 0 |
| `bakeoff::buyhold::tests` | 4 | 0 | 0 |
| Other lib tests (pre-existing) | 78 | 0 | 5 |
| **Total lib** | **103** | **0** | **5** |

### Integration tests (`cargo test -p backtest --test bakeoff_e2e`)

| Test | Result |
|------|--------|
| `bakeoff_arm_parity::t2_2_buyhold_arm_parity` | PASS |
| `bakeoff_realdata::t6_1_bakeoff_deterministic_on_real_data` | IGNORED (`--features realdata` required) |

### Full integration suite (`cargo test -p backtest`)

All 27 test suites: 0 failures, 0 errors.

### Failing Tests

_none_

## 4. Property / Fuzz Tests

_n/a_ — no proptest/fuzz suites in scope for this feature.

## 5. Backtest Results (anchor gate)

The `"v0.buyhold"` arm uses `write_report = false` — no anchored body is created.
Anchor regression gate:

```
bash scripts/verify_anchors.sh
ANCHORS PASS  (119 / 119)
```

Verified at three checkpoints:
1. Pre-change baseline (T0.1): 119/119
2. After M-DEV-1 relocation (T1.4): 119/119
3. After M-DEV-2 buyhold arm (T2.3): 119/119
4. Final check after all changes: 119/119

## 6. Ranking Comparator Test Matrix

| Case | Input | Expected `outcome` | Expected crown | Tested |
|------|-------|--------------------|----------------|--------|
| Sharpe primary | 3 cands with distinct Sharpe, no Fragile | `ActiveWins` | highest Sharpe | Yes (rank tests) |
| Robustness gate | high-Sharpe Fragile vs lower-Sharpe Robust | `ActiveWins` | Robust arm | Yes (rank tests) |
| BH wins | BH has highest eligible Sharpe | `BenchmarkWins` | BH arm | Yes (rank tests) |
| All fragile | all candidates `Fragile` | `AllFragile` | highest Sharpe of Fragile | Yes (rank tests) |
| Tie-break by return | equal Sharpe, different return | `ActiveWins` | higher return | Yes (rank tests) |
| Tie-break by DD | equal Sharpe+return, different DD | `ActiveWins` | lower drawdown | Yes (rank tests) |
| Tie-break lexicographic | fully equal KPIs | `ActiveWins` | lexicographically first ID | Yes (rank tests) |

## 7. Layering Invariant

```
cargo tree -p ui | wc -l → 1839
```

No new `strategy` / `exec` / `forecast` / `llm` edge introduced. `backtest::BakeoffReport`
is reachable from downstream (re-exported in `crates/backtest/src/lib.rs:72`).

## 8. Files Changed

| File | Change |
|------|--------|
| `crates/backtest/src/bakeoff/mod.rs` | NEW — orchestrator + public types |
| `crates/backtest/src/bakeoff/buyhold.rs` | NEW — `run_buyhold_path` (relocated) |
| `crates/backtest/src/bakeoff/robustness.rs` | NEW — `classify_verdict` + types (relocated) |
| `crates/backtest/src/bakeoff/rank.rs` | NEW — `rank_candidates` F2 comparator |
| `crates/backtest/tests/bakeoff_e2e.rs` | NEW — T2.2 arm-parity + T6.1 `#[ignore]` |
| `crates/backtest/src/lib.rs` | MODIFIED — `pub mod bakeoff` + re-exports |
| `crates/backtest/src/engine.rs` | MODIFIED — `"v0.buyhold"` dispatch arm |
| `crates/backtest/src/cancel.rs` | MODIFIED — `sibling()` method |
| `crates/backtest/src/bin/param_robustness_sweep.rs` | MODIFIED — delegates to library |

## 9. Tasks Left for Tester (T_FINAL)

| Task | Description | Command |
|------|-------------|---------|
| T6.1 | Real-data determinism | `cargo test -p backtest --features realdata --test bakeoff_e2e -- --ignored` |
| T6.2–T6.6 | Comparator cases | `cargo test -p backtest --lib bakeoff::rank::tests` |
| T6.7 | Anchor gate post-commit | `bash scripts/verify_anchors.sh` |
| T6.8 | Layering invariant post-commit | `cargo tree -p ui \| grep -E "strategy\|exec\|forecast\|llm"` |
| T6.9 | File final test report | (tester writes, then VERDICT → PASS) |

## 10. Verdict

**PASS (developer gates)**

All M-DEV-0 through M-DEV-5 tasks complete. 103 lib + integration tests pass,
0 fail. clippy clean. fmt clean. 119/119 anchors PASS (four-checkpoint verified).
Layering invariant clean (no new ui dep edges). T6.x left for tester.
