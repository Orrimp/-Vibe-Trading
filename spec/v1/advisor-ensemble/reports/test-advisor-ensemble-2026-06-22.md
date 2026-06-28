---
title: Test Report — Tester Verification
feature: advisor-ensemble
run_id: 2026-06-22-1100-UTC
commit: c16a37ca507e8c8d5a37bf7598cdec819b4a3c25
agent: tester
verdict: PASS
---

# Test Report — advisor-ensemble — 2026-06-22 11:00 UTC

## 1. Scope

- **Feature / change under test:** F8 strategy-mix ensemble candidates for the single-coin advisor. Two day-1 mandatory e2e gates: ensemble vote divergence (ensemble equity differs from each member strategy) and robustness bootstrap gate (bootstrap actually bites on declining equity). Leaderboard ensemble + Fragile badge render verified.
- **Spec refs:** `spec/advisor-ensemble/feature.md`
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

### Day-1 gate 1 — ensemble vote divergence (`cargo test -p strategy --test ensemble_vote_divergence_end_to_end`)

| Test | Result |
|------|--------|
| `build_ensemble_unanimous_succeeds` | PASS |
| `build_ensemble_majority_succeeds` | PASS |
| `majority_ensemble_diverges_from_each_sma_member` | PASS |
| `unanimous_ensemble_diverges_from_majority` | PASS |
| `ensemble_equity_deterministic` | PASS |
| + 7 additional ensemble tests | PASS |

**12 passed; 0 failed; 0 ignored.**

This is the CLAUDE.md non-negotiable gate: `majority_ensemble_diverges_from_each_sma_member` asserts the ensemble equity diverges from each constituent strategy by ≥ 1 bp, proving the ensemble vote is not a no-op pass-through.

### Day-1 gate 2 — robustness bootstrap bites (`cargo test -p backtest --test robustness_bootstrap_bites`)

| Test | Result |
|------|--------|
| `bootstrap_fragile_for_declining_equity` | PASS |
| `bootstrap_not_fragile_for_growing_equity` | PASS |
| `bootstrap_compute_deterministic_growing_500` | PASS |
| `bootstrap_flags_populate_in_bakeoff` | PASS |
| `bakeoff_with_ensemble_field_runs_and_flags_them` | PASS |
| + 10 additional bootstrap robustness tests | PASS |

**15 passed; 0 failed; 0 ignored.** Duration: 0.52 s.

### Leaderboard ensemble + Fragile badge render (`cargo test -p ui --test leaderboard_populated_render`)

| Test | Result |
|------|--------|
| `leaderboard_f8_ensembles_and_fragile_badge_paint` | PASS |
| `leaderboard_f8_strictly_exceeds_five_arm_field` | PASS |
| + 9 other leaderboard render tests | PASS |

**11 passed; 0 failed; 0 ignored.** Duration: 6.16 s.

### Failing Tests

_none_

## 4. Property / Fuzz Tests

_n/a_ — no proptest/fuzz suites for this feature.

## 5. Backtest Results

_n/a_ — no new anchored backtest scenario. The ensemble candidates participate in the bakeoff but do not produce new anchored report bodies (`write_report = false` on the bakeoff path).

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

Both day-1 mandatory e2e gates pass: (1) the ensemble vote divergence gate proves the voting aggregation is not a no-op (12 tests); (2) the robustness bootstrap gate proves the `RobustnessMode::Bootstrap` path bites on declining equity and that ensemble candidates receive robustness flags in the bakeoff output (15 tests). Leaderboard render tests confirm the ensemble rows and Fragile badge paint at the pixel layer (11 tests, 6.16 s). Static analysis clean workspace-wide.

## 9. Routing

`VERDICT → PASS` — ready; feature.md status bumped to `shipped`.
