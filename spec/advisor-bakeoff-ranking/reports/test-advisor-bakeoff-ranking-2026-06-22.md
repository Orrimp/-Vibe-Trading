---
title: Test Report — Tester Verification
feature: advisor-bakeoff-ranking
run_id: 2026-06-22-1100-UTC
commit: c16a37ca507e8c8d5a37bf7598cdec819b4a3c25
agent: tester
verdict: PASS
---

# Test Report — advisor-bakeoff-ranking — 2026-06-22 11:00 UTC

## 1. Scope

- **Feature / change under test:** Advisor bake-off + ranking engine (F1+F2+F3, ADR-0059). Independent tester verification to close the formal loop (spec-auditor flag 2026-06-22).
- **Spec refs:** `spec/advisor-bakeoff-ranking/feature.md`
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

### Bakeoff lib unit tests (`cargo test -p backtest bakeoff`)

| Crate / module | Passed | Failed | Ignored |
|----------------|-------:|-------:|--------:|
| `backtest::bakeoff` (bootstrap, rank, robustness, buyhold, mod) | 31 | 0 | 0 |
| Other lib tests (pre-existing, filtered out) | — | — | — |
| **Total (bakeoff filter)** | **31** | **0** | **0** |

### Bakeoff integration tests (`cargo test -p backtest --test bakeoff_e2e`)

| Test | Result |
|------|--------|
| `bakeoff_arm_parity::t2_2_buyhold_arm_parity` | PASS |
| `bakeoff_progress::t_prog_disabled_produces_no_events` | PASS |
| `bakeoff_progress::t_prog_1_bakeoff_progress_sequence` | PASS |

**3 passed; 0 failed; 0 ignored.**

### Leaderboard render tests (`cargo test -p ui --test leaderboard_populated_render`)

| Test | Result |
|------|--------|
| `leaderboard_populated_strictly_exceeds_empty` | PASS |
| `leaderboard_error_no_data_renders` | PASS |
| `leaderboard_benchmark_wins_headline_renders` | PASS |
| `leaderboard_f8_ensembles_and_fragile_badge_paint` | PASS |
| `leaderboard_f8_strictly_exceeds_five_arm_field` | PASS |
| + 6 additional leaderboard render tests | PASS |

**11 passed; 0 failed; 0 ignored.** Duration: 6.16 s.

### Bakeoff progress render tests (`cargo test -p ui --test bakeoff_progress_render`)

**3 passed; 0 failed; 0 ignored.** Duration: 6.99 s.

### Failing Tests

_none_

## 4. Property / Fuzz Tests

_n/a_ — no proptest/fuzz suites for this feature.

## 5. Backtest Results

_n/a_ — no new anchored backtest scenario. The bakeoff engine uses `write_report = false` on the `"v0.buyhold"` arm; no new report body is produced.

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

All 31 bakeoff lib unit tests, 3 bakeoff_e2e integration tests, 11 leaderboard render tests, and 3 bakeoff progress render tests pass with 0 failures and 0 new ignored tests. Static analysis clean workspace-wide. Anchor gate 119/119 confirmed. The developer T6.1–T6.9 items are satisfied: comparator cases verified (rank tests), anchor gate PASS, layering invariant unmodified.

## 9. Routing

`VERDICT → PASS` — ready; feature.md status bumped to `shipped`.
