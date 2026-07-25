---
title: Test Report — Tester Verification
feature: advisor-forward-plan
run_id: 2026-06-22-1100-UTC
commit: c16a37ca507e8c8d5a37bf7598cdec819b4a3c25
agent: tester
verdict: PASS
---

# Test Report — advisor-forward-plan — 2026-06-22 11:00 UTC

## 1. Scope

- **Feature / change under test:** Advisor forward buy/sell plan (F6). Anti-drift gate ensuring `plan_describe` output matches `on_bar` decisions for the same bar; forward plan populated UI render; F8-compatible ensemble named-plan render.
- **Spec refs:** `spec/advisor-forward-plan/feature.md`
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

### Anti-drift gate (`cargo test -p strategy --test plan_describe_matches_on_bar`)

| Test | Result |
|------|--------|
| `sma_long_plan_matches_on_bar` | PASS |
| `sma_flat_plan_matches_on_bar` | PASS |
| `always_long_plan_matches_on_bar` | PASS |
| `sma_describe_plan_does_not_mutate_state` | PASS |
| `composed_strategy_fresh_plan_is_flat` | PASS |
| + 1 additional anti-drift test | PASS |

**6 passed; 0 failed; 0 ignored.**

### Forward plan populated render (`cargo test -p ui --test forward_plan_populated_render`)

| Test | Result |
|------|--------|
| `forward_plan_rsi_reversion_paints_faithful_rules` | PASS |
| `forward_plan_empty_paints_no_plan` | PASS |
| `forward_plan_buy_and_hold_is_the_negative_control` | PASS |
| `forward_plan_f8_buy_and_hold_is_the_negative_control` | PASS |
| `forward_plan_f8_ensemble_paints_vote_rule_and_tally` | PASS |
| + 1 additional render test | PASS |

**6 passed; 0 failed; 0 ignored.** Duration: 4.20 s.

### F6 ensemble named render (`cargo test -p ui --test forward_f6_ensemble_named_render`)

| Test | Result |
|------|--------|
| `forward_f6_ensemble_names_its_members` | PASS |
| `named_ensemble_rules_band_exceeds_single_strategy` | PASS |

**2 passed; 0 failed; 0 ignored.** Duration: 4.24 s.

### Failing Tests

_none_

## 4. Property / Fuzz Tests

_n/a_ — no proptest/fuzz suites for this feature.

## 5. Backtest Results

_n/a_ — no new anchored backtest scenario for this feature; plan generation is a deterministic derivation from existing strategy state.

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

6 anti-drift tests, 6 forward-plan UI render tests, and 2 F6 ensemble named-render tests all pass with 0 failures. The anti-drift gate confirms `plan_describe` output is consistent with `on_bar` decisions (no silent state divergence). Render tests verify at the pixel layer that the plan paints correctly and the negative control (buy-and-hold) correctly produces a minimal plan. Static analysis clean workspace-wide.

## 9. Routing

`VERDICT → PASS` — ready; feature.md status bumped to `shipped`.
