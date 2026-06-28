---
title: Test Report
feature: leaderboard-timeframe-capital
run_id: 2026-06-26-0900-UTC
commit: 2080b217a985bd63298b0d7b627c0e0850ca4b41
agent: tester
verdict: PASS
---

# Test Report — leaderboard-timeframe-capital — 2026-06-26 09:00 UTC

## 1. Scope

- **Feature / change under test:** Bake-off timeframe (H1/H4/D1) and start-capital tuning knobs. Adds timeframe-resampling to the bake-off engine (`backtest::bakeoff_e2e`) and a capital scaling parameter; wires both into the leaderboard UI state (cockpit slice #4). Tests assert that different timeframe or capital values produce divergent equity curves from the H1/€100k default.
- **Spec refs:** `spec/leaderboard-timeframe-capital/feature.md`
- **Commit SHA:** `2080b217a985bd63298b0d7b627c0e0850ca4b41`
- **Rust toolchain:** `rustc 1.94.1 (e408947bf 2026-03-25)`
- **OS / arch:** `Darwin arm64`

## 2. Static Analysis

| Check               | Result | Notes                                                    |
|---------------------|--------|----------------------------------------------------------|
| `cargo fmt --check` | PASS   | Exit 0 — workspace clean                                 |
| `cargo clippy`      | PASS   | `cargo clippy --workspace --all-targets --features ui/live -- -D warnings` exit 0; zero warnings emitted |
| `cargo audit`       | n/a    | Not run this cycle (no dependency changes in scope)      |
| `cargo deny`        | n/a    | Not run this cycle                                       |

spec-lint: PASS (0 violations) — `python3 scripts/spec_lint.py` exit 0.

## 3. Unit & Integration Tests

| Suite | Test binary / target | Passed | Failed | Ignored | Duration |
|-------|---------------------|-------:|-------:|--------:|---------:|
| `backtest` — `bakeoff_e2e` (filtered: `leaderboard_tuning_divergence`) | `tests/bakeoff_e2e.rs` | 3 | 0 | 0 | 0.02s |
| `ui` — `bakeoff_config_from_state` (fixtures) | `tests/bakeoff_config_from_state.rs` | 6 | 0 | 0 | 0.00s |
| **Total** | | **9** | **0** | **0** | |

### Test detail — bakeoff_e2e :: leaderboard_tuning_divergence (3/3)

```
leaderboard_tuning_divergence::t_timeframe_div_resampler_reduces_bar_count_4to1       ... ok
leaderboard_tuning_divergence::t_timeframe_bars_resampled_bars_produce_different_equity ... ok
leaderboard_tuning_divergence::t_capital_div_2x_capital_doubles_absolute_equity        ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 3 filtered out; finished in 0.02s
```

Three divergence scenarios verified:
- `t_timeframe_div_resampler_reduces_bar_count_4to1`: H4 resampling produces 4x fewer bars than H1 — confirms the resampler is active.
- `t_timeframe_bars_resampled_bars_produce_different_equity`: H4 equity curve differs from H1 equity curve on the same raw data — functional divergence gate.
- `t_capital_div_2x_capital_doubles_absolute_equity`: doubling start capital doubles absolute equity (linear scaling confirmed).

### Test detail — bakeoff_config_from_state (6/6)

```
bakeoff_config_state_wiring::t_state_d1_timeframe_knob_wires_to_horizon         ... ok
bakeoff_config_state_wiring::t_state_capital_invalid_falls_back_to_100k         ... ok
bakeoff_config_state_wiring::t_state_h1_default_is_identity_and_100k            ... ok
bakeoff_config_state_wiring::t_state_capital_knob_wires_parsed_value            ... ok
bakeoff_config_state_wiring::t_state_both_knobs_independently_addressable       ... ok
bakeoff_config_state_wiring::t_state_h4_timeframe_knob_wires_to_horizon         ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; finished in 0.00s
```

Six UI state wiring scenarios verified:
- H1 default is identity (no resampling) with €100k capital.
- H4 and D1 timeframe knobs wire to the correct horizon enum variant.
- Capital knob parses the text-field value and wires it to the bake-off request.
- Invalid capital text falls back to the €100k default.
- Both knobs are independently addressable (no cross-contamination).

### Failing Tests

_none_

## 4. Property / Fuzz Tests

_n/a_ — no proptest/fuzz suites for this feature.

## 5. Backtest Results

_n/a_ — this feature adds tuning knobs to the bake-off configuration; it does not add new strategy logic. The divergence tests in §3 serve as the functional correctness gate (different timeframe → different equity; different capital → scaled equity). No separate backtest report required.

## 6. Benchmarks

_n/a_ — no hot-path changes. Resampling is O(n) over the bar sequence and occurs once per bake-off run, not in the tick loop.

## 7. Environment / Infrastructure Issues

No render tests for this feature — the `bakeoff_config_from_state` test suite uses the `fixtures` feature for state wiring but does not invoke the iced render pipeline. No CoreText deadlock risk. Multiple cargo invocations for `crates/backtest` serialized behind the artifact directory lock without failures.

## 8. Verdict

**PASS**

All 9 tests across 2 suites pass. Static analysis (clippy, fmt) is clean. Anchor gate holds at 119/119. spec-lint reports 0 violations. The three divergence scenarios confirm that the H4 resampler reduces bar counts by 4x, produces different equity from H1, and that the capital parameter scales linearly. The six UI wiring tests confirm that all three timeframe knob positions and the capital field correctly populate the bake-off request. No regressions detected.

## 9. Routing

`VERDICT → PASS` — ready to ship.

---

## Shared Gates (cited in all three reports)

| Gate | Result |
|------|--------|
| `cargo fmt --check` (workspace) | PASS — exit 0 |
| `cargo clippy --workspace --all-targets --features ui/live -- -D warnings` | PASS — exit 0 |
| `bash scripts/verify_anchors.sh` | PASS — 119/119 |
| `python3 scripts/spec_lint.py` | PASS — 0 violations |
